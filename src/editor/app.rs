use eframe::egui;
use crate::io::chr::render_full_image;
use crate::io::nes::RomData;
use crate::model::palette::{DatPalette, MasterPalette};
use super::bank_view::FocusSize;
use super::dot_editor::EditorAction;
use super::theme;
use super::png_import::PngImportDialog;

/// デフォルトで読み込むパレットファイル（バイナリに埋め込み）
const DEFAULT_PAL: &[u8] = include_bytes!("../../assets/rchr.pal");
const DEFAULT_DAT: &[u8] = include_bytes!("../../assets/rchr.dat");
/// NES 標準 64色パレット（リセット用）
pub(super) const NES_PAL: &[u8] = include_bytes!("../../assets/nes.pal");
/// 起動時に表示するデフォルトドット絵（R-CHR ロゴ入り CHR バイナリ）
const DEFAULT_BIN: &[u8] = include_bytes!("../../assets/rchr.bin");

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
    /// アンドゥスタック: 1操作 = Vec<(バイトオフセット, 変更前16バイト)>
    pub(super) undo_stack: Vec<Vec<(usize, [u8; 16])>>,
    /// 現在のドラッグ操作で既にアンドゥ保存済みのタイルオフセット集合
    pub(super) drag_undo_tiles: std::collections::HashSet<usize>,
    /// ペン（パターン）ツールのドラッグ起点パリティ（(global_x + global_y) % 2）
    pub(super) drag_pattern_parity: u8,
    /// 線ツールのドラッグ起点（ブロック内ドット座標）
    pub(super) line_start_dot: Option<(usize, usize)>,
    /// スタンプ Phase1: 選択ドラッグ起点
    pub(super) stamp_sel_start: Option<(usize, usize)>,
    /// スタンプ Phase2: 確定済み (w, h, pixels[h][w]) バッファ
    pub(super) stamp_buffer: Option<(usize, usize, Vec<Vec<u8>>)>,
    /// スタンプ Phase2: 現在の貼り付け位置（ブロック左上ドット座標）
    pub(super) stamp_paste_pos: (usize, usize),
    /// スタンプ Phase2: ドラッグ中のアンカーオフセット（ドラッグ開始点 - 貼り付け位置）
    pub(super) stamp_drag_anchor: Option<(i32, i32)>,

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
    pub(super) png_import_dialog: Option<PngImportDialog>,

    /// ダークモード有効フラグ
    pub(super) dark_mode: bool,

    /// About ダイアログ表示フラグ
    pub(super) show_about: bool,

    /// タイルコピーバッファ: (n辺タイル数, n×n タイルの CHR バイト列)
    pub(super) tile_clipboard: Option<(usize, Vec<u8>)>,
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
            drag_undo_tiles: std::collections::HashSet::new(),
            drag_pattern_parity: 0,
            line_start_dot: None,
            stamp_sel_start: None,
            stamp_buffer: None,
            stamp_paste_pos: (0, 0),
            stamp_drag_anchor: None,
            file_path: None,
            raw_file_data: None,
            is_modified: false,
            address_input: "000000".into(),
            editing_palette_cell: None,
            png_import_dialog: None,
            dark_mode: true,
            show_about: false,
            tile_clipboard: None,
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
        self.handle_native_menu(ctx);

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
        self.show_menu_bar(ctx);

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
        let dot_max_w = (ctx.screen_rect().width() - 245.0 - theme::BANK_VIEW_MIN_W).max(180.0);
        let dot_resp = egui::SidePanel::right("dot_editor_panel")
            .resizable(true)
            .default_width(420.0)
            .min_width(180.0)
            .max_width(dot_max_w)
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

        // ── About ダイアログ
        if self.show_about {
            egui::Window::new("R-CHR について")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        ui.heading("R-CHR");
                        ui.add_space(4.0);
                        ui.label(concat!("Version ", env!("CARGO_PKG_VERSION")));
                        ui.add_space(8.0);
                        ui.label("NES CHR エディタ");
                        ui.add_space(12.0);
                        if ui.button("  閉じる  ").clicked() {
                            self.show_about = false;
                        }
                        ui.add_space(8.0);
                    });
                });
        }

        self.handle_keyboard(ctx);
    }
}