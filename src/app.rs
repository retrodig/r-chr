use eframe::egui;
use crate::chr::{bank_count, decode_tile, encode_dot, render_bank_image};
use crate::nes::{NesRom, parse_nes};
use crate::palette::DatPalette;

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
    rom: Option<NesRom>,
    file_name: Option<String>,
    error_msg: Option<String>,

    bank_offset: usize,
    bank_texture: Option<egui::TextureHandle>,
    texture_dirty: bool,

    dat_palette: DatPalette,
    selected_palette_set: usize,

    selected_tile: Option<usize>,

    /// 現在の描画色インデックス（0〜3）
    drawing_color_idx: u8,
    /// アンドゥスタック: (タイルのバイトオフセット, 変更前の 16バイト)
    undo_stack: Vec<(usize, [u8; 16])>,
}

impl Default for RChrApp {
    fn default() -> Self {
        Self {
            rom: None,
            file_name: None,
            error_msg: None,
            bank_offset: 0,
            bank_texture: None,
            texture_dirty: false,
            dat_palette: DatPalette::default(),
            selected_palette_set: 0,
            selected_tile: None,
            drawing_color_idx: 1,
            undo_stack: Vec::new(),
        }
    }
}

// ── メインループ ───────────────────────────────────────────────────

impl eframe::App for RChrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.texture_dirty {
            if let Some(rom) = &self.rom {
                if !rom.chr_rom.is_empty() {
                    let image = render_bank_image(
                        &rom.chr_rom,
                        self.bank_offset,
                        &self.dat_palette,
                        self.selected_palette_set,
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

        // ── メニューバー
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("ファイル", |ui| {
                    if ui.button("開く…").clicked() {
                        self.open_file();
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
                    if !rom.chr_rom.is_empty() {
                        let cur = self.bank_offset / 0x1000;
                        let tot = bank_count(&rom.chr_rom);
                        ui.label(format!("バンク: {cur} / {tot}  |  0x{:06X}", self.bank_offset));
                        if let Some(idx) = self.selected_tile {
                            ui.separator();
                            let addr = self.bank_offset + idx * 16;
                            ui.label(format!("タイル: {idx} (0x{addr:06X})"));
                        }
                    }
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
                        ui.label("ファイルメニューから NES ROM を開いてください");
                    });
                }
                Some(rom) => {
                    if rom.chr_rom.is_empty() {
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

        self.handle_keyboard(ctx);
    }
}

// ── ファイル操作 ───────────────────────────────────────────────────

impl RChrApp {
    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NES ROM", &["nes"])
            .add_filter("すべてのファイル", &["*"])
            .pick_file()
        else {
            return;
        };

        self.file_name = Some(
            path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        );

        match std::fs::read(&path) {
            Err(e) => {
                self.error_msg = Some(format!("読み込み失敗: {e}"));
                self.rom = None;
            }
            Ok(data) => match parse_nes(&data) {
                Err(e) => {
                    self.error_msg = Some(e.to_string());
                    self.rom = None;
                }
                Ok(rom) => {
                    self.error_msg = None;
                    self.bank_offset = 0;
                    self.selected_tile = None;
                    self.undo_stack.clear();
                    self.rom = Some(rom);
                    self.texture_dirty = true;
                }
            },
        }
    }

    // ── バンクビュー ──────────────────────────────────────────────

    fn show_bank_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let can_prev = self.bank_offset >= 0x1000;
            let can_next = self.rom.as_ref().map_or(false, |r| {
                self.bank_offset + 0x1000 < r.chr_rom.len()
            });
            if ui.add_enabled(can_prev, egui::Button::new("◀ 前")).clicked() {
                self.bank_offset -= 0x1000;
                self.texture_dirty = true;
            }
            if ui.add_enabled(can_next, egui::Button::new("次 ▶")).clicked() {
                self.bank_offset += 0x1000;
                self.texture_dirty = true;
            }
        });
        ui.add_space(4.0);

        if let Some(texture) = &self.bank_texture {
            let available = ui.available_size();
            let size = available.x.min(available.y).min(512.0);

            let response = ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(egui::vec2(size, size))
                    .sense(egui::Sense::click()),
            );
            let rect = response.rect;
            let tile_px = size / 16.0;

            // タイルクリック検出
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let tx = ((pos.x - rect.left()) / tile_px) as usize;
                    let ty = ((pos.y - rect.top()) / tile_px) as usize;
                    if tx < 16 && ty < 16 {
                        self.selected_tile = Some(ty * 16 + tx);
                    }
                }
            }

            let painter = ui.painter();

            // グリッド線
            let grid_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 40);
            for i in 1..16 {
                let x = rect.left() + tile_px * i as f32;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(1.0, grid_color),
                );
                let y = rect.top() + tile_px * i as f32;
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(1.0, grid_color),
                );
            }

            // 選択タイルのハイライト
            if let Some(idx) = self.selected_tile {
                let tx = (idx % 16) as f32;
                let ty = (idx / 16) as f32;
                let hl = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + tx * tile_px, rect.top() + ty * tile_px),
                    egui::vec2(tile_px, tile_px),
                );
                painter.rect_stroke(
                    hl, 0.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 80, 80)),
                    egui::StrokeKind::Outside,
                );
            }
        }
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
                let fill = self.dat_palette.color32(self.selected_palette_set, c as usize);
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
        let Some(tile_idx) = self.selected_tile else {
            ui.label("← タイルをクリックして選択");
            return None;
        };
        let Some(rom) = &self.rom else { return None };
        let tile_offset = self.bank_offset + tile_idx * 16;
        if tile_offset + 16 > rom.chr_rom.len() { return None }

        let tile = decode_tile(&rom.chr_rom[tile_offset..tile_offset + 16]);

        // ── ドットキャンバス
        let available = ui.available_size();
        let dot_size = (available.x.min(available.y) / 8.0).floor().max(8.0);
        let canvas = dot_size * 8.0;

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(canvas, canvas),
            egui::Sense::click_and_drag(),
        );
        let painter = ui.painter();

        for py in 0..8usize {
            for px in 0..8usize {
                let fill = self.dat_palette.color32(self.selected_palette_set, tile[py][px] as usize);
                let dot_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + px as f32 * dot_size, rect.top() + py as f32 * dot_size),
                    egui::vec2(dot_size, dot_size),
                );
                painter.rect_filled(dot_rect, 0.0, fill);
                painter.rect_stroke(
                    dot_rect, 0.0,
                    egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
                    egui::StrokeKind::Inside,
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
        if px >= 8 || py >= 8 { return None }

        // 右クリック → スポイト
        if response.secondary_clicked() {
            return Some(EditorAction::Eyedrop { color_idx: tile[py][px] });
        }

        // 左クリック / 左ドラッグ → 描画
        let drag_started = response.drag_started_by(egui::PointerButton::Primary);
        let dragging     = response.dragged_by(egui::PointerButton::Primary);
        let clicked      = response.clicked_by(egui::PointerButton::Primary);

        if drag_started || dragging || clicked {
            // ドラッグ開始またはシングルクリックの初回のみ undo を保存
            let push_undo = drag_started || clicked;
            return Some(EditorAction::PaintDot {
                tile_offset, px, py,
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
                if tile_offset + 16 > rom.chr_rom.len() { return }

                if push_undo {
                    let saved: [u8; 16] = rom.chr_rom[tile_offset..tile_offset + 16]
                        .try_into()
                        .unwrap();
                    if self.undo_stack.len() >= 100 {
                        self.undo_stack.remove(0);
                    }
                    self.undo_stack.push((tile_offset, saved));
                }

                encode_dot(&mut rom.chr_rom[tile_offset..tile_offset + 16], px, py, color);
                self.texture_dirty = true;
            }
        }
    }

    fn do_undo(&mut self) {
        let Some((offset, saved)) = self.undo_stack.pop() else { return };
        let Some(rom) = &mut self.rom else { return };
        if offset + 16 <= rom.chr_rom.len() {
            rom.chr_rom[offset..offset + 16].copy_from_slice(&saved);
            self.texture_dirty = true;
        }
    }

    // ── パレットパネル ────────────────────────────────────────────

    fn show_palette_panel(&mut self, ui: &mut egui::Ui) {
        ui.strong("パレット");
        ui.separator();
        let swatch_size = egui::vec2(24.0, 24.0);
        let mut changed = false;

        for set_idx in 0..4 {
            let is_selected = self.selected_palette_set == set_idx;
            let frame = egui::Frame::new()
                .stroke(if is_selected {
                    egui::Stroke::new(2.0, egui::Color32::WHITE)
                } else {
                    egui::Stroke::new(1.0, egui::Color32::from_gray(80))
                })
                .inner_margin(2.0);

            let resp = frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for color_idx in 0..4 {
                        let color = self.dat_palette.color32(set_idx, color_idx);
                        let (rect, _) = ui.allocate_exact_size(swatch_size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, color);
                    }
                    ui.label(format!("#{set_idx}"));
                });
            });

            if resp.response.interact(egui::Sense::click()).clicked() {
                self.selected_palette_set = set_idx;
                changed = true;
            }
            ui.add_space(2.0);
        }
        if changed {
            self.texture_dirty = true;
        }
    }

    // ── キーボード操作 ────────────────────────────────────────────

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let chr_empty = self.rom.as_ref().map_or(true, |r| r.chr_rom.is_empty());
        if chr_empty { return }
        let total = self.rom.as_ref().map_or(0, |r| bank_count(&r.chr_rom));

        let mut bank_delta: i32 = 0;
        let mut new_palette_set: Option<usize> = None;
        let mut do_undo = false;

        ctx.input(|i| {
            if i.key_pressed(egui::Key::PageUp)   { bank_delta -= 1; }
            if i.key_pressed(egui::Key::PageDown)  { bank_delta += 1; }

            let undo_mod = i.modifiers.ctrl || i.modifiers.mac_cmd;
            if undo_mod && i.key_pressed(egui::Key::Z) {
                do_undo = true;
            } else if i.key_pressed(egui::Key::Z) {
                new_palette_set = Some(0);
            }
            if i.key_pressed(egui::Key::X) { new_palette_set = Some(1); }
            if i.key_pressed(egui::Key::C) { new_palette_set = Some(2); }
            if i.key_pressed(egui::Key::V) { new_palette_set = Some(3); }
        });

        if bank_delta < 0 && self.bank_offset >= 0x1000 {
            self.bank_offset -= 0x1000;
            self.texture_dirty = true;
        }
        if bank_delta > 0 && self.bank_offset / 0x1000 + 1 < total {
            self.bank_offset += 0x1000;
            self.texture_dirty = true;
        }
        if let Some(set) = new_palette_set {
            self.selected_palette_set = set;
            self.texture_dirty = true;
        }
        if do_undo {
            self.do_undo();
        }
    }
}