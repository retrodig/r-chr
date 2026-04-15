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
├── [macOS] native_menu::try_recv_action()
│     MenuAction を受け取りディスパッチ
│     （ファイル開く・保存・アンドゥ・パレット操作など）
│
├── ドラッグ & ドロップ検知
│     .nes / .bin → open_file_from_path()
│     .png        → open_png_import_from_path()
│
├── キーボードショートカット処理
│     Cmd/Ctrl+S → save_file()
│     Cmd/Ctrl+Z → undo()
│     矢印キー   → タイル選択移動
│
├── CentralPanel
│   ├── [Windows/Linux] egui メニューバー
│   ├── バンクビュー（ScrollArea + bank_texture）
│   │     bank_texture は texture_dirty フラグで再生成
│   ├── ドットエディタ（8×8 ピクセルグリッド）
│   │     EditorAction を収集 → apply_editor_actions()
│   └── パレットピッカー
│
├── PNG インポートダイアログ（Window）
│
├── [macOS] native_menu::sync_state()
│     enabled / checked 状態を毎フレーム更新
│
└── ステータスバー（BottomPanel）
```

---

## モジュール間の依存関係

```
editor::app
    ├── io::chr          (タイルデコード・エンコード・レンダリング)
    ├── io::nes          (ROM パース・RomData)
    ├── io::png          (PNG インポート)
    └── model::palette   (MasterPalette・DatPalette)

io::chr
    └── model::palette   (color_rgb)

io::png
    ├── model::palette
    └── io::chr          (encode_dot)

native_menu              (macOS のみ、editor::app から呼ばれる)
```

**依存の方向**: `editor` → `io` → `model`（モデル層は外部に依存しない）

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
                    editor::app::update() でディスパッチ
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
        "nes" | "bin" => self.open_file_from_path(&path),
        "png"         => self.open_png_import_from_path(&path),
        _             => {}
    }
}
```

---

## アンドゥ機構

- `undo_stack: Vec<(usize, [u8; 16])>`：(タイルバイトオフセット, 変更前 16 バイト)
- ドット描画の**最初のマウスダウン時のみ** `push_undo=true` でスタックに積む
- ドラッグ中は `push_undo=false`（1 操作 = 1 アンドゥエントリ）
- Cmd+Z / Ctrl+Z で pop → CHR データを書き戻し → `texture_dirty = true`