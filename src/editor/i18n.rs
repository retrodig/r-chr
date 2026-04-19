//! 多言語対応（日本語 / English）

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang { #[default] Ja, En }

/// UI で使う全静的文字列のセット
pub struct Strings {
    // ── アプリメニュー (R-CHR) ──
    pub about:              &'static str,
    pub lang_english:       &'static str,   // チェックアイテムのラベル

    // ── ファイルメニュー ──
    pub menu_file:          &'static str,
    pub file_new:           &'static str,
    pub file_open:          &'static str,
    pub file_import_png:    &'static str,
    pub file_save:          &'static str,
    pub file_save_as:       &'static str,

    // ── 編集メニュー ──
    pub menu_edit:          &'static str,
    pub edit_undo:          &'static str,
    pub edit_copy:          &'static str,
    pub edit_paste:         &'static str,

    // ── 表示メニュー ──
    pub menu_view:          &'static str,
    pub view_dark_mode:     &'static str,

    // ── パレットメニュー ──
    pub menu_palette:       &'static str,
    pub pal_open_pal:       &'static str,
    pub pal_open_dat:       &'static str,
    pub pal_save_dat:       &'static str,
    pub pal_reset:          &'static str,

    // ── バンクビュー ──
    pub address:            &'static str,
    pub go:                 &'static str,

    // ── 情報パネル ──
    pub tile:               &'static str,
    pub drawing_color:      &'static str,
    pub nes_palette:        &'static str,
    pub palette_section:    &'static str,
    pub palette_click_hint: &'static str,

    // ── ドットエディタ ──
    pub click_tile_hint:    &'static str,

    // ── PNG インポートダイアログ ──
    pub img_import_title:   &'static str,
    pub mapping_strategy:   &'static str,
    pub preview_label:      &'static str,
    pub paste_btn:          &'static str,
    pub cancel_btn:         &'static str,
    pub bmp_note:           &'static str,
    pub close_btn:          &'static str,

    // ── About ダイアログ ──
    pub about_desc:         &'static str,

    // ── 未保存ダイアログ ──
    pub unsaved_title:      &'static str,
    pub save_and_close:     &'static str,
    pub discard_btn:        &'static str,
    pub cancel_btn2:        &'static str,   // NSAlert 用（close_btn と区別）

    // ── ファイルダイアログ フィルタ ──
    pub filter_all:         &'static str,
    pub filter_chr:         &'static str,
    pub filter_pal:         &'static str,
    pub filter_dat:         &'static str,
    pub filter_png_bmp:     &'static str,

    // ── ステータス / エラー（静的部分） ──
    pub status_pal_reset:   &'static str,
    pub err_no_rom:         &'static str,
    pub err_bin_empty:      &'static str,
    pub err_no_save_path:   &'static str,
    pub err_no_raw:         &'static str,
    pub err_filesize:       &'static str,
    pub err_zip_no_nes:     &'static str,
    pub err_no_file_first:  &'static str,
    pub err_idx_direct:     &'static str,
    pub err_no_plte:        &'static str,
    pub err_pal_match_only: &'static str,
    pub err_img_size:       &'static str,

    // ── CHR-RAM メッセージ ──
    pub chr_ram_note:       &'static str,

    // ── バンクビュー 「タイルをクリック」 ──
    pub no_file_hint:       &'static str,
}

static JA: Strings = Strings {
    about:              "R-CHR について…",
    lang_english:       "English",

    menu_file:          "ファイル",
    file_new:           "新規作成",
    file_open:          "開く…",
    file_import_png:    "PNG / BMP をインポート…",
    file_save:          "保存",
    file_save_as:       "別名で保存…",

    menu_edit:          "編集",
    edit_undo:          "元に戻す",
    edit_copy:          "タイルをコピー",
    edit_paste:         "タイルをペースト",

    menu_view:          "表示",
    view_dark_mode:     "ダークモード",

    menu_palette:       "パレット",
    pal_open_pal:       "PAL ファイルを開く…",
    pal_open_dat:       "DAT ファイルを開く…",
    pal_save_dat:       "DAT ファイルを保存…",
    pal_reset:          "マスターパレットをリセット (NES 標準)",

    address:            "アドレス",
    go:                 "移動",

    tile:               "タイル",
    drawing_color:      "描画色",
    nes_palette:        "NES パレット",
    palette_section:    "パレット",
    palette_click_hint: "パレットの色をクリックして変更",

    click_tile_hint:    "← タイルをクリックして選択",

    img_import_title:   "画像インポート",
    mapping_strategy:   "マッピング戦略:",
    preview_label:      "プレビュー（変換後）:",
    paste_btn:          "貼り付け",
    cancel_btn:         "キャンセル",
    bmp_note:           "BMP はインデックスカラー情報がないため RGB 近似のみ使用できます",
    close_btn:          "  閉じる  ",

    about_desc:         "NES CHR エディタ",

    unsaved_title:      "未保存の変更があります",
    save_and_close:     "保存して閉じる",
    discard_btn:        "保存せず閉じる",
    cancel_btn2:        "キャンセル",

    filter_all:         "すべてのファイル",
    filter_chr:         "CHR バイナリ",
    filter_pal:         "NES パレット",
    filter_dat:         "DAT パレット",
    filter_png_bmp:     "PNG / BMP 画像",

    status_pal_reset:   "NES 標準パレットにリセットしました",
    err_no_rom:         "ROM が読み込まれていません",
    err_bin_empty:      "BIN ファイルが空です",
    err_no_save_path:   "保存先パスがありません",
    err_no_raw:         "元ファイルデータがありません",
    err_filesize:       "ファイルサイズが不正です",
    err_zip_no_nes:     "ZIP 内に .nes ファイルが見つかりませんでした",
    err_no_file_first:  "先に NES / BIN ファイルを開いてください",
    err_idx_direct:     "インデックス直接モードはインデックスカラー PNG 専用です",
    err_no_plte:        "PLTE（パレット）チャンクが見つかりません",
    err_pal_match_only: "パレット照合モードはインデックスカラー PNG 専用です",
    err_img_size:       "画像サイズ不一致",

    chr_ram_note:       "この ROM は CHR-RAM を使用しています（CHR データなし）",
    no_file_hint:       "ファイルメニューから NES / BIN ファイルを開いてください",
};

static EN: Strings = Strings {
    about:              "About R-CHR…",
    lang_english:       "English",

    menu_file:          "File",
    file_new:           "New File",
    file_open:          "Open…",
    file_import_png:    "Import PNG / BMP…",
    file_save:          "Save",
    file_save_as:       "Save As…",

    menu_edit:          "Edit",
    edit_undo:          "Undo",
    edit_copy:          "Copy Tiles",
    edit_paste:         "Paste Tiles",

    menu_view:          "View",
    view_dark_mode:     "Dark Mode",

    menu_palette:       "Palette",
    pal_open_pal:       "Open PAL File…",
    pal_open_dat:       "Open DAT File…",
    pal_save_dat:       "Save DAT File…",
    pal_reset:          "Reset Master Palette (NES Standard)",

    address:            "Address",
    go:                 "Go",

    tile:               "Tile",
    drawing_color:      "Color",
    nes_palette:        "NES Palette",
    palette_section:    "Palette",
    palette_click_hint: "Click a color to change it",

    click_tile_hint:    "← Click a tile to select",

    img_import_title:   "Import Image",
    mapping_strategy:   "Mapping Strategy:",
    preview_label:      "Preview (after conversion):",
    paste_btn:          "Paste",
    cancel_btn:         "Cancel",
    bmp_note:           "BMP has no index color info — RGB Approximate only",
    close_btn:          "  Close  ",

    about_desc:         "NES CHR Editor",

    unsaved_title:      "Unsaved Changes",
    save_and_close:     "Save and Close",
    discard_btn:        "Discard",
    cancel_btn2:        "Cancel",

    filter_all:         "All Files",
    filter_chr:         "CHR Binary",
    filter_pal:         "NES Palette",
    filter_dat:         "DAT Palette",
    filter_png_bmp:     "PNG / BMP Images",

    status_pal_reset:   "Master palette reset to NES standard",
    err_no_rom:         "No ROM loaded",
    err_bin_empty:      "BIN file is empty",
    err_no_save_path:   "No save path set",
    err_no_raw:         "Original file data not available",
    err_filesize:       "File size mismatch",
    err_zip_no_nes:     "No .nes file found inside ZIP",
    err_no_file_first:  "Please open a NES / BIN file first",
    err_idx_direct:     "Index Direct mode requires an indexed-color PNG",
    err_no_plte:        "No PLTE (palette) chunk found",
    err_pal_match_only: "Palette Match mode requires an indexed-color PNG",
    err_img_size:       "Image size mismatch",

    chr_ram_note:       "This ROM uses CHR-RAM (no CHR data)",
    no_file_hint:       "Open a NES / BIN file from the File menu",
};

/// 現在の言語の文字列セットを返す
pub fn t(lang: Lang) -> &'static Strings {
    match lang { Lang::Ja => &JA, Lang::En => &EN }
}

impl Lang {
    /// format 文字列ヘルパー群
    pub fn fmt_tile_addr(self, addr: usize, tiles: usize) -> String {
        match self {
            Self::Ja => format!("0x{addr:06X}  ({tiles} タイル)"),
            Self::En => format!("0x{addr:06X}  ({tiles} tiles)"),
        }
    }
    pub fn fmt_palette_editing(self, set: usize, color: usize) -> String {
        match self {
            Self::Ja => format!("セット #{set}  色 {color} を変更"),
            Self::En => format!("Set #{set}  Color {color}"),
        }
    }
    pub fn fmt_nes_hover(self, idx: usize) -> String {
        match self {
            Self::Ja => format!("NES 0x{idx:02X}  クリックで変更"),
            Self::En => format!("NES 0x{idx:02X}  click to change"),
        }
    }
    pub fn fmt_img_file(self, name: &str, pw: u32, ph: u32, tw: u32, th: u32) -> String {
        match self {
            Self::Ja => format!("ファイル: {name}  ({pw}×{ph} px = {tw}×{th} タイル)"),
            Self::En => format!("File: {name}  ({pw}×{ph} px = {tw}×{th} tiles)"),
        }
    }
    pub fn fmt_paste_at(self, tile: usize) -> String {
        match self {
            Self::Ja => format!("貼り付け先: タイル {} (0x{:06X}) から", tile, tile * 16),
            Self::En => format!("Paste at: Tile {} (0x{:06X})", tile, tile * 16),
        }
    }
    pub fn fmt_png_done(self, tw: usize, th: usize) -> String {
        match self {
            Self::Ja => format!("PNG インポート完了: {tw}×{th} タイル"),
            Self::En => format!("PNG import done: {tw}×{th} tiles"),
        }
    }
    pub fn fmt_pal_loaded(self, name: &str) -> String {
        match self {
            Self::Ja => format!("PAL 読み込み: {name}"),
            Self::En => format!("PAL loaded: {name}"),
        }
    }
    pub fn fmt_dat_loaded(self, name: &str) -> String {
        match self {
            Self::Ja => format!("DAT 読み込み: {name}"),
            Self::En => format!("DAT loaded: {name}"),
        }
    }
    pub fn fmt_dat_saved(self, name: &str) -> String {
        match self {
            Self::Ja => format!("DAT 保存: {name}"),
            Self::En => format!("DAT saved: {name}"),
        }
    }
    pub fn fmt_pal_too_short(self, len: usize) -> String {
        match self {
            Self::Ja => format!("PAL ファイルが短すぎます（{len}バイト、192バイト必要）"),
            Self::En => format!("PAL file too short ({len} bytes, need 192)"),
        }
    }
    pub fn fmt_unsaved_body(self, file_name: &str) -> String {
        match self {
            Self::Ja => format!("「{file_name}」への変更が保存されていません。\n終了する前に保存しますか？"),
            Self::En => format!("Changes to \"{file_name}\" have not been saved.\nSave before closing?"),
        }
    }
    pub fn fmt_idx_warn(self, max_val: u8) -> String {
        match self {
            Self::Ja => format!("インデックス値の最大が {} です。mod 4 で変換します。", max_val),
            Self::En => format!("Max index value is {}; converting with mod 4.", max_val),
        }
    }
    pub fn fmt_transparent_px(self, n: usize) -> String {
        match self {
            Self::Ja => format!("透明ピクセル {n} px → インデックス 0 に変換"),
            Self::En => format!("{n} transparent pixel(s) → mapped to index 0"),
        }
    }
    pub fn fmt_transparent_pal(self, n: usize) -> String {
        match self {
            Self::Ja => format!("透明パレットエントリ {n} 色 → インデックス 0 に変換"),
            Self::En => format!("{n} transparent palette entry(s) → mapped to index 0"),
        }
    }
    pub fn fmt_approx_colors(self, n: usize) -> String {
        match self {
            Self::Ja => format!("{n} 色がパレットに完全一致せず近似されました"),
            Self::En => format!("{n} color(s) could not be exactly matched and were approximated"),
        }
    }
    pub fn fmt_approx_pixels(self, n: usize) -> String {
        match self {
            Self::Ja => format!("{n} ピクセルが DAT パレット 4 色に完全一致せず近似されました"),
            Self::En => format!("{n} pixel(s) approximated to the nearest DAT palette color"),
        }
    }
}