use eframe::egui;
use crate::io::chr::render_full_image;
use crate::io::nes::{RomData, parse_nes};
use crate::model::palette::{DatPalette, MasterPalette};
use crate::io::png::{MappingStrategy, PngImportResult};
use super::bank_view::FocusSize;
use super::dot_editor::EditorAction;
use super::theme;

/// デフォルトで読み込むパレットファイル（バイナリに埋め込み）
const DEFAULT_PAL: &[u8] = include_bytes!("../../assets/rchr.pal");
const DEFAULT_DAT: &[u8] = include_bytes!("../../assets/rchr.dat");
/// NES 標準 64色パレット（リセット用）
const NES_PAL: &[u8] = include_bytes!("../../assets/nes.pal");
/// 起動時に表示するデフォルトドット絵（R-CHR ロゴ入り CHR バイナリ）
const DEFAULT_BIN: &[u8] = include_bytes!("../../assets/rchr.bin");

/// 起動時に日本語フォントをセットアップする
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Noto Sans JP Regular — 本文フォント
    fonts.font_data.insert(
        "noto_regular".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Noto_Sans_JP/static/NotoSansJP-Regular.ttf"
        )).into(),
    );
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "noto_regular".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("noto_regular".to_owned());

    // Noto Sans JP Bold — bold_font named family
    fonts.font_data.insert(
        "noto_bold".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Noto_Sans_JP/static/NotoSansJP-Bold.ttf"
        )).into(),
    );
    fonts.families.insert(
        egui::FontFamily::Name("bold_font".into()),
        vec!["noto_bold".to_owned()],
    );

    ctx.set_fonts(fonts);
}

// ── PNG インポートダイアログ状態 ───────────────────────────────────

struct PngImportDialog {
    /// 読み込んだ画像の生バイト（再マッピング用）
    png_bytes: Vec<u8>,
    /// ファイル名（表示用）
    file_name: String,
    /// PNG なら true、BMP なら false（false の場合は RgbApprox のみ使用可）
    is_png: bool,
    /// 現在のマッピング戦略
    strategy: MappingStrategy,
    /// 現在の変換結果
    result: PngImportResult,
    /// プレビューテクスチャ（変換後 CHR 色でレンダリング）
    preview_texture: Option<egui::TextureHandle>,
    /// プレビューテクスチャが古くなっているか
    preview_dirty: bool,
}

impl PngImportDialog {
    fn new(png_bytes: Vec<u8>, file_name: String, is_png: bool, result: PngImportResult) -> Self {
        let strategy = result.strategy;
        Self {
            png_bytes,
            file_name,
            is_png,
            strategy,
            result,
            preview_texture: None,
            preview_dirty: true,
        }
    }
}

// ── アプリ状態 ─────────────────────────────────────────────────────

pub struct RChrApp {
    pub(super) rom: Option<RomData>,
    pub(super) file_name: Option<String>,
    pub(super) error_msg: Option<String>,

    /// スクロール位置（ステータス表示用、毎フレーム更新）
    pub(super) scroll_addr: usize,
    /// アドレスジャンプ時の目標アドレス（次フレームで ScrollArea に適用）
    pub(super) pending_scroll_addr: Option<usize>,
    /// バンクビューの表示先頭行（矢印キーのスクロール判定用）
    pub(super) scroll_top_row: usize,
    /// バンクビューの可視行数（矢印キーのスクロール判定用）
    pub(super) visible_tile_rows: usize,
    pub(super) bank_texture: Option<egui::TextureHandle>,
    pub(super) texture_dirty: bool,

    /// バンクビューの選択ブロックサイズ
    pub(super) focus_size: FocusSize,

    pub(super) dat_palette: DatPalette,
    pub(super) master_palette: MasterPalette,
    pub(super) selected_palette_set: usize,

    /// ステータスバーに表示する一時メッセージ
    pub(super) status_msg: Option<String>,

    pub(super) selected_tile: Option<usize>,

    /// 現在の描画色インデックス（0〜3）
    pub(super) drawing_color_idx: u8,
    /// 選択中の描画ツール（0=pencil, 1=pencil_pattern, ...）
    pub(super) drawing_tool: usize,
    /// アンドゥスタック: (タイルのバイトオフセット, 変更前の 16バイト)
    pub(super) undo_stack: Vec<(usize, [u8; 16])>,

    /// 開いているファイルのフルパス（上書き保存に使用）
    pub(super) file_path: Option<std::path::PathBuf>,
    /// 元のファイルバイト列（CHR 部分を書き戻すために保持）
    pub(super) raw_file_data: Option<Vec<u8>>,
    /// 未保存の変更があるか
    pub(super) is_modified: bool,
    /// アドレスジャンプ入力フィールドの内容（16進数文字列）
    pub(super) address_input: String,

    /// パレットピッカーで編集中のセル (set_idx, color_idx)
    pub(super) editing_palette_cell: Option<(usize, usize)>,

    /// PNG インポートダイアログの状態
    png_import_dialog: Option<PngImportDialog>,

    /// ダークモード有効フラグ
    pub(super) dark_mode: bool,
}

impl Default for RChrApp {
    fn default() -> Self {
        Self {
            rom: Some(RomData::Bin(DEFAULT_BIN.to_vec())),
            file_name: Some("rchr.bin".into()),
            error_msg: None,
            scroll_addr: 0,
            pending_scroll_addr: None,
            scroll_top_row: 0,
            visible_tile_rows: 0,
            bank_texture: None,
            texture_dirty: true,
            focus_size: FocusSize::S8,
            dat_palette: DatPalette::from_dat_bytes(DEFAULT_DAT).unwrap_or_default(),
            master_palette: MasterPalette::from_pal_bytes(DEFAULT_PAL).unwrap_or_default(),
            selected_palette_set: 0,
            status_msg: None,
            selected_tile: Some(0),
            drawing_color_idx: 1,
            drawing_tool: 0,
            undo_stack: Vec::new(),
            file_path: None,
            raw_file_data: None,
            is_modified: false,
            address_input: "000000".into(),
            editing_palette_cell: None,
            png_import_dialog: None,
            dark_mode: true,
        }
    }
}

// ── メインループ ───────────────────────────────────────────────────

impl eframe::App for RChrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut visuals = if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.override_text_color = Some(theme::COL_TEXT);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, theme::COL_BORDER_DARK);
        ctx.set_visuals(visuals);

        // ── macOS ネイティブメニュー: イベント処理 ─────────────────
        #[cfg(target_os = "macos")]
        {
            use crate::native_menu::{self, MenuAction};
            while let Some(action) = native_menu::try_recv_action() {
                match action {
                    MenuAction::FileOpen        => self.open_file(),
                    MenuAction::FileImportPng   => self.open_png_import(),
                    MenuAction::FileSave        => { if let Err(e) = self.save_file()    { self.error_msg = Some(e); } }
                    MenuAction::FileSaveAs      => { if let Err(e) = self.save_file_as() { self.error_msg = Some(e); } }
                    MenuAction::EditUndo        => self.do_undo(),
                    MenuAction::ViewDarkMode(v) => {
                        self.dark_mode = v;
                        native_menu::set_app_appearance(v);
                    }
                    MenuAction::PaletteOpenPal  => self.load_pal_file(),
                    MenuAction::PaletteOpenDat  => self.load_dat_file(),
                    MenuAction::PaletteSaveDat  => self.save_dat_file(),
                    MenuAction::PaletteReset    => {
                        self.master_palette = MasterPalette::from_pal_bytes(NES_PAL)
                            .unwrap_or_default();
                        self.texture_dirty = true;
                        self.status_msg = Some("NES 標準パレットにリセットしました".into());
                    }
                }
            }

            // macOS ネイティブメニュー: enabled / checked 状態を毎フレーム同期
            native_menu::sync_state(
                self.file_path.is_some() && self.is_modified,
                !self.undo_stack.is_empty(),
                self.dark_mode,
            );
        }

        if self.texture_dirty {
            if let Some(rom) = &self.rom {
                if !rom.chr_data().is_empty() {
                    let image = render_full_image(
                        rom.chr_data(),
                        &self.dat_palette,
                        self.selected_palette_set,
                        &self.master_palette,
                    );
                    self.bank_texture = Some(ctx.load_texture(
                        "bank_view",
                        image,
                        egui::TextureOptions::NEAREST,
                    ));
                }
            }
            self.texture_dirty = false;
        }

        // ── ウィンドウ閉じるリクエストの処理
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.is_modified {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                let file_name = self.file_name.as_deref().unwrap_or("(無題)").to_owned();

                #[cfg(target_os = "macos")]
                let choice = {
                    use crate::native_menu::{unsaved_changes_dialog, UnsavedChoice};
                    match unsaved_changes_dialog(&file_name) {
                        UnsavedChoice::Save    => 0,
                        UnsavedChoice::Discard => 1,
                        UnsavedChoice::Cancel  => 2,
                    }
                };
                #[cfg(not(target_os = "macos"))]
                let choice = {
                    use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
                    let r = MessageDialog::new()
                        .set_title("未保存の変更があります")
                        .set_description(format!(
                            "「{file_name}」への変更が保存されていません。\n終了する前に保存しますか？"
                        ))
                        .set_buttons(MessageButtons::YesNoCancel)
                        .set_level(MessageLevel::Warning)
                        .show();
                    match r {
                        MessageDialogResult::Yes => 0,
                        MessageDialogResult::No  => 1,
                        _                        => 2,
                    }
                };

                match choice {
                    0 => { // 保存して閉じる
                        let r = if self.file_path.is_some() {
                            self.save_file()
                        } else {
                            self.save_file_as()
                        };
                        match r {
                            Err(e) => self.error_msg = Some(e),
                            Ok(()) if !self.is_modified => {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            Ok(()) => {} // save_file_as でキャンセルされた
                        }
                    }
                    1 => { // 保存せず閉じる
                        self.is_modified = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    _ => {} // キャンセル
                }
            }
        }

        // ── タイトルバー更新（未保存変更を * で表示）
        let title = if self.is_modified {
            format!(
                "*{}",
                self.file_name.as_deref().unwrap_or("")
            )
        } else {
            format!(
                "{}",
                self.file_name.as_deref().unwrap_or("")
            )
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        // ── メニューバー (macOS はネイティブメニューを使うため非表示)
        #[cfg(not(target_os = "macos"))]
        let _menu_resp = egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("ファイル", |ui| {
                    if ui.button("開く…  ⌘O").clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                    if ui.button("PNG / BMP をインポート…").clicked() {
                        self.open_png_import();
                        ui.close_menu();
                    }
                    ui.separator();
                    let can_save = self.file_path.is_some() && self.is_modified;
                    if ui.add_enabled(can_save, egui::Button::new("保存  ⌘S")).clicked() {
                        if let Err(e) = self.save_file() {
                            self.error_msg = Some(e);
                        }
                        ui.close_menu();
                    }
                    if ui.button("別名で保存…  ⌘⇧S").clicked() {
                        if let Err(e) = self.save_file_as() {
                            self.error_msg = Some(e);
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("編集", |ui| {
                    let can_undo = !self.undo_stack.is_empty();
                    if ui.add_enabled(can_undo, egui::Button::new("元に戻す  ⌘Z / Ctrl+Z")).clicked() {
                        self.do_undo();
                        ui.close_menu();
                    }
                });
                ui.menu_button("表示", |ui| {
                    if ui.checkbox(&mut self.dark_mode, "ダークモード").clicked() {
                        ui.close_menu();
                    }
                });
                ui.menu_button("パレット", |ui| {
                    if ui.button("PAL ファイルを開く…").clicked() {
                        self.load_pal_file();
                        ui.close_menu();
                    }
                    if ui.button("DAT ファイルを開く…").clicked() {
                        self.load_dat_file();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("DAT ファイルを保存…").clicked() {
                        self.save_dat_file();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("マスターパレットをリセット (NES 標準)").clicked() {
                        self.master_palette = MasterPalette::from_pal_bytes(NES_PAL)
                            .unwrap_or_default();
                        self.texture_dirty = true;
                        self.status_msg = Some("NES 標準パレットにリセットしました".into());
                        ui.close_menu();
                    }
                });
            });
        });

        // メニューバー直下の1pxボーダー
        egui::TopBottomPanel::top("top_border")
            .exact_height(1.0)
            .frame(egui::Frame::new().fill(theme::COL_BORDER_DARK))
            .show(ctx, |_ui| {});

        // ── 右パネル（情報・描画色・パレット - 245px固定）
        let info_resp = egui::SidePanel::right("info_panel")
            .resizable(false)
            .exact_width(245.0)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::symmetric(12, 8)).fill(theme::COL_PANEL_BG))
            .show(ctx, |ui| {
                self.show_info_panel(ui);
            });

        // ── 中央パネル（ドットエディタ）
        let mut editor_action: Option<EditorAction> = None;
        let dot_resp = egui::SidePanel::right("dot_editor_panel")
            .resizable(true)
            .default_width(420.0)
            .min_width(180.0)
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                editor_action = self.show_dot_editor(ui);
            });

        // ── バンクビュー（メイン）
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
            if let Some(err) = self.error_msg.clone() {
                ui.colored_label(egui::Color32::RED, format!("エラー: {err}"));
                if ui.button("閉じる").clicked() {
                    self.error_msg = None;
                }
                return;
            }
            match &self.rom {
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label("ファイルメニューから NES / BIN ファイルを開いてください");
                    });
                }
                Some(rom) => {
                    if rom.chr_data().is_empty() {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.colored_label(
                                egui::Color32::YELLOW,
                                "この ROM は CHR-RAM を使用しています（CHR データなし）",
                            );
                        });
                        return;
                    }
                    self.show_bank_view(ui);
                }
            }
        });

        // ── アクション適用（UI 描画後）
        if let Some(action) = editor_action {
            self.apply_action(action);
        }

        // suppress unused warnings for panel responses
        let _ = (info_resp, dot_resp);

        // ── PNG インポートダイアログ
        if self.png_import_dialog.is_some() {
            self.show_png_import_dialog(ctx);
        }

        // ── ドラッグ＆ドロップ
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
                _             => {}
            }
        }

        self.handle_keyboard(ctx);
    }
}

// ── ZIP ユーティリティ ─────────────────────────────────────────────

/// ZIP バイト列から最初の .nes ファイルを取り出す。
/// 戻り値: (ファイル名, NES バイト列)
fn extract_nes_from_zip(zip_data: &[u8]) -> Result<(String, Vec<u8>), String> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("ZIP 読み込み失敗: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("ZIP エントリ読み込み失敗: {e}"))?;
        if entry.name().to_ascii_lowercase().ends_with(".nes") {
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data)
                .map_err(|e| format!("ZIP 展開失敗: {e}"))?;
            return Ok((name, data));
        }
    }
    Err("ZIP 内に .nes ファイルが見つかりませんでした".into())
}

// ── ファイル操作 ───────────────────────────────────────────────────

impl RChrApp {
    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NES / BIN / ZIP", &["nes", "bin", "zip"])
            .add_filter("すべてのファイル", &["*"])
            .pick_file()
        else {
            return;
        };
        self.open_file_from_path(&path);
    }

    fn open_file_from_path(&mut self, path: &std::path::Path) {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        let data = match std::fs::read(path) {
            Err(e) => {
                self.error_msg = Some(format!("読み込み失敗: {e}"));
                return;
            }
            Ok(d) => d,
        };

        // ZIP の場合: 内部の最初の .nes を取り出して処理
        let (nes_data, display_name, save_path) = if ext == "zip" {
            match extract_nes_from_zip(&data) {
                Err(e) => {
                    self.error_msg = Some(e);
                    return;
                }
                Ok((inner_name, inner_data)) => {
                    // ZIP から展開した場合は保存先が確定しないので file_path は None
                    (inner_data, inner_name, None)
                }
            }
        } else {
            (data, file_name, Some(path.to_path_buf()))
        };

        let rom_data = if save_path.as_ref().map_or(false, |p| {
            p.extension().and_then(|e| e.to_str()).map_or(false, |e| e.eq_ignore_ascii_case("bin"))
        }) || (ext == "bin") {
            if nes_data.is_empty() {
                self.error_msg = Some("BIN ファイルが空です".into());
                return;
            }
            self.raw_file_data = None;
            RomData::Bin(nes_data)
        } else {
            match parse_nes(&nes_data) {
                Err(e) => {
                    self.error_msg = Some(e.to_string());
                    return;
                }
                Ok(rom) => {
                    self.raw_file_data = Some(nes_data);
                    RomData::Nes(rom)
                }
            }
        };

        self.error_msg = None;
        self.file_name = Some(display_name);
        self.scroll_addr = 0;
        self.pending_scroll_addr = Some(0);
        self.selected_tile = None;
        self.undo_stack.clear();
        self.file_path = save_path;
        self.is_modified = false;
        self.rom = Some(rom_data);
        self.texture_dirty = true;
    }

    // ── 保存 ──────────────────────────────────────────────────────

    /// 現在のパスに上書き保存する
    fn save_file(&mut self) -> Result<(), String> {
        let path = self.file_path.clone().ok_or("保存先パスがありません")?;
        self.write_to_path(&path)
    }

    /// ダイアログで保存先を選んで保存する
    fn save_file_as(&mut self) -> Result<(), String> {
        let is_bin = self.rom.as_ref().map_or(false, |r| !r.is_nes());
        let default_name = self.file_name.clone().unwrap_or_else(|| {
            if is_bin { "output.bin".into() } else { "output.nes".into() }
        });
        let mut dialog = rfd::FileDialog::new().set_file_name(&default_name);
        dialog = if is_bin {
            dialog.add_filter("CHR バイナリ", &["bin"])
        } else {
            dialog.add_filter("NES ROM", &["nes"])
        };
        let Some(path) = dialog.save_file() else {
            return Ok(()); // キャンセル
        };
        self.write_to_path(&path)?;
        // 保存先を新しいパスに更新
        self.file_name = Some(
            path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        );
        self.file_path = Some(path);
        Ok(())
    }

    /// CHR データをファイルへ出力する（NES: 元データに書き戻し / BIN: CHR データをそのまま書き出し）
    fn write_to_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        let rom = self.rom.as_ref().ok_or("ROM が読み込まれていません")?;

        match rom {
            RomData::Nes(nes_rom) => {
                let raw = self.raw_file_data.as_mut().ok_or("元ファイルデータがありません")?;
                let start = nes_rom.chr_data_offset;
                let end   = start + nes_rom.chr_rom.len();
                if end > raw.len() {
                    return Err("ファイルサイズが不正です".into());
                }
                raw[start..end].copy_from_slice(&nes_rom.chr_rom);
                std::fs::write(path, raw as &[u8]).map_err(|e| format!("保存失敗: {e}"))?;
            }
            RomData::Bin(chr_data) => {
                std::fs::write(path, chr_data).map_err(|e| format!("保存失敗: {e}"))?;
            }
        }

        self.is_modified = false;
        Ok(())
    }

    // ── PAL / DAT パレットファイル操作 ───────────────────────────

    /// .pal ファイル（64色 × RGB 3バイト = 192バイト）を読み込む
    fn load_pal_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NES パレット", &["pal"])
            .add_filter("すべてのファイル", &["*"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Err(e) => {
                self.error_msg = Some(format!("読み込み失敗: {e}"));
            }
            Ok(data) => match MasterPalette::from_pal_bytes(&data) {
                None => {
                    self.error_msg = Some(
                        format!("PAL ファイルが短すぎます（{}バイト、192バイト必要）", data.len())
                    );
                }
                Some(master) => {
                    self.master_palette = master;
                    self.texture_dirty = true;
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    self.status_msg = Some(format!("PAL 読み込み: {name}"));
                }
            },
        }
    }

    /// .dat ファイル（4セット × 4色 = 16バイト以上）を読み込む
    fn load_dat_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("DAT パレット", &["dat"])
            .add_filter("すべてのファイル", &["*"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Err(e) => {
                self.error_msg = Some(format!("読み込み失敗: {e}"));
            }
            Ok(data) => match DatPalette::from_dat_bytes(&data) {
                None => {
                    self.error_msg = Some(
                        format!("DAT ファイルが短すぎます（{}バイト、16バイト必要）", data.len())
                    );
                }
                Some(palette) => {
                    self.dat_palette = palette;
                    self.texture_dirty = true;
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    self.status_msg = Some(format!("DAT 読み込み: {name}"));
                }
            },
        }
    }

    /// 現在の dat_palette を .dat ファイルとして保存する
    fn save_dat_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("DAT パレット", &["dat"])
            .set_file_name("palette.dat")
            .save_file()
        else {
            return;
        };
        let bytes = self.dat_palette.to_dat_bytes();
        match std::fs::write(&path, &bytes) {
            Err(e) => {
                self.error_msg = Some(format!("保存失敗: {e}"));
            }
            Ok(()) => {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                self.status_msg = Some(format!("DAT 保存: {name}"));
            }
        }
    }

    // ── PNG インポート ────────────────────────────────────────────

    /// メニューから PNG / BMP ファイルを選択して開く
    fn open_png_import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG / BMP 画像", &["png", "bmp"])
            .add_filter("すべてのファイル", &["*"])
            .pick_file()
        else {
            return;
        };
        self.open_png_import_from_path(&path);
    }

    /// パスを直接指定して画像インポートダイアログを開く（D&D 用）
    fn open_png_import_from_path(&mut self, path: &std::path::Path) {
        if self.rom.is_none() {
            self.error_msg = Some("先に NES / BIN ファイルを開いてください".into());
            return;
        }
        let img_bytes = match std::fs::read(path) {
            Err(e) => {
                self.error_msg = Some(format!("画像読み込み失敗: {e}"));
                return;
            }
            Ok(b) => b,
        };
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        let is_png = ext != "bmp";
        self.open_image_import_with_bytes(img_bytes, file_name, is_png);
    }

    fn open_image_import_with_bytes(&mut self, img_bytes: Vec<u8>, file_name: String, is_png: bool) {
        let result = if is_png {
            crate::io::png::import_png(
                &img_bytes,
                &self.dat_palette,
                self.selected_palette_set,
                &self.master_palette,
                None,
            )
        } else {
            crate::io::png::import_bmp(
                &img_bytes,
                &self.dat_palette,
                self.selected_palette_set,
                &self.master_palette,
            )
        };
        match result {
            Err(e) => self.error_msg = Some(format!("変換失敗: {e}")),
            Ok(r) => self.png_import_dialog = Some(PngImportDialog::new(img_bytes, file_name, is_png, r)),
        }
    }

    /// PNG インポートダイアログを表示する
    fn show_png_import_dialog(&mut self, ctx: &egui::Context) {
        let dialog = match &mut self.png_import_dialog {
            Some(d) => d,
            None => return,
        };

        // プレビューテクスチャの更新
        if dialog.preview_dirty {
            let w = dialog.result.width;
            let h = dialog.result.height;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..h {
                for x in 0..w {
                    let ci = dialog.result.pixels[y][x] as usize;
                    let [r, g, b] = self.dat_palette.color_rgb(
                        self.selected_palette_set, ci, &self.master_palette,
                    );
                    let i = (y * w + x) * 4;
                    rgba[i]     = r;
                    rgba[i + 1] = g;
                    rgba[i + 2] = b;
                    rgba[i + 3] = 255;
                }
            }
            let image = egui::ColorImage::from_rgba_unmultiplied([w.max(1), h.max(1)], &rgba);
            dialog.preview_texture = Some(ctx.load_texture(
                "png_preview",
                image,
                egui::TextureOptions::NEAREST,
            ));
            dialog.preview_dirty = false;
        }

        // ── ダイアログウィンドウ
        let mut do_import = false;
        let mut do_close  = false;
        let mut new_strategy: Option<MappingStrategy> = None;

        egui::Window::new("画像インポート")
            .resizable(true)
            .min_width(360.0)
            .show(ctx, |ui| {
                // ファイル情報
                let tw = dialog.result.tile_width();
                let th = dialog.result.tile_height();
                ui.label(format!(
                    "ファイル: {}  ({}×{} px = {}×{} タイル)",
                    dialog.file_name,
                    dialog.result.width, dialog.result.height,
                    tw, th,
                ));
                ui.add_space(6.0);

                // マッピング戦略選択（BMP は RgbApprox のみ）
                ui.label("マッピング戦略:");
                ui.horizontal(|ui| {
                    for s in [MappingStrategy::PaletteMatch, MappingStrategy::IndexDirect, MappingStrategy::RgbApprox] {
                        let enabled = dialog.is_png || s == MappingStrategy::RgbApprox;
                        let resp = ui.add_enabled(enabled, egui::RadioButton::new(dialog.strategy == s, s.label()));
                        if resp.clicked() && dialog.strategy != s {
                            new_strategy = Some(s);
                        }
                    }
                });
                if !dialog.is_png {
                    ui.colored_label(egui::Color32::GRAY, "BMP はインデックスカラー情報がないため RGB 近似のみ使用できます");
                }
                ui.add_space(6.0);

                // 警告表示
                if !dialog.result.warnings.is_empty() {
                    for w in &dialog.result.warnings {
                        ui.colored_label(egui::Color32::YELLOW, format!("⚠ {w}"));
                    }
                    ui.add_space(4.0);
                }

                // プレビュー
                ui.label("プレビュー（変換後）:");
                if let Some(tex) = &dialog.preview_texture {
                    let pw = (dialog.result.width  * 2).min(512) as f32;
                    let ph = (dialog.result.height * 2).min(512) as f32;
                    let ratio = dialog.result.width as f32 / dialog.result.height.max(1) as f32;
                    let (pw, ph) = if pw / ph > ratio {
                        (ph * ratio, ph)
                    } else {
                        (pw, pw / ratio.max(0.01))
                    };
                    ui.image(egui::load::SizedTexture::new(tex.id(), egui::vec2(pw, ph)));
                }
                ui.add_space(8.0);

                // 貼り付け先情報
                let dest_tile = self.selected_tile.unwrap_or(0);
                ui.label(format!("貼り付け先: タイル {} (0x{:06X}) から", dest_tile, dest_tile * 16));
                ui.add_space(8.0);

                // ボタン行
                ui.horizontal(|ui| {
                    if ui.button("貼り付け").clicked() {
                        do_import = true;
                    }
                    if ui.button("キャンセル").clicked() {
                        do_close = true;
                    }
                });
            });

        // 戦略変更時は再マッピング
        if let Some(s) = new_strategy {
            let (img_bytes, is_png) = {
                let d = self.png_import_dialog.as_ref().unwrap();
                (d.png_bytes.clone(), d.is_png)
            };
            let result = if is_png {
                crate::io::png::import_png(&img_bytes, &self.dat_palette, self.selected_palette_set, &self.master_palette, Some(s))
            } else {
                crate::io::png::import_bmp(&img_bytes, &self.dat_palette, self.selected_palette_set, &self.master_palette)
            };
            match result {
                Ok(result) => {
                    let dialog = self.png_import_dialog.as_mut().unwrap();
                    dialog.strategy = s;
                    dialog.result = result;
                    dialog.preview_dirty = true;
                }
                Err(e) => {
                    self.error_msg = Some(format!("変換失敗: {e}"));
                }
            }
        }

        // 貼り付け実行
        if do_import {
            self.apply_png_import();
            self.png_import_dialog = None;
            return;
        }

        if do_close {
            self.png_import_dialog = None;
        }
    }

    /// PNG インポート結果を CHR データに書き込む
    fn apply_png_import(&mut self) {
        let dialog = match &self.png_import_dialog {
            Some(d) => d,
            None => return,
        };
        let Some(rom) = &mut self.rom else { return };
        let top_left_tile = self.selected_tile.unwrap_or(0);
        let chr_len = rom.chr_data().len();

        // Undo 用: 影響範囲の全タイルを保存
        let tw = dialog.result.tile_width();
        let th = dialog.result.tile_height();
        let top_row = top_left_tile / 16;
        let top_col = top_left_tile % 16;
        for by in 0..th {
            for bx in 0..tw {
                let tile_global = (top_row + by) * 16 + (top_col + bx);
                let offset = tile_global * 16;
                if offset + 16 <= chr_len {
                    let saved: [u8; 16] = rom.chr_data()[offset..offset + 16].try_into().unwrap();
                    if self.undo_stack.len() >= 200 {
                        self.undo_stack.remove(0);
                    }
                    self.undo_stack.push((offset, saved));
                }
            }
        }

        // CHR へ書き込み
        let result = &dialog.result;
        crate::io::png::write_to_chr(rom.chr_data_mut(), result, top_left_tile, 16);

        self.is_modified = true;
        self.texture_dirty = true;
        let (tw, th) = (result.tile_width(), result.tile_height());
        self.status_msg = Some(format!("PNG インポート完了: {}×{} タイル", tw, th));
    }

    // ── キーボード操作 ────────────────────────────────────────────

    pub(super) fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let chr_empty = self.rom.as_ref().map_or(true, |r| r.chr_data().is_empty());
        if chr_empty { return }

        let mut new_palette_set: Option<usize> = None;
        let mut do_undo = false;
        let mut do_save = false;
        let mut do_save_as = false;
        let mut d_col: i32 = 0; // 矢印キーによる列移動量（ブロック単位）
        let mut d_row: i32 = 0; // 矢印キーによる行移動量（ブロック単位）

        ctx.input(|i| {
            let cmd = i.modifiers.ctrl || i.modifiers.mac_cmd;

            // macOS では NSMenu が Cmd+Z / Cmd+S をインターセプトするため egui には届かない。
            // NSMenu が機能しない場合のフォールバックとして egui 側にも残す（二重実行にはならない）。
            if cmd && i.key_pressed(egui::Key::Z) {
                do_undo = true;
            } else if i.key_pressed(egui::Key::Z) {
                new_palette_set = Some(0);
            }
            if i.key_pressed(egui::Key::X) { new_palette_set = Some(1); }
            if i.key_pressed(egui::Key::C) { new_palette_set = Some(2); }
            if i.key_pressed(egui::Key::V) { new_palette_set = Some(3); }

            if cmd && i.key_pressed(egui::Key::S) {
                if i.modifiers.shift { do_save_as = true; } else { do_save = true; }
            }

            // 矢印キー（フォーカスブロック単位で移動）
            if i.key_pressed(egui::Key::ArrowRight) { d_col += 1; }
            if i.key_pressed(egui::Key::ArrowLeft)  { d_col -= 1; }
            if i.key_pressed(egui::Key::ArrowDown)  { d_row += 1; }
            if i.key_pressed(egui::Key::ArrowUp)    { d_row -= 1; }
        });

        if let Some(set) = new_palette_set {
            self.selected_palette_set = set;
            self.texture_dirty = true;
        }
        if do_undo { self.do_undo(); }
        if do_save {
            if let Err(e) = self.save_file() { self.error_msg = Some(e); }
        }
        if do_save_as {
            if let Err(e) = self.save_file_as() { self.error_msg = Some(e); }
        }

        // 矢印キーによるタイル選択移動（起点は常に 1 タイル単位）
        if d_col != 0 || d_row != 0 {
            let total_tiles = self.rom.as_ref().map_or(0, |r| r.chr_data().len() / 16);
            if total_tiles == 0 { return; }
            let total_rows = ((total_tiles + 15) / 16) as i32;

            let current = self.selected_tile.unwrap_or(0);
            let cur_col = (current % 16) as i32;
            let cur_row = (current / 16) as i32;

            // 1 タイル単位で移動し、端でクランプ
            let new_col = (cur_col + d_col).clamp(0, 15) as usize;
            let new_row = (cur_row + d_row).clamp(0, total_rows - 1) as usize;
            let new_tile = new_row * 16 + new_col;

            if new_tile < total_tiles {
                self.selected_tile = Some(new_tile);
                // 選択タイルが可視範囲外に出た場合のみスクロール
                let visible_end = self.scroll_top_row + self.visible_tile_rows.max(1);
                if new_row < self.scroll_top_row {
                    // 上に出た → 選択行を先頭に
                    self.pending_scroll_addr = Some(new_row * 0x100);
                } else if new_row >= visible_end {
                    // 下に出た → 選択行が末尾に来るよう調整
                    let start_row = new_row + 1 - self.visible_tile_rows.max(1);
                    self.pending_scroll_addr = Some(start_row * 0x100);
                }
                // 可視範囲内なら scroll しない（チラツキ防止）
            }
        }
    }
}