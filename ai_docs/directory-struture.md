# ディレクトリ構成

```
r-chr/
├── Cargo.toml
├── assets/
│   ├── icon.png          # ウィンドウアイコン（include_bytes! で埋め込み）
│   ├── rchr.bin          # 起動時デフォルト CHR データ（R-CHR ロゴ）
│   ├── rchr.pal          # デフォルトマスターパレット（192 バイト）
│   ├── rchr.dat          # デフォルト DAT パレット（16 バイト）
│   └── nes.pal           # NES 標準 64 色パレット（リセット用）
├── ai_docs/              # AI 向けドキュメント（本ディレクトリ）
│   ├── architecture.md
│   ├── data-struture.md
│   ├── directory-struture.md
│   ├── implementation-tasks.md
│   ├── requirement.md
│   └── tech-stack.md
└── src/
    ├── main.rs           # エントリポイント・ウィンドウ設定・アイコン読み込み
    ├── native_menu.rs    # macOS ネイティブメニューバー（#[cfg(target_os = "macos")]）
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
        └── app.rs        # RChrApp（eframe::App 実装・メインループ）
```

---

## 各ファイルの役割

### `src/main.rs`

- `eframe::run_native()` でウィンドウを起動
- `load_icon()` で `assets/icon.png` を RGBA に変換してアイコン設定
- macOS のみ `native_menu::init()` と `native_menu::set_app_appearance()` を呼び出し
- `editor::app::setup_fonts()` で日本語フォントを登録

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
- `eframe::App::update()` でフレームごとに UI を構築・イベントを処理
- `setup_fonts()`: Meiryo フォントの登録
- ドラッグ & ドロップ、ショートカットキー、PNG インポートダイアログを含む