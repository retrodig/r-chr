use eframe::egui;
use crate::chr::{bank_count, render_bank_image};
use crate::nes::{NesRom, parse_nes};

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
        }
    }
}

impl eframe::App for RChrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // テクスチャの再生成（ROM 読み込み・バンク移動後）
        if self.texture_dirty {
            if let Some(rom) = &self.rom {
                if !rom.chr_rom.is_empty() {
                    let image = render_bank_image(&rom.chr_rom, self.bank_offset);
                    self.bank_texture = Some(ctx.load_texture(
                        "bank_view",
                        image,
                        egui::TextureOptions::NEAREST, // ドット絵なのでニアレストネイバー
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

        // メインパネル
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

        // キーボードでバンク移動
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

    fn show_bank_view(&mut self, ui: &mut egui::Ui) {
        // バンク移動ボタン
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

        // バンクビュー（テクスチャ表示）
        if let Some(texture) = &self.bank_texture {
            let available = ui.available_size();
            // アスペクト比 1:1 を保ちつつ、利用可能な領域に収める
            let size = available.x.min(available.y).min(512.0);

            let response = ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(egui::vec2(size, size))
                    .sense(egui::Sense::click()),
            );

            // グリッド線をタイル境界（8×8）に描画
            self.draw_grid(ui, response.rect, size);
        }
    }

    /// バンクビュー上にタイルグリッド線を描画する
    fn draw_grid(&self, ui: &egui::Ui, rect: egui::Rect, view_size: f32) {
        let painter = ui.painter();
        let tile_px = view_size / 16.0; // 1タイルあたりの表示ピクセル数

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

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let Some(rom) = &self.rom else { return };
        if rom.chr_rom.is_empty() { return };
        let total = bank_count(&rom.chr_rom);

        ctx.input(|i| {
            if i.key_pressed(egui::Key::PageUp) && self.bank_offset >= 0x1000 {
                self.bank_offset -= 0x1000;
                self.texture_dirty = true;
            }
            if i.key_pressed(egui::Key::PageDown) && self.bank_offset / 0x1000 + 1 < total {
                self.bank_offset += 0x1000;
                self.texture_dirty = true;
            }
        });
    }
}