use eframe::egui;
use crate::chr::{bank_count, render_bank_image};
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
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .push("jp_font".to_owned());
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("jp_font".to_owned());
        ctx.set_fonts(fonts);
    }
}

pub struct RChrApp {
    rom: Option<NesRom>,
    file_name: Option<String>,
    error_msg: Option<String>,

    /// 現在表示中のバンクオフセット（バイト単位、0x1000 の倍数）
    bank_offset: usize,
    /// バンクビュー用テクスチャ
    bank_texture: Option<egui::TextureHandle>,
    /// テクスチャの再生成が必要かどうか
    texture_dirty: bool,

    /// DAT パレット（4セット × 4色）
    dat_palette: DatPalette,
    /// 現在選択中のパレットセット（0〜3）
    selected_palette_set: usize,
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
        }
    }
}

impl eframe::App for RChrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // テクスチャの再生成（ROM 読み込み・バンク移動・パレット変更後）
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

        // メニューバー
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("ファイル", |ui| {
                    if ui.button("開く…").clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                });
            });
        });

        // ステータスバー
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(name) = &self.file_name {
                    ui.label(format!("ファイル: {name}"));
                    ui.separator();
                }
                if let Some(rom) = &self.rom {
                    if !rom.chr_rom.is_empty() {
                        let current_bank = self.bank_offset / 0x1000;
                        let total_banks = bank_count(&rom.chr_rom);
                        ui.label(format!(
                            "バンク: {current_bank} / {total_banks}  |  オフセット: 0x{:06X}",
                            self.bank_offset
                        ));
                    }
                }
            });
        });

        // パレットサイドパネル（右側）
        egui::SidePanel::right("palette_panel")
            .resizable(false)
            .exact_width(160.0)
            .show(ctx, |ui| {
                self.show_palette_panel(ui);
            });

        // メインパネル（バンクビュー）
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

        self.handle_keyboard(ctx);
    }
}

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
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
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
                    self.rom = Some(rom);
                    self.texture_dirty = true;
                }
            },
        }
    }

    // ── バンクビュー ────────────────────────────────────────────────

    fn show_bank_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let can_prev = self.bank_offset >= 0x1000;
            let can_next = self.rom.as_ref().map_or(false, |r| {
                self.bank_offset + 0x1000 < r.chr_rom.len()
            });

            if ui.add_enabled(can_prev, egui::Button::new("◀ 前のバンク")).clicked() {
                self.bank_offset -= 0x1000;
                self.texture_dirty = true;
            }
            if ui.add_enabled(can_next, egui::Button::new("次のバンク ▶")).clicked() {
                self.bank_offset += 0x1000;
                self.texture_dirty = true;
            }
        });

        ui.add_space(8.0);

        if let Some(texture) = &self.bank_texture {
            let available = ui.available_size();
            let size = available.x.min(available.y).min(512.0);

            let response = ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(egui::vec2(size, size))
                    .sense(egui::Sense::click()),
            );

            self.draw_grid(ui, response.rect, size);
        }
    }

    fn draw_grid(&self, ui: &egui::Ui, rect: egui::Rect, view_size: f32) {
        let painter = ui.painter();
        let tile_px = view_size / 16.0;
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
    }

    // ── パレットパネル ──────────────────────────────────────────────

    fn show_palette_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.strong("パレット");
        ui.separator();
        ui.add_space(4.0);

        let swatch_size = egui::vec2(28.0, 28.0);
        let mut changed = false;

        for set_idx in 0..4 {
            let is_selected = self.selected_palette_set == set_idx;

            // 選択中セットは枠線でハイライト
            let frame = egui::Frame::new()
                .stroke(if is_selected {
                    egui::Stroke::new(2.0, egui::Color32::WHITE)
                } else {
                    egui::Stroke::new(1.0, egui::Color32::from_gray(80))
                })
                .inner_margin(2.0);

            let response = frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for color_idx in 0..4 {
                        let color = self.dat_palette.color32(set_idx, color_idx);
                        // 色見本を描画
                        let (rect, _) = ui.allocate_exact_size(swatch_size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, color);
                    }
                    // セット番号
                    ui.label(format!("#{set_idx}"));
                });
            });

            // 行全体をクリックでセット選択
            if response.response.interact(egui::Sense::click()).clicked() {
                self.selected_palette_set = set_idx;
                changed = true;
            }

            ui.add_space(4.0);
        }

        if changed {
            self.texture_dirty = true;
        }

        ui.separator();
        ui.add_space(4.0);
        ui.label(format!("選択中: #{}", self.selected_palette_set));

        ui.add_space(8.0);
        ui.label("キーボード:");
        ui.label("Z/X/C/V: セット 0〜3");
        ui.label("PgUp/PgDn: バンク移動");
    }

    // ── キーボード操作 ──────────────────────────────────────────────

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let Some(rom) = &self.rom else { return };
        if rom.chr_rom.is_empty() { return }
        let total = bank_count(&rom.chr_rom);

        ctx.input(|i| {
            // バンク移動
            if i.key_pressed(egui::Key::PageUp) && self.bank_offset >= 0x1000 {
                self.bank_offset -= 0x1000;
                self.texture_dirty = true;
            }
            if i.key_pressed(egui::Key::PageDown) && self.bank_offset / 0x1000 + 1 < total {
                self.bank_offset += 0x1000;
                self.texture_dirty = true;
            }

            // パレットセット選択
            let new_set = if i.key_pressed(egui::Key::Z) { Some(0) }
                else if i.key_pressed(egui::Key::X) { Some(1) }
                else if i.key_pressed(egui::Key::C) { Some(2) }
                else if i.key_pressed(egui::Key::V) { Some(3) }
                else { None };

            if let Some(set) = new_set {
                self.selected_palette_set = set;
                self.texture_dirty = true;
            }
        });
    }
}