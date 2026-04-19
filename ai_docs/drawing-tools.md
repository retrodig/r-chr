# R-CHR 描画ツール実装解説

NES CHR エディタ **R-CHR** に実装した10種類の描画ツールについて、UX・アルゴリズム・ソースコード構造をまとめる。

---

## 全体アーキテクチャ

描画ツールは `src/editor/dot_editor.rs` に集約されている。  
UI の描画と CHR データへの書き込みを分離するため、**EditorAction** という列挙型を介してデータフローが流れる。

```
show_dot_editor() → Option<EditorAction>
         ↓
apply_action(action: EditorAction)  ← CHR 書き込み・undo 処理
```

```rust
pub(super) enum EditorAction {
    PaintDot { tile_offset: usize, px: usize, py: usize, color: u8, push_undo: bool },
    Eyedrop  { color_idx: u8 },
    SelectDrawingColor { color_idx: u8 },
    ApplyLine  { pixels: Vec<(usize, usize, usize)> },        // (offset, px, py)
    ApplyStamp { pixels: Vec<(usize, usize, usize, u8)> },    // (offset, px, py, color)
}
```

この設計により、egui の即時モード描画ループの中でデータ変更が起きず、フレームごとに確実に一度だけ書き込みが行われる。

---

## ツール一覧と tool インデックス

| # | アイコン名 | ツール名 |
|---|-----------|---------|
| 0 | pencil | ペン |
| 1 | pencil_pattern | ペン（パターン） |
| 2 | slash | 線 |
| 3 | square | 矩形（枠線） |
| 4 | square_fill | 矩形（塗り） |
| 5 | square_pattern | 矩形（パターン） |
| 6 | circle | 楕円（枠線） |
| 7 | circle_fill | 楕円（塗り） |
| 8 | paint-bucket | 塗りつぶし |
| 9 | stamp | スタンプ |

`self.drawing_tool: usize` に現在の選択ツールインデックスが入り、ツールバーのボタンクリックで切り替わる。

---

## 座標系

ドットエディタのキャンバスは「フォーカスブロック」と呼ぶ N×N タイル（N = 1/2/4/8/16）の領域を表示する。

```
block_px = N * 8   // キャンバスのドット数（1辺）
```

CHR データは 1 タイル = 16 バイトの 2BPP 形式。グローバルタイルインデックスからバイトオフセットへの変換：

```
tile_offset = tile_global * 16
tile_global = (top_row + block_row) * 16 + (top_col + block_col)
```

ここで `top_left_tile = selected_tile`（バンクビューの選択タイル）。

---

## 0. ペン（tool=0）

### 挙動
クリック・ドラッグで 1 ドットずつ描画。  
ドラッグ開始時に undo バッチを新規作成し、ドラッグ中に触れた新タイルを同バッチへ追記する。

### undo 設計

```rust
if push_undo {
    // ドラッグ開始 or クリック: 新規バッチ
    self.drag_undo_tiles.clear();
    let saved = rom.chr_data()[tile_offset..tile_offset+16].try_into().unwrap();
    self.push_undo_batch(vec![(tile_offset, saved)]);
    self.drag_undo_tiles.insert(tile_offset);
} else if !self.drag_undo_tiles.contains(&tile_offset) {
    // ドラッグ中に初めて触れたタイル: 同バッチへ追記
    let saved = rom.chr_data()[tile_offset..tile_offset+16].try_into().unwrap();
    if let Some(batch) = self.undo_stack.last_mut() {
        batch.push((tile_offset, saved));
    }
    self.drag_undo_tiles.insert(tile_offset);
}
```

`drag_undo_tiles: HashSet<usize>` でタイル境界をまたいだドラッグを正しく 1 操作として扱う。

---

## 1. ペン（パターン）（tool=1）

### 挙動
クリック起点のパリティ `(x + y) % 2` を記録し、ドラッグ中は同パリティのドットのみ描画する。結果としてチェッカーボードパターンになる。

### アルゴリズム

```rust
let global_x = (tile_global % 16) * 8 + px;
let global_y = (tile_global / 16) * 8 + py;
let parity   = ((global_x + global_y) % 2) as u8;

if push_undo {
    self.drag_pattern_parity = parity;  // 起点のパリティを確定
}
// ドラッグ中: 起点と異なるパリティはスキップ
if self.drawing_tool == 1 && parity != self.drag_pattern_parity {
    return;
}
```

グローバル座標を使うことで、タイル境界をまたいでも連続したチェッカーボードになる。

---

## 2. 線（tool=2）— Bresenham の直線

### 挙動
ドラッグ起点〜現在位置をリアルタイムプレビューし、ボタンを離した瞬間に確定する。

### Bresenham アルゴリズム

```rust
fn bresenham(x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<(usize, usize)> {
    let (mut x, mut y) = (x0 as i32, y0 as i32);
    let (x1, y1) = (x1 as i32, y1 as i32);
    let dx = (x1 - x).abs();
    let dy = -(y1 - y).abs();
    let sx: i32 = if x < x1 { 1 } else { -1 };
    let sy: i32 = if y < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut pts = Vec::new();
    loop {
        pts.push((x as usize, y as usize));
        if x == x1 && y == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
    pts
}
```

整数演算のみで動作し、浮動小数点誤差がない。

### ドラッグ終了検出

egui の `interact_pointer_pos()` はボタンリリース時に `None` を返す場合があるため、リリース検出を `interact_pointer_pos()` の前に配置している。

```rust
let just_released = ui.ctx().input(|i| i.pointer.button_released(Primary));
if just_released {
    let end = ui.ctx().input(|i| i.pointer.hover_pos());
    // hover_pos から終点座標を取得して ApplyLine を返す
}
```

---

## 3〜5. 矩形ツール（tool=3/4/5）

### 挙動
線ツールと同じドラッグ→プレビュー→リリース確定のフロー。`shape_dots()` でツール種別を振り分ける。

### アルゴリズム

```rust
fn rect_dots(sx: usize, sy: usize, ex: usize, ey: usize, kind: usize) -> Vec<(usize, usize)> {
    let (x0, x1) = (sx.min(ex), sx.max(ex));
    let (y0, y1) = (sy.min(ey), sy.max(ey));
    let parity = (sx + sy) % 2;   // パターンツール用
    let mut pts = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let draw = match kind {
                3 => x == x0 || x == x1 || y == y0 || y == y1, // 枠線
                4 => true,                                        // 塗り
                5 => (x + y) % 2 == parity,                     // パターン
                _ => false,
            };
            if draw { pts.push((x, y)); }
        }
    }
    pts
}
```

パターン矩形は起点 `(sx, sy)` のパリティを基準にするため、どこから描き始めても一貫したチェッカーボードが得られる。

### ディスパッチ関数

```rust
fn shape_dots(sx: usize, sy: usize, ex: usize, ey: usize, tool: usize) -> Vec<(usize, usize)> {
    match tool {
        2     => bresenham(sx, sy, ex, ey),
        3|4|5 => rect_dots(sx, sy, ex, ey, tool),
        6     => ellipse_dots(sx, sy, ex, ey, false),
        7     => ellipse_dots(sx, sy, ex, ey, true),
        _     => vec![],
    }
}
```

プレビュー描画・リリース確定の両方が `shape_dots()` を通るため、ロジックの重複がない。

---

## 6〜7. 楕円ツール（tool=6/7）

### 挙動
バウンディングボックス（対角2点）で楕円を定義し、ドラッグでリアルタイムプレビュー。

### アルゴリズム

楕円の数式 `(x-cx)²/rx² + (y-cy)²/ry² ≤ 1` を直接使う浮動小数点アプローチを採用。

**塗り（fill=true）**：スキャンライン法。各行 y について x 方向の幅を計算してその範囲を塗りつぶす。

```rust
// 各行の楕円内スキャンライン
for y in y0..=y1 {
    let dy = y as f64 - cy;
    let t  = 1.0 - dy * dy / ry2;
    if t < 0.0 { continue; }
    let hw = (rx2 * t).sqrt();
    let xl = (cx - hw).ceil()  as i32;
    let xr = (cx + hw).floor() as i32;
    for x in xl.max(x0)..=xr.min(x1) {
        set.insert((x as usize, y as usize));
    }
}
```

**輪郭（fill=false）**：各行の左右端点 ＋ 各列の上下端点を求め `HashSet` で重複除去する。  
行だけだと急勾配部分で隙間が生じるため、列方向でも端点を補完する。

```rust
// 各行の端点
for y in y0..=y1 { let hw = ...; xl = round(cx - hw); xr = round(cx + hw); ... }
// 各列の端点（隙間埋め）
for x in x0..=x1 { let hh = ...; yt = round(cy - hh); yb = round(cy + hh); ... }
```

小さい楕円（数ピクセル）でも比較的きれいな輪郭が得られる。

---

## 8. 塗りつぶし（tool=8）— BFS フラッドフィル

### 挙動
クリックした位置の色インデックスが連続している範囲を、選択中の描画色で塗りつぶす。4方向連結。

### アルゴリズム

```rust
fn flood_fill_dots(
    block: &[Vec<u8>],
    sx: usize, sy: usize,
    block_px: usize,
    top_row: usize, top_col: usize,
    fill_color: u8, chr_len: usize,
) -> Vec<(usize, usize, usize)> {
    let target = block[sy][sx];
    if target == fill_color { return vec![]; }  // 同色なら何もしない

    let mut visited = vec![vec![false; block_px]; block_px];
    let mut queue   = std::collections::VecDeque::new();
    queue.push_back((sx, sy));
    visited[sy][sx] = true;
    let mut pixels  = Vec::new();

    while let Some((x, y)) = queue.pop_front() {
        pixels.push(/* tile_offset, dot_px, dot_py */);
        for (nx, ny) in [上, 下, 左, 右] {
            if 範囲内 && !visited[ny][nx] && block[ny][nx] == target {
                visited[ny][nx] = true;
                queue.push_back((nx, ny));
            }
        }
    }
    pixels
}
```

- 比較対象は RGB ではなくパレットの **色インデックス（0〜3）**
- デコード済みの `block: Vec<Vec<u8>>` を使うため CHR を再解析しない
- 収集したドット列を `ApplyLine` に渡し、undo 対応の一括書き込みを行う

---

## 9. スタンプ（tool=9）— 2フェーズ操作

### 挙動

**Phase1（選択）**  
ドラッグで矩形範囲を指定。点線でリアルタイムプレビュー。  
ボタンを離すと選択範囲の色インデックスをバッファに取り込む。

**Phase2（貼り付け）**  
ドラッグでスタンプ位置を決め、離した瞬間に CHR へ書き込む。  
バッファは保持されるため何度でもスタンプ可能。右クリックでキャンセル。

### 状態管理

```rust
stamp_sel_start:   Option<(usize, usize)>,              // Phase1 ドラッグ起点
stamp_buffer:      Option<(usize, usize, Vec<Vec<u8>>)>, // (w, h, pixels[h][w])
stamp_paste_pos:   (usize, usize),                       // 現在の貼り付け位置
stamp_drag_anchor: Option<(i32, i32)>,                   // ドラッグアンカー
```

Phase 判定：

| 状態 | 条件 |
|------|------|
| Idle | buffer=None, sel_start=None |
| Phase1 選択中 | sel_start=Some |
| Phase2 待機 | buffer=Some, sel_start=None, anchor=None |
| Phase2 ドラッグ中 | buffer=Some, sel_start=None, anchor=Some |

### バッファ取り込み

```rust
let pixels: Vec<Vec<u8>> = (y0..=y1)
    .map(|y| (x0..=x1).map(|x| block[y][x]).collect())
    .collect();
self.stamp_buffer = Some((w, h, pixels));
```

デコード済み `block` から色インデックスをそのまま切り出すため、パレットに依存しない。  
スタンプ先でパレットが変わっても正しく再解釈される。

### ドラッグ中の位置更新

```rust
if let Some((ax, ay)) = self.stamp_drag_anchor {
    self.stamp_paste_pos = (
        (cx as i32 - ax).max(0) as usize,
        (cy as i32 - ay).max(0) as usize,
    );
}
```

`stamp_drag_anchor` はドラッグ開始時の「カーソルとスタンプ左上の差」。これを使って相対移動させることで、スタンプがカーソルにスナップせずに自然に動く。

### 点線描画

```rust
fn draw_dashed_rect_outline(painter: &egui::Painter, r: egui::Rect, dash: f32) {
    for (p0, p1) in [top, right, bottom, left] {
        let len   = 辺の長さ;
        let steps = (len / dash).ceil() as usize;
        for i in 0..steps {
            let color = if i % 2 == 0 { WHITE } else { BLACK };
            painter.line_segment([t0..t1], Stroke::new(1.5, color));
        }
    }
}
```

白と黒のセグメントを交互に描くことで、背景色によらず視認できる点線になる。

---

## 共通 Undo 設計

全ツールのデータ書き込みは `push_undo_batch` を経由する。

```rust
pub(super) fn push_undo_batch(&mut self, batch: Vec<(usize, [u8; 16])>) {
    if batch.is_empty() { return; }
    const UNDO_LIMIT: usize = 100;
    if self.undo_stack.len() >= UNDO_LIMIT {
        self.undo_stack.remove(0);  // 古いものを破棄
    }
    self.undo_stack.push(batch);
}
```

- undo の単位は「タイル（16バイト）のスナップショットのまとまり（バッチ）」
- 1 ドラッグ操作 / 1 図形 / 1 スタンプ = 1 バッチ
- Cmd+Z で最新バッチをまるごと巻き戻す

ペンツールではドラッグ中に複数タイルをまたぐ場合があるため、`drag_undo_tiles: HashSet<usize>` で「このドラッグですでに保存済みのタイル」を追跡する。

```rust
// ドラッグ中に初めて触れたタイルのみ保存
if !self.drag_undo_tiles.contains(&tile_offset) {
    if let Some(batch) = self.undo_stack.last_mut() {
        batch.push((tile_offset, saved));
    }
    self.drag_undo_tiles.insert(tile_offset);
}
```

---

## プレビュー描画の仕組み

線・矩形・楕円・スタンプは「ドラッグ中は CHR を書き換えず、painter で仮描画する」設計。

```
毎フレーム:
  1. CHR から block をデコード（現在の確定データ）
  2. block をキャンバスに描画
  3. プレビュー（仮のドット）を painter で上書き描画
  4. ユーザーが離したら ApplyLine / ApplyStamp で確定書き込み
```

egui は即時モードなので、プレビューは「毎フレーム painter に描くだけ」で実装できる。CHR データに触れないため、undo スタックも汚れない。