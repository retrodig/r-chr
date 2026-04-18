# アーキテクチャ

## 全体構成

```
┌──────────────────────────────────────────────────────────┐
│  eframe（ネイティブウィンドウ・OpenGL / Metal）           │
│  ┌────────────────────────────────────────────────────┐  │
│  │  egui（即時モード GUI レンダリング）               │  │
│  │  ┌──────────────────────────────────────────────┐  │  │
│  │  │  editor::app::RChrApp（アプリ状態）          │  │  │
│  │  │    update() ─ 毎フレーム呼ばれる UI 構築     │  │  │
│  │  └──────────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
         │                    │
    io モジュール          model モジュール
    ├── chr.rs              └── palette.rs
    ├── nes.rs
    └── png.rs

[macOS のみ]
native_menu.rs ──── muda（NSMenu）
                └── objc2-app-kit（NSAppearance）
```

---

## メインループ（`eframe::App::update()`）

```
update() 呼び出し（1フレーム）
│
├── Visuals 設定（dark / light）
│
├── [macOS] handle_native_menu()           ← mac/menu_events.rs
│     native_menu::try_recv_action() でイベント受信・ディスパッチ
│     native_menu::sync_state() で enabled/checked 同期
│
├── テクスチャ再生成（texture_dirty フラグ）
│     render_full_image() → ctx.load_texture()
│
├── ウィンドウ閉じるリクエスト処理
│     未保存変更ダイアログ → 保存 / 破棄 / キャンセル
│
├── タイトルバー更新（未保存変更を * で表示）
│
├── [非 macOS] show_menu_bar()             ← mac/menu_bar.rs
│     egui によるメニューバー描画
│
├── 1px ボーダー（TopBottomPanel）
│
├── 右パネル: show_info_panel()            ← info_panel.rs
│
├── 中央パネル: show_dot_editor()          ← dot_editor.rs
│
├── バンクビュー: show_bank_view()         ← bank_view.rs
│     bank_texture（GPUキャッシュ）を ScrollArea に表示
│
├── EditorAction 適用（apply_action）
│     PaintDot / Eyedrop / SelectDrawingColor
│
├── PNG インポートダイアログ               ← png_import.rs
│
├── ドラッグ & ドロップ検知
│     .nes / .bin / .zip → open_file_from_path()
│     .png / .bmp        → open_png_import_from_path()
│
├── About ダイアログ（egui::Window）
│
└── handle_keyboard()                      ← keyboard.rs
      パレット切替・アンドゥ・保存・矢印キー移動
```

---

## モジュール間の依存関係

```
editor::app
    ├── io::chr          (タイルデコード・エンコード・レンダリング)
    ├── io::nes          (ROM パース・RomData)
    ├── io::png          (PNG インポート)
    └── model::palette   (MasterPalette・DatPalette)

editor::mac::menu_events  (macOS のみ)
    └── native_menu      (try_recv_action / sync_state)

editor::mac::menu_bar     (非 macOS のみ)
    └── editor::app      (RChrApp メソッド呼び出し)

io::chr
    └── model::palette   (color_rgb)

io::png
    ├── model::palette
    └── io::chr          (encode_dot)

native_menu              (macOS のみ、editor::app から呼ばれる)
```

**依存の方向**: `editor` → `io` → `model`（モデル層は外部に依存しない）

---

## editor/ モジュール構成の設計方針

`app.rs` を薄いオーケストレーター（`update()` の骨格）として保ち、
機能ごとに `impl RChrApp` ブロックを別ファイルに分離する。

| ファイル | 責務 |
|--------|------|
| `app.rs` | 構造体定義・`update()` 骨格・フォント登録 |
| `file_ops.rs` | `new_file` / `open_file` / `save_file` / PAL / DAT |
| `keyboard.rs` | キーボードショートカット処理 |
| `mac/menu_events.rs` | macOS ネイティブメニューイベント（`#[cfg(target_os = "macos")]`） |
| `mac/menu_bar.rs` | egui メニューバー（`#[cfg(not(target_os = "macos"))]`） |
| `bank_view.rs` | CHR バンク全体表示 |
| `dot_editor.rs` | 8×8 ピクセルグリッドエディタ |
| `info_panel.rs` | 右パネル |
| `png_import.rs` | PNG インポートダイアログ |
| `theme.rs` | UI カラー定数 |

---

## テクスチャキャッシュ戦略

- `bank_texture: Option<egui::TextureHandle>` に GPU テクスチャをキャッシュ
- `texture_dirty: bool` フラグが立ったフレームのみ `render_full_image()` → `ctx.load_texture()` で再生成
- タイルを編集した直後・バンク切り替え・パレット変更時に `texture_dirty = true`

---

## macOS ネイティブメニューの設計

### 初期化順序の制約

`muda::init_for_nsapp()` は `NSApplication` が存在した後でなければならない。
`NSApplication` は `eframe::run_native()` 内部で初期化されるため、
`native_menu::init()` は eframe の **creation callback の中**（`Box::new(|cc| { ... })`）で呼ぶ。

### 非 Send な MenuItem の保持

`muda::MenuItem` は内部で `Rc` を使用しており `Send` でない。
`OnceLock<MenuHandles>` は使えないため `thread_local!` + `RefCell` で保持する。

```rust
thread_local! {
    static HANDLES: RefCell<Option<MenuHandles>> = RefCell::new(None);
}
```

### macOS のアプリメニュー規則

macOS は **メニューバーの先頭サブメニューを必ずアプリ名メニューとして使用**する。
そのため、最初のサブメニューとして `Submenu::new("R-CHR", true)` を明示的に追加し、
ファイル・編集・表示・パレットはその後に続ける。

### メニューイベントフロー

```
NSMenu ──(クリック)──→ muda::MenuEvent::receiver()
                              │
                    native_menu::try_recv_action()
                              │
                         MenuAction enum
                              │
                    editor::mac::menu_events::handle_native_menu() でディスパッチ
```

---

## ドラッグ & ドロップ

```rust
// update() 内
let dropped = ctx.input(|i| {
    i.raw.dropped_files.iter().find_map(|f| {
        let path = f.path.as_ref()?;
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Some((path.clone(), ext))
    })
});
if let Some((path, ext)) = dropped {
    match ext.as_str() {
        "nes" | "bin" | "zip" => self.open_file_from_path(&path),
        "png" | "bmp"         => self.open_png_import_from_path(&path),
        _                     => {}
    }
}
```

---

## アンドゥ機構

- `undo_stack: Vec<(usize, [u8; 16])>`：(タイルバイトオフセット, 変更前 16 バイト)
- ドット描画の**最初のマウスダウン時のみ** `push_undo=true` でスタックに積む
- ドラッグ中は `push_undo=false`（1 操作 = 1 アンドゥエントリ）
- Cmd+Z / Ctrl+Z で pop → CHR データを書き戻し → `texture_dirty = true`