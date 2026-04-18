# データ構造

## ファイルデータ（`src/io/nes.rs`）

### `RomData`

アプリが保持する「開いているファイル」の統一 enum。

```rust
pub enum RomData {
    Nes(NesRom),   // iNES (.nes) ファイル
    Bin(Vec<u8>),  // 生 CHR バイナリ (.bin) ファイル
}
```

- `chr_data() -> &[u8]`: CHR バイト列への不変参照
- `chr_data_mut() -> &mut [u8]`: CHR バイト列への可変参照
- `is_nes() -> bool`: NES ファイルかどうか

### `NesRom`

iNES ファイルのパース結果。

```rust
pub struct NesRom {
    pub header: NesHeader,
    pub prg_rom: Vec<u8>,         // PRG-ROM バイト列
    pub chr_rom: Vec<u8>,         // CHR-ROM バイト列（CHR-RAM の場合は空）
    pub chr_data_offset: usize,   // 元ファイル内での CHR-ROM 開始オフセット（保存時に使用）
}
```

### `NesHeader`

```rust
pub struct NesHeader {
    pub prg_rom_banks: u8,        // PRG-ROM バンク数（16KB 単位）
    pub chr_rom_banks: u8,        // CHR-ROM バンク数（8KB 単位）; 0 = CHR-RAM
    pub mapper: u8,               // マッパー番号
    pub vertical_mirroring: bool,
    pub has_battery: bool,
}
```

---

## パレット（`src/model/palette.rs`）

### `MasterPalette`

NES ハードウェアが表示できる 64 色の RGB テーブル。

```rust
pub struct MasterPalette {
    pub colors: [[u8; 3]; 64],  // [color_index][R/G/B]
}
```

- デフォルト値 = `NES_PALETTE` 定数（nesdev.org 準拠）
- `from_pal_bytes(data: &[u8]) -> Option<Self>`: 192 バイトの `.pal` ファイルからパース

### `DatPalette`

NES PPU パレットメモリに相当する 4 セット × 4 色のテーブル。
各値は `MasterPalette` へのインデックス（0x00〜0x3F）。

```rust
pub struct DatPalette {
    pub sets: [[u8; 4]; 4],  // [set_index][color_index] = NES palette index
}
```

- `color_rgb(set, color_idx, master) -> [u8; 3]`: RGB を取得
- `color32(set, color_idx, master) -> egui::Color32`: egui 用 Color32 を取得
- `from_dat_bytes(data: &[u8]) -> Option<Self>`: 16 バイトの `.dat` ファイルからパース
- `to_dat_bytes() -> [u8; 16]`: `.dat` 形式でシリアライズ

---

## PNG インポート（`src/io/png.rs`）

### `MappingStrategy`

```rust
pub enum MappingStrategy {
    IndexDirect,   // インデックス PNG のピクセル値 mod 4（高速・単純）
    PaletteMatch,  // PLTE をマスターパレットに照合（推奨）
    RgbApprox,     // フルカラー PNG を DAT パレット 4 色に RGB 近似
}
```

### `PngImportResult`

```rust
pub struct PngImportResult {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Vec<u8>>,   // [y][x] = CHR カラーインデックス 0〜3
    pub strategy: MappingStrategy,
    pub warnings: Vec<String>,
}
```

---

## アプリ状態（`src/editor/app.rs`）

### `RChrApp`

`eframe::App` を実装するルート構造体。すべての UI 状態を保持する。

```rust
pub struct RChrApp {
    // ── ファイル ──────────────────────────────
    rom: Option<RomData>,           // 現在開いているファイルデータ
    file_name: Option<String>,      // 表示用ファイル名
    file_path: Option<PathBuf>,     // フルパス（上書き保存用）
    raw_file_data: Option<Vec<u8>>, // 元ファイルバイト列（CHR 書き戻し用）
    is_modified: bool,              // 未保存変更フラグ
    error_msg: Option<String>,      // エラーメッセージ

    // ── バンクビュー ──────────────────────────
    bank_texture: Option<egui::TextureHandle>,  // GPU テクスチャキャッシュ
    texture_dirty: bool,            // テクスチャ再生成フラグ
    focus_size: FocusSize,          // 選択ブロックサイズ（8/16/32/64/128 px）
    scroll_addr: usize,             // 現在のスクロール位置（バイトアドレス）
    pending_scroll_addr: Option<usize>, // 次フレームで適用するジャンプ先
    scroll_top_row: usize,          // 先頭表示行（キー操作判定用）
    visible_tile_rows: usize,       // 可視タイル行数

    // ── パレット ──────────────────────────────
    dat_palette: DatPalette,
    master_palette: MasterPalette,
    selected_palette_set: usize,    // 現在選択中のパレットセット（0〜3）
    editing_palette_cell: Option<(usize, usize)>,  // 編集中セル (set, color)

    // ── ドット編集 ────────────────────────────
    selected_tile: Option<usize>,   // 選択中タイルのグローバルインデックス
    drawing_color_idx: u8,          // 描画色インデックス（0〜3）
    drawing_tool: usize,            // 描画ツール種別
    undo_stack: Vec<(usize, [u8; 16])>,  // (tile_offset, 変更前 16 バイト)

    // ── PNG インポート ────────────────────────
    png_import_dialog: Option<PngImportDialog>,

    // ── UI 設定 ───────────────────────────────
    dark_mode: bool,
    show_about: bool,               // About ダイアログ表示フラグ
    status_msg: Option<String>,     // ステータスバー一時メッセージ
    address_input: String,          // アドレスジャンプ入力文字列
}
```

### `FocusSize`

バンクビューの選択ブロックサイズを表す enum。

```rust
enum FocusSize { S8=8, S16=16, S32=32, S64=64, S128=128 }
```

- `tile_count() -> usize`: 1 辺のタイル数（例: S32 → 4）

### `EditorAction`

ドットエディタが発行する操作（描画と状態変更を分離するため）。

```rust
enum EditorAction {
    PaintDot { tile_offset, px, py, color, push_undo },
    Eyedrop { color_idx },
    SelectDrawingColor { color_idx },
}
```

### `PngImportDialog`

PNG インポートダイアログの一時状態。

```rust
struct PngImportDialog {
    png_bytes: Vec<u8>,              // 生 PNG バイト（再変換用）
    file_name: String,
    strategy: MappingStrategy,
    result: PngImportResult,
    preview_texture: Option<egui::TextureHandle>,
    preview_dirty: bool,
}
```

---

## 新規ファイルの初期状態

`new_file()` は 0x4000 バイト（16KB）のゼロデータを `RomData::Bin` として生成する。
ファイル名は `"newfile"`、`file_path` は `None`（保存先未確定）。