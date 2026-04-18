# ディレクトリ構成

```
r-chr/
├── Cargo.toml
├── assets/
│   ├── icon.png          # ウィンドウアイコン（include_bytes! で埋め込み）
│   ├── rchr.bin          # 起動時デフォルト CHR データ（R-CHR ロゴ）
│   ├── rchr.pal          # デフォルトマスターパレット（192 バイト）
│   ├── rchr.dat          # デフォルト DAT パレット（16 バイト）
│   ├── nes.pal           # NES 標準 64 色パレット（リセット用）
│   └── fonts/
│       └── Noto_Sans_JP/ # 埋め込み日本語フォント
├── ai_docs/              # AI 向けドキュメント（本ディレクトリ）
│   ├── architecture.md
│   ├── data-struture.md
│   ├── directory-struture.md
│   ├── implementation-tasks.md
│   ├── requirement.md
│   └── tech-stack.md
└── src/
    ├── main.rs           # エントリポイント・ウィンドウ設定・アイコン読み込み
    ├── native_menu.rs    # macOS ネイティブメニューバー定義（#[cfg(target_os = "macos")]）
    ├── io/               # ファイル形式の入出力
    │   ├── mod.rs
    │   ├── chr.rs        # 2BPP CHR エンコード / デコード / レンダリング
    │   ├── nes.rs        # iNES パーサ（NesRom, NesHeader, RomData）
    │   └── png.rs        # PNG → CHR カラーインデックス変換
    ├── model/            # ドメインデータ構造
    │   ├── mod.rs
    │   └── palette.rs    # MasterPalette（64 色）・DatPalette（4×4）
    └── editor/           # UI レイヤ
        ├── mod.rs
        ├── app.rs        # RChrApp（eframe::App 実装・メインループ）
        ├── bank_view.rs  # CHR バンク全体表示（ScrollArea + テクスチャ）
        ├── dot_editor.rs # 8×8 ピクセルグリッドエディタ
        ├── file_ops.rs   # ファイル入出力（new / open / save / PAL / DAT）
        ├── info_panel.rs # 右パネル（描画色・パレット・タイル情報）
        ├── keyboard.rs   # キーボードショートカット処理
        ├── mac/          # プラットフォーム別メニュー処理
        │   ├── mod.rs
        │   ├── menu_bar.rs     # egui メニューバー（非 macOS 用）
        │   └── menu_events.rs  # macOS ネイティブメニューイベント処理
        ├── png_import.rs # PNG インポートダイアログ
        └── theme.rs      # UI カラー定数
```

---

## 各ファイルの役割

### `src/main.rs`

- `eframe::run_native()` でウィンドウを起動
- `load_icon()` で `assets/icon.png` を RGBA に変換してアイコン設定
- macOS のみ `native_menu::init()` と `native_menu::set_app_appearance()` を呼び出し
- `editor::app::setup_fonts()` で Noto Sans JP フォントを登録

### `src/native_menu.rs`（macOS 専用）

- `muda` クレートで NSMenu を構築（アプリメニュー・ファイル・編集・表示・パレット）
- `MenuAction` enum でメニューイベントを型安全にディスパッチ
- `thread_local! { static HANDLES }` で `MenuItem`（非 Send）を保持
- `sync_state(can_save, can_undo, dark_mode)` で毎フレーム enabled / checked 状態を更新
- `set_app_appearance(dark: bool)` で `NSAppearance` を設定（ウィンドウ枠の外観）

### `src/io/chr.rs`

- `decode_tile(&[u8]) -> [[u8;8];8]`: 16 バイト → 8×8 カラーインデックス
- `encode_dot(data, px, py, color_idx)`: ドット書き込み
- `decode_block(chr_data, top_left_tile, tiles_per_row, n)`: N×N タイルブロックのデコード
- `render_full_image(chr_data, palette, palette_set, master) -> ColorImage`: 全体レンダリング
- `bank_count(chr_data) -> usize`: 有効バンク数（1 バンク = 0x1000 バイト）

### `src/io/nes.rs`

- `parse_nes(&[u8]) -> Result<NesRom, ParseError>`: iNES パース
- `RomData` enum: `Nes(NesRom)` / `Bin(Vec<u8>)` を統一 API で扱う
- `NesHeader`: PRG/CHR バンク数・マッパー番号・ミラーリング・バッテリー情報
- `NesRom`: ヘッダ・PRG/CHR バイト列・CHR オフセット

### `src/io/png.rs`

- `MappingStrategy` enum: `IndexDirect` / `PaletteMatch` / `RgbApprox`
- `PngImportResult`: 変換後のカラーインデックス配列・警告メッセージ
- PNG PLTE を NES マスターパレットに照合して DAT パレットインデックスに変換

### `src/model/palette.rs`

- `NES_PALETTE: [[u8;3]; 64]`: NES 標準 64 色定数
- `MasterPalette`: 64 色 RGB テーブル（`.pal` ファイルで差し替え可能）
- `DatPalette`: 4 セット × 4 色（各値は MasterPalette のインデックス）

### `src/editor/app.rs`

- `RChrApp` 構造体: アプリ全状態を保持
- `eframe::App::update()` でフレームごとに UI を構築・イベントを委譲
- `setup_fonts()`: Noto Sans JP フォントの登録
- ドラッグ & ドロップ処理、About ダイアログ、未保存変更ダイアログを含む

### `src/editor/keyboard.rs`

- `handle_keyboard(ctx)`: キーボードショートカット処理
- パレットセット切り替え（Z/X/C/V）、アンドゥ（Cmd+Z）、保存（Cmd+S）
- 矢印キーによるタイル選択移動・スクロール追従

### `src/editor/mac/menu_events.rs`（macOS 専用）

- `handle_native_menu(ctx)`: `native_menu::try_recv_action()` からイベントを受け取りディスパッチ
- `native_menu::sync_state()` で enabled / checked 状態を毎フレーム更新

### `src/editor/mac/menu_bar.rs`（非 macOS 専用）

- `show_menu_bar(ctx)`: egui によるメニューバー描画
- ファイル・編集・表示・パレットの各メニュー