//! 右情報パネル（パレット・描画色・タイル情報）
use eframe::egui;
use crate::model::palette::NES_PALETTE;
use super::app::RChrApp;
use super::dot_editor::EditorAction;
use super::theme;

impl RChrApp {
    // ── 右情報パネル ─────────────────────────────────────────────

    pub(super) fn show_info_panel(&mut self, ui: &mut egui::Ui) {
        ui.visuals_mut().widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, theme::COL_SEPARATOR);

        // アドレス・タイル情報
        if let Some(rom) = &self.rom {
            if !rom.chr_data().is_empty() {
                let total_tiles = rom.chr_data().len() / 16;
                ui.label(format!("0x{:06X}  ({} タイル)", self.scroll_addr, total_tiles));
                ui.separator();
            }
        }

        ui.add_space(6.0);
        if let Some(idx) = self.selected_tile {
            ui.label(
                theme::rich_label("タイル"),
            );
            ui.add_space(2.0);
            ui.label(format!("{}  (0x{:06X})", idx, idx * 16));
        }
        ui.add_space(6.0);
        ui.separator();

        // 描画色セレクタ
        ui.add_space(4.0);
        ui.label(
            theme::rich_label("描画色"),
        );
        ui.add_space(10.0);

        let mut color_action: Option<EditorAction> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for c in 0..4u8 {
                let fill = self.dat_palette.color32(self.selected_palette_set, c as usize, &self.master_palette);
                let is_active = self.drawing_color_idx == c;
                let (rect, resp) = ui.allocate_exact_size(
                    egui::vec2(theme::SWATCH_PX, theme::SWATCH_PX),
                    egui::Sense::click(),
                );
                ui.painter().rect_filled(rect, theme::CR_SM, fill);
                ui.painter().rect_stroke(
                    rect, theme::CR_SM,
                    egui::Stroke::new(
                        if is_active { 2.5 } else { 1.0 },
                        if is_active { egui::Color32::WHITE } else { theme::COL_SWATCH_BORDER },
                    ),
                    egui::StrokeKind::Outside,
                );
                if resp.clicked() {
                    color_action = Some(EditorAction::SelectDrawingColor { color_idx: c });
                }
            }
        });
        if let Some(action) = color_action {
            self.apply_action(action);
        }

        ui.add_space(6.0);
        ui.separator();

        // パレットパネル
        ui.add_space(4.0);
        self.show_palette_panel(ui);

        ui.add_space(4.0);
        ui.separator();

        // NES パレット（常に表示）
        ui.add_space(4.0);
        ui.label(
            theme::rich_label("NES パレット"),
        );

        if let Some((set_idx, color_idx)) = self.editing_palette_cell {
            ui.label(format!("セット #{set_idx}  色 {color_idx} を変更"));
        } else {
            ui.colored_label(egui::Color32::from_gray(140), "パレットの色をクリックして変更");
        }
        ui.add_space(10.0);

        let cell_size = 26.0;
        let mut selected_nes_idx: Option<u8> = None;
        for row in 0..8usize {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                for col in 0..8usize {
                    let nes_idx = (row * 8 + col) as u8;
                    let [r, g, b] = NES_PALETTE[nes_idx as usize];
                    let color = egui::Color32::from_rgb(r, g, b);
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(cell_size, cell_size),
                        egui::Sense::click(),
                    );
                    ui.painter().rect_filled(rect, theme::CR_SM, color);
                    // 編集中セルの現在値をハイライト
                    if let Some((set_idx, color_idx)) = self.editing_palette_cell {
                        let current_idx = self.dat_palette.sets[set_idx][color_idx];
                        if current_idx == nes_idx {
                            ui.painter().rect_stroke(
                                rect, theme::CR_SM,
                                egui::Stroke::new(2.0, egui::Color32::WHITE),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                    let clicked = resp.clicked();
                    resp.on_hover_text(format!("0x{nes_idx:02X}"));
                    if clicked { selected_nes_idx = Some(nes_idx); }
                }
            });
        }
        if let (Some(idx), Some((set_idx, color_idx))) = (selected_nes_idx, self.editing_palette_cell) {
            self.dat_palette.sets[set_idx][color_idx] = idx;
            self.texture_dirty = true;
            self.editing_palette_cell = None;
        }
    }

    // ── パレットパネル ────────────────────────────────────────────

    fn show_palette_panel(&mut self, ui: &mut egui::Ui) {
        ui.label(
            theme::rich_label("パレット"),
        );
        ui.add_space(6.0);

        let swatch_size = egui::vec2(theme::SWATCH_PX, theme::SWATCH_PX);
        let mut set_changed = false;
        let mut open_picker: Option<(usize, usize)> = None;

        for set_idx in 0..4 {
            let is_selected = self.selected_palette_set == set_idx;
            let frame = egui::Frame::new()
                .corner_radius(theme::CR_SM)
                .stroke(if is_selected {
                    egui::Stroke::new(2.0, egui::Color32::WHITE)
                } else {
                    egui::Stroke::new(2.0, egui::Color32::TRANSPARENT)
                })
                .inner_margin(6.0);

            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for color_idx in 0..4 {
                        let color = self.dat_palette.color32(set_idx, color_idx, &self.master_palette);
                        let (rect, resp) = ui.allocate_exact_size(swatch_size, egui::Sense::click());
                        ui.painter().rect_filled(rect, theme::CR_SM, color);
                        ui.painter().rect_stroke(
                            rect, theme::CR_SM,
                            egui::Stroke::new(1.0, theme::COL_SWATCH_BORDER),
                            egui::StrokeKind::Outside,
                        );
                        // 編集中セルの枠を強調
                        if self.editing_palette_cell == Some((set_idx, color_idx)) {
                            ui.painter().rect_stroke(
                                rect, theme::CR_SM,
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
                    ui.add_space(6.0);
                    let label_resp = ui.label(
                        theme::rich_palette_idx(format!("#{set_idx}")),
                    );
                    if label_resp.interact(egui::Sense::click()).clicked() {
                        self.selected_palette_set = set_idx;
                        set_changed = true;
                    }
                });
            });
            ui.add_space(2.0);
        }

        if let Some(cell) = open_picker {
            self.selected_palette_set = cell.0;
            self.editing_palette_cell = Some(cell);
            set_changed = true;
        }
        if set_changed {
            self.texture_dirty = true;
        }
    }
}