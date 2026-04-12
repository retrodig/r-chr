use eframe::egui;
use crate::chr::{decode_block, encode_dot, render_full_image};
use crate::nes::{RomData, parse_nes};
use crate::palette::{DatPalette, MasterPalette, NES_PALETTE};

/// デフォルトで読み込むパレットファイル（バイナリに埋め込み）
const DEFAULT_PAL: &[u8] = include_bytes!("../assets/rchr.pal");
const DEFAULT_DAT: &[u8] = include_bytes!("../assets/rchr.dat");
/// NES 標準 64色パレット（リセット用）
const NES_PAL: &[u8] = include_bytes!("../assets/nes.pal");

/// 起動時に日本語フォントをセットアップする
pub fn setup_fonts(ctx: &egui::Context) {
    let font_path = "/Library/Fonts/Microsoft/MS Gothic.ttf";
    if let Ok(font_data) = std::fs::read(font_path) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "jp_font".to_owned(),
            egui::FontData::from_owned(font_data).into(),
        );
        fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().push("jp_font".to_owned());
        fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("jp_font".to_owned());
        ctx.set_fonts(fonts);
    }
}

// ── フォーカスサイズ ────────────────────────────────────────────────

/// バンクビューの選択ブロックサイズ（ピクセル単位 = タイル数 × 8）
#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusSize {
    S8   = 8,
    S16  = 16,
    S32  = 32,
    S64  = 64,
    S128 = 128,
}

impl FocusSize {
    /// 1 辺のタイル数（例: S32 → 4）
    fn tile_count(self) -> usize { self as usize / 8 }

    /// UI ラベル文字列
    fn label(self) -> &'static str {
        match self {
            Self::S8   => "8",
            Self::S16  => "16",
            Self::S32  => "32",
            Self::S64  => "64",
            Self::S128 => "128",
        }
    }
}

// ── アクション ──────────────────────────────────────────────────────

/// ドットエディタが発行するアクション（UI 描画とデータ変更を分離するため）
enum EditorAction {
    /// ドットを塗る。push_undo=true のときは変更前のタイルを undo スタックへ
    PaintDot { tile_offset: usize, px: usize, py: usize, color: u8, push_undo: bool },
    /// スポイト：ドットの色を描画色として取得
    Eyedrop { color_idx: u8 },
    /// 描画色の選択
    SelectDrawingColor { color_idx: u8 },
}

// ── アプリ状態 ─────────────────────────────────────────────────────

pub struct RChrApp {
    rom: Option<RomData>,
    file_name: Option<String>,
    error_msg: Option<String>,

    /// スクロール位置（ステータス表示用、毎フレーム更新）
    scroll_addr: usize,
    /// アドレスジャンプ時の目標アドレス（次フレームで ScrollArea に適用）
    pending_scroll_addr: Option<usize>,
    /// バンクビューの表示先頭行（矢印キーのスクロール判定用）
    scroll_top_row: usize,
    /// バンクビューの可視行数（矢印キーのスクロール判定用）
    visible_tile_rows: usize,
    bank_texture: Option<egui::TextureHandle>,
    texture_dirty: bool,

    /// バンクビューの選択ブロックサイズ
    focus_size: FocusSize,

    dat_palette: DatPalette,
    master_palette: MasterPalette,
    selected_palette_set: usize,

    /// ステータスバーに表示する一時メッセージ
    status_msg: Option<String>,

    selected_tile: Option<usize>,

    /// 現在の描画色インデックス（0〜3）
    drawing_color_idx: u8,
    /// アンドゥスタック: (タイルのバイトオフセット, 変更前の 16バイト)
    undo_stack: Vec<(usize, [u8; 16])>,

    /// 開いているファイルのフルパス（上書き保存に使用）
    file_path: Option<std::path::PathBuf>,
    /// 元のファイルバイト列（CHR 部分を書き戻すために保持）
    raw_file_data: Option<Vec<u8>>,
    /// 未保存の変更があるか
    is_modified: bool,
    /// 未保存変更ありで閉じようとしたときの確認ダイアログ表示フラグ
    show_close_dialog: bool,

    /// アドレスジャンプ入力フィールドの内容（16進数文字列）
    address_input: String,

    /// パレットピッカーで編集中のセル (set_idx, color_idx)
    editing_palette_cell: Option<(usize, usize)>,
}

impl Default for RChrApp {
    fn default() -> Self {
        Self {
            rom: None,
            file_name: None,
            error_msg: None,
            scroll_addr: 0,
            pending_scroll_addr: None,
            scroll_top_row: 0,
            visible_tile_rows: 0,
            bank_texture: None,
            texture_dirty: false,
            focus_size: FocusSize::S8,
            dat_palette: DatPalette::from_dat_bytes(DEFAULT_DAT).unwrap_or_default(),
            master_palette: MasterPalette::from_pal_bytes(DEFAULT_PAL).unwrap_or_default(),
            selected_palette_set: 0,
            status_msg: None,
            selected_tile: None,
            drawing_color_idx: 1,
            undo_stack: Vec::new(),
            file_path: None,
            raw_file_data: None,
            is_modified: false,
            show_close_dialog: false,
            address_input: "000000".into(),
            editing_palette_cell: None,
        }
    }
}

// ── メインループ ───────────────────────────────────────────────────

impl eframe::App for RChrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                // 閉じるをキャンセルしてダイアログを出す
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_close_dialog = true;
            }
        }

        // ── 未保存変更の確認ダイアログ
        if self.show_close_dialog {
            self.show_close_confirm_dialog(ctx);
        }

        // ── タイトルバー更新（未保存変更を * で表示）
        let title = if self.is_modified {
            format!(
                "R-CHR  *{}",
                self.file_name.as_deref().unwrap_or("")
            )
        } else {
            format!(
                "R-CHR  {}",
                self.file_name.as_deref().unwrap_or("")
            )
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        // ── メニューバー
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("ファイル", |ui| {
                    if ui.button("開く…  ⌘O").clicked() {
                        self.open_file();
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

        // ── ステータスバー
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(name) = &self.file_name {
                    ui.label(format!("ファイル: {name}"));
                    ui.separator();
                }
                if let Some(rom) = &self.rom {
                    if !rom.chr_data().is_empty() {
                        let total_tiles = rom.chr_data().len() / 16;
                        ui.label(format!("0x{:06X}  ({} タイル)", self.scroll_addr, total_tiles));
                        if let Some(idx) = self.selected_tile {
                            ui.separator();
                            let addr = idx * 16;
                            ui.label(format!("タイル: {} (0x{:06X})", idx, addr));
                        }
                    }
                }
                if let Some(msg) = &self.status_msg {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(100, 220, 100)));
                    });
                }
            });
        });

        // ── 右パネル（ドットエディタ + パレット）
        // show_dot_editor は &self でアクションを返す → 後で apply する
        let mut editor_action: Option<EditorAction> = None;

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(280.0)
            .min_width(200.0)
            .show(ctx, |ui| {
                let panel_h = ui.available_height();
                egui::TopBottomPanel::bottom("palette_sub")
                    .resizable(false)
                    .exact_height(panel_h * 0.35)
                    .frame(egui::Frame::new())
                    .show_inside(ui, |ui| {
                        self.show_palette_panel(ui);
                    });
                egui::CentralPanel::default()
                    .frame(egui::Frame::new())
                    .show_inside(ui, |ui| {
                        editor_action = self.show_dot_editor(ui);
                    });
            });

        // ── バンクビュー（メイン）
        egui::CentralPanel::default().show(ctx, |ui| {
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

        // ── NES 64色パレットピッカー
        if self.editing_palette_cell.is_some() {
            self.show_nes_palette_picker(ctx);
        }

        self.handle_keyboard(ctx);
    }
}

// ── ファイル操作 ───────────────────────────────────────────────────

impl RChrApp {
    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NES / BIN", &["nes", "bin"])
            .add_filter("すべてのファイル", &["*"])
            .pick_file()
        else {
            return;
        };

        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        let data = match std::fs::read(&path) {
            Err(e) => {
                self.error_msg = Some(format!("読み込み失敗: {e}"));
                return;
            }
            Ok(d) => d,
        };

        let rom_data = if ext == "bin" {
            // 生 CHR バイナリ：ヘッダなしでそのまま使用
            if data.is_empty() {
                self.error_msg = Some("BIN ファイルが空です".into());
                return;
            }
            self.raw_file_data = None;
            RomData::Bin(data)
        } else {
            // NES (iNES) フォーマット
            match parse_nes(&data) {
                Err(e) => {
                    self.error_msg = Some(e.to_string());
                    return;
                }
                Ok(rom) => {
                    self.raw_file_data = Some(data);
                    RomData::Nes(rom)
                }
            }
        };

        self.error_msg = None;
        self.file_name = Some(file_name);
        self.scroll_addr = 0;
        self.pending_scroll_addr = Some(0);
        self.selected_tile = None;
        self.undo_stack.clear();
        self.file_path = Some(path);
        self.is_modified = false;
        self.rom = Some(rom_data);
        self.texture_dirty = true;
    }

    // ── バンクビュー ──────────────────────────────────────────────

    fn show_bank_view(&mut self, ui: &mut egui::Ui) {
        // 利用可能幅から表示スケールを計算（整数スケール）
        let available_w = ui.available_width();
        let scale = (available_w / 128.0).floor().max(1.0);
        let tile_px = 8.0 * scale; // 1 タイルの表示サイズ（px）

        // ── ツールバー（アドレスジャンプ + フォーカスサイズ）
        ui.horizontal(|ui| {
            // アドレスジャンプ入力
            ui.label("アドレス:");
            let addr_resp = ui.add(
                egui::TextEdit::singleline(&mut self.address_input)
                    .desired_width(70.0)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("001000"),
            );

            let enter_pressed = addr_resp.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let button_clicked = ui.button("移動").clicked();

            if enter_pressed || button_clicked {
                self.jump_to_address();
                ui.ctx().memory_mut(|m| m.surrender_focus(addr_resp.id));
            } else if !addr_resp.has_focus() {
                self.address_input = match self.selected_tile {
                    Some(idx) => format!("{:06X}", idx * 16),
                    None      => format!("{:06X}", self.scroll_addr),
                };
            }

            ui.separator();

            // フォーカスサイズ切り替えボタン
            ui.label("サイズ:");
            for &fs in &[FocusSize::S8, FocusSize::S16, FocusSize::S32, FocusSize::S64, FocusSize::S128] {
                if ui.selectable_label(self.focus_size == fs, fs.label()).clicked() {
                    self.focus_size = fs;
                    // 選択タイルの起点は変えない（8×8 単位で自由に設定できる）
                }
            }
        });
        ui.add_space(4.0);

        // ── スクロールビューの準備（self からコピーが必要な値を事前抽出）
        let total_tiles = self.rom.as_ref().map_or(0, |r| r.chr_data().len() / 16);
        if total_tiles == 0 { return; }
        let total_rows = (total_tiles + 15) / 16;
        let texture_id = match &self.bank_texture {
            Some(t) => t.id(),
            None => return,
        };
        let n = self.focus_size.tile_count();
        let selected_tile_snap = self.selected_tile;
        let display_w = 128.0 * scale;
        let display_h = total_rows as f32 * tile_px;

        // ── ScrollArea のスクロール位置をアドレスジャンプで制御
        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("bank_scroll")
            .auto_shrink([false, false]);

        if let Some(addr) = self.pending_scroll_addr.take() {
            let row = addr / 0x100;
            scroll_area = scroll_area.vertical_scroll_offset(row as f32 * tile_px);
        }

        let scroll_out = scroll_area.show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(display_w, display_h),
                egui::Sense::click(),
            );

            // テクスチャ描画
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(texture_id, rect, uv, egui::Color32::WHITE);

            let painter = ui.painter();

            // マイナーグリッド（タイル単位）
            let minor = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25);
            for col in 1..16 {
                let x = rect.left() + tile_px * col as f32;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(0.5, minor),
                );
            }
            for row in 1..total_rows {
                let y = rect.top() + tile_px * row as f32;
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(0.5, minor),
                );
            }

            // メジャーグリッド（フォーカスブロック単位）
            if n > 1 {
                let major = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 70);
                for col in (0..=16).step_by(n) {
                    let x = rect.left() + tile_px * col as f32;
                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        egui::Stroke::new(1.0, major),
                    );
                }
                for row in (0..=total_rows).step_by(n) {
                    let y = rect.top() + tile_px * row as f32;
                    painter.line_segment(
                        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                        egui::Stroke::new(1.0, major),
                    );
                }
            }

            // タイルクリック検出（フォーカスグリッドに snap）
            let new_tile = if response.clicked() {
                response.interact_pointer_pos().and_then(|pos| {
                    let rel_x = pos.x - rect.left();
                    let rel_y = pos.y - rect.top();
                    if rel_x < 0.0 || rel_y < 0.0 { return None; }
                    let col = (rel_x / tile_px) as usize;
                    let row = (rel_y / tile_px) as usize;
                    if col >= 16 { return None; }
                    let global_tile = row * 16 + col;
                    (global_tile < total_tiles).then_some(global_tile)
                })
            } else {
                None
            };

            // 選択ブロックのハイライト
            if let Some(tile_idx) = selected_tile_snap {
                let t_row = tile_idx / 16;
                let t_col = tile_idx % 16;
                let bx = rect.left() + t_col as f32 * tile_px;
                let by_ = rect.top()  + t_row as f32 * tile_px;
                let bs  = tile_px * n as f32;
                let hl  = egui::Rect::from_min_size(egui::pos2(bx, by_), egui::vec2(bs, bs));
                painter.rect_stroke(
                    hl, 0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 80, 80)),
                    egui::StrokeKind::Outside,
                );
            }

            new_tile
        });

        // スクロール結果を self に反映
        if let Some(tile) = scroll_out.inner {
            self.selected_tile = Some(tile);
        }
        let scroll_y = scroll_out.state.offset.y;
        self.scroll_addr = (scroll_y / tile_px) as usize * 0x100;
        // 矢印キーのスクロール判定用にビューポート情報を保存
        self.scroll_top_row = (scroll_y / tile_px) as usize;
        self.visible_tile_rows = (scroll_out.inner_rect.height() / tile_px).floor() as usize;
    }

    // ── ドットエディタ ────────────────────────────────────────────
    // &self で描画意図を返す（データ変更は apply_action で行う）

    fn show_dot_editor(&self, ui: &mut egui::Ui) -> Option<EditorAction> {
        ui.strong("ドットエディタ");
        ui.separator();

        // ── 描画色セレクタ
        let mut color_action: Option<EditorAction> = None;
        ui.horizontal(|ui| {
            ui.label("描画色:");
            ui.spacing_mut().item_spacing.x = 3.0;
            for c in 0..4u8 {
                let fill = self.dat_palette.color32(self.selected_palette_set, c as usize, &self.master_palette);
                let is_active = self.drawing_color_idx == c;
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                ui.painter().rect_filled(rect, 2.0, fill);
                ui.painter().rect_stroke(
                    rect, 2.0,
                    egui::Stroke::new(if is_active { 2.5 } else { 1.0 },
                        if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(80) }),
                    egui::StrokeKind::Outside,
                );
                if resp.clicked() {
                    color_action = Some(EditorAction::SelectDrawingColor { color_idx: c });
                }
            }
        });
        if color_action.is_some() {
            return color_action;
        }

        ui.add_space(4.0);

        // ── タイルが未選択
        let Some(top_left_tile) = self.selected_tile else {
            ui.label("← タイルをクリックして選択");
            return None;
        };
        let Some(rom) = &self.rom else { return None };

        let n = self.focus_size.tile_count(); // ブロック 1 辺のタイル数
        let block_px = n * 8;                // ブロック 1 辺のドット数

        // フォーカスブロック全体をデコード（N×N タイル → block_px × block_px ドット）
        let block = decode_block(rom.chr_data(), top_left_tile, 16, n);

        // ── ドットキャンバス
        let available = ui.available_size();
        let dot_size = (available.x.min(available.y) / block_px as f32)
            .floor()
            .max(1.0);
        let canvas = dot_size * block_px as f32;

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(canvas, canvas),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter();

        for py in 0..block_px {
            for px in 0..block_px {
                let fill = self.dat_palette.color32(
                    self.selected_palette_set,
                    block[py][px] as usize,
                    &self.master_palette,
                );
                let dot_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + px as f32 * dot_size, rect.top() + py as f32 * dot_size),
                    egui::vec2(dot_size, dot_size),
                );
                painter.rect_filled(dot_rect, 0.0, fill);
                // ドットグリッド線（block_px が小さいときのみ描画）
                if dot_size >= 4.0 {
                    painter.rect_stroke(
                        dot_rect, 0.0,
                        egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }

        // タイル境界線（フォーカスが 2 タイル以上のとき）
        if n > 1 && dot_size >= 2.0 {
            let tile_line_color = egui::Color32::from_rgba_unmultiplied(200, 200, 200, 80);
            let tile_step = dot_size * 8.0;
            for t in 1..n {
                let x = rect.left() + tile_step * t as f32;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(1.0, tile_line_color),
                );
                let y = rect.top() + tile_step * t as f32;
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(1.0, tile_line_color),
                );
            }
        }

        // ── クリック / ドラッグ検出
        let Some(pos) = response.interact_pointer_pos() else { return None };
        let rel_x = pos.x - rect.left();
        let rel_y = pos.y - rect.top();
        if rel_x < 0.0 || rel_y < 0.0 { return None }

        let px = (rel_x / dot_size) as usize;
        let py = (rel_y / dot_size) as usize;
        if px >= block_px || py >= block_px { return None }

        // 右クリック → スポイト
        if response.secondary_clicked() {
            return Some(EditorAction::Eyedrop { color_idx: block[py][px] });
        }

        // クリック / ドラッグしたドットが属するタイルのオフセットを計算
        let block_col = px / 8;
        let block_row = py / 8;
        let top_col = top_left_tile % 16;
        let top_row = top_left_tile / 16;
        let tile_global = (top_row + block_row) * 16 + (top_col + block_col);
        let tile_offset = tile_global * 16;
        let dot_px = px % 8;
        let dot_py = py % 8;

        if tile_offset + 16 > rom.chr_data().len() { return None; }

        // 左クリック / 左ドラッグ → 描画
        let drag_started = response.drag_started_by(egui::PointerButton::Primary);
        let dragging     = response.dragged_by(egui::PointerButton::Primary);
        let clicked      = response.clicked_by(egui::PointerButton::Primary);

        if drag_started || dragging || clicked {
            let push_undo = drag_started || clicked;
            return Some(EditorAction::PaintDot {
                tile_offset, px: dot_px, py: dot_py,
                color: self.drawing_color_idx,
                push_undo,
            });
        }

        None
    }

    // ── アクション適用 ────────────────────────────────────────────

    fn apply_action(&mut self, action: EditorAction) {
        match action {
            EditorAction::SelectDrawingColor { color_idx } => {
                self.drawing_color_idx = color_idx;
            }
            EditorAction::Eyedrop { color_idx } => {
                self.drawing_color_idx = color_idx;
            }
            EditorAction::PaintDot { tile_offset, px, py, color, push_undo } => {
                let Some(rom) = &mut self.rom else { return };
                if tile_offset + 16 > rom.chr_data().len() { return }

                if push_undo {
                    let saved: [u8; 16] = rom.chr_data()[tile_offset..tile_offset + 16]
                        .try_into()
                        .unwrap();
                    if self.undo_stack.len() >= 100 {
                        self.undo_stack.remove(0);
                    }
                    self.undo_stack.push((tile_offset, saved));
                }

                encode_dot(&mut rom.chr_data_mut()[tile_offset..tile_offset + 16], px, py, color);
                self.is_modified = true;
                self.texture_dirty = true;
            }
        }
    }

    /// アドレス入力フィールドの内容をパースして該当タイルへスクロール・フォーカス
    fn jump_to_address(&mut self) {
        let raw = self.address_input.trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        if let Ok(addr) = usize::from_str_radix(raw, 16) {
            let total_tiles = self.rom.as_ref().map_or(0, |r| r.chr_data().len() / 16);
            if total_tiles > 0 {
                let tile_idx = (addr / 16).min(total_tiles.saturating_sub(1));
                let n = self.focus_size.tile_count();
                let snap_col = (tile_idx % 16 / n) * n;
                let snap_row = (tile_idx / 16 / n) * n;
                let snapped  = snap_row * 16 + snap_col;

                self.selected_tile      = Some(snapped);
                self.pending_scroll_addr = Some(snap_row * 0x100);
                self.address_input       = format!("{:06X}", snapped * 16);
                return;
            }
        }
        // パース失敗・範囲外の場合は現在値に戻す
        self.address_input = match self.selected_tile {
            Some(idx) => format!("{:06X}", idx * 16),
            None      => format!("{:06X}", self.scroll_addr),
        };
    }

    fn do_undo(&mut self) {
        let Some((offset, saved)) = self.undo_stack.pop() else { return };
        let Some(rom) = &mut self.rom else { return };
        if offset + 16 <= rom.chr_data().len() {
            rom.chr_data_mut()[offset..offset + 16].copy_from_slice(&saved);
            self.texture_dirty = true;
        }
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

    // ── 未保存変更の確認ダイアログ ────────────────────────────────

    fn show_close_confirm_dialog(&mut self, ctx: &egui::Context) {
        let modal = egui::Modal::new("close_confirm".into());
        modal.show(ctx, |ui| {
            ui.set_width(320.0);
            ui.heading("未保存の変更があります");
            ui.add_space(8.0);
            ui.label(format!(
                "「{}」への変更が保存されていません。",
                self.file_name.as_deref().unwrap_or("(無題)")
            ));
            ui.label("終了する前に保存しますか？");
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                // 保存して閉じる
                if ui.button("💾 保存して閉じる").clicked() {
                    let result = if self.file_path.is_some() {
                        self.save_file()
                    } else {
                        self.save_file_as()
                    };
                    match result {
                        Err(e) => {
                            self.error_msg = Some(e);
                            self.show_close_dialog = false;
                        }
                        Ok(()) if !self.is_modified => {
                            // 保存成功 → 閉じる
                            self.show_close_dialog = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        Ok(()) => {
                            // save_file_as でキャンセルされた (is_modified のまま) → 何もしない
                        }
                    }
                }

                // 保存せず閉じる
                if ui.button("🗑 保存せず閉じる").clicked() {
                    self.show_close_dialog = false;
                    // Close 後に close_requested が再発火したとき CancelClose しないよう先にクリア
                    self.is_modified = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                // キャンセル
                if ui.button("キャンセル").clicked() {
                    self.show_close_dialog = false;
                }
            });
        });
    }

    // ── パレットパネル ────────────────────────────────────────────

    fn show_palette_panel(&mut self, ui: &mut egui::Ui) {
        ui.strong("パレット");
        ui.separator();
        let swatch_size = egui::vec2(24.0, 24.0);
        let mut set_changed = false;
        let mut open_picker: Option<(usize, usize)> = None;

        for set_idx in 0..4 {
            let is_selected = self.selected_palette_set == set_idx;
            let frame = egui::Frame::new()
                .stroke(if is_selected {
                    egui::Stroke::new(2.0, egui::Color32::WHITE)
                } else {
                    egui::Stroke::new(1.0, egui::Color32::from_gray(80))
                })
                .inner_margin(2.0);

            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for color_idx in 0..4 {
                        let color = self.dat_palette.color32(set_idx, color_idx, &self.master_palette);
                        let (rect, resp) = ui.allocate_exact_size(swatch_size, egui::Sense::click());
                        ui.painter().rect_filled(rect, 0.0, color);
                        // 編集中セルの枠を強調
                        if self.editing_palette_cell == Some((set_idx, color_idx)) {
                            ui.painter().rect_stroke(
                                rect, 0.0,
                                egui::Stroke::new(2.0, egui::Color32::YELLOW),
                                egui::StrokeKind::Outside,
                            );
                        }
                        let nes_idx = self.dat_palette.sets[set_idx][color_idx];
                        let clicked = resp.clicked();
                        resp.on_hover_text(format!("NES 0x{nes_idx:02X}  クリックで変更"));
                        if clicked {
                            open_picker = Some((set_idx, color_idx));
                        }
                    }
                    // ラベル部分クリックでセット選択
                    let label_resp = ui.label(format!("#{set_idx}"));
                    if label_resp.interact(egui::Sense::click()).clicked() {
                        self.selected_palette_set = set_idx;
                        set_changed = true;
                    }
                });
            });
            ui.add_space(2.0);
        }

        if let Some(cell) = open_picker {
            // 対応するパレットセットも選択状態にする
            self.selected_palette_set = cell.0;
            self.editing_palette_cell = Some(cell);
            set_changed = true;
        }
        if set_changed {
            self.texture_dirty = true;
        }
    }

    // ── NES 64色パレットピッカー ──────────────────────────────────

    fn show_nes_palette_picker(&mut self, ctx: &egui::Context) {
        let Some((set_idx, color_idx)) = self.editing_palette_cell else { return };

        // Esc で閉じる
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editing_palette_cell = None;
            return;
        }

        let mut open = true;
        egui::Window::new("NES パレット")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(format!("セット #{set_idx}  色 {color_idx} を変更"));
                ui.separator();

                let cell_size = 20.0;
                let mut selected_nes_idx: Option<u8> = None;

                // 8列 × 8行 で 64色を表示
                for row in 0..8usize {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(1.0, 1.0);
                        for col in 0..8usize {
                            let nes_idx = (row * 8 + col) as u8;
                            let [r, g, b] = NES_PALETTE[nes_idx as usize];
                            let color = egui::Color32::from_rgb(r, g, b);
                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(cell_size, cell_size),
                                egui::Sense::click(),
                            );
                            ui.painter().rect_filled(rect, 1.0, color);

                            // 現在選択中インデックスをハイライト
                            let current_idx = self.dat_palette.sets[set_idx][color_idx];
                            if current_idx == nes_idx {
                                ui.painter().rect_stroke(
                                    rect, 1.0,
                                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                                    egui::StrokeKind::Outside,
                                );
                            }

                            let clicked = resp.clicked();
                            resp.on_hover_text(format!("0x{nes_idx:02X}"));
                            if clicked {
                                selected_nes_idx = Some(nes_idx);
                            }
                        }
                    });
                }

                ui.add_space(6.0);
                if ui.button("閉じる").clicked() {
                    self.editing_palette_cell = None;
                }

                if let Some(idx) = selected_nes_idx {
                    self.dat_palette.sets[set_idx][color_idx] = idx;
                    self.texture_dirty = true;
                    self.editing_palette_cell = None;
                }
            });

        if !open {
            self.editing_palette_cell = None;
        }
    }

    // ── キーボード操作 ────────────────────────────────────────────

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
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