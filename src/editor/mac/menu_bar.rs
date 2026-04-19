//! egui メニューバー（macOS はネイティブメニューを使うため非表示）
#![cfg(not(target_os = "macos"))]

use crate::model::palette::MasterPalette;
use super::super::app::{NES_PAL, RChrApp};
use super::super::i18n::Lang;

impl RChrApp {
    pub(in crate::editor) fn show_menu_bar(&mut self, ctx: &egui::Context) {
        let t = self.t();
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button(t.menu_file, |ui| {
                    if ui.button(format!("{}  ⌘N", t.file_new)).clicked() {
                        self.new_file();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(format!("{}  ⌘O", t.file_open)).clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                    if ui.button(t.file_import_png).clicked() {
                        self.open_png_import();
                        ui.close_menu();
                    }
                    ui.separator();
                    let can_save = self.file_path.is_some() && self.is_modified;
                    if ui.add_enabled(can_save, egui::Button::new(format!("{}  ⌘S", t.file_save))).clicked() {
                        if let Err(e) = self.save_file() {
                            self.error_msg = Some(e);
                        }
                        ui.close_menu();
                    }
                    if ui.button(format!("{}  ⌘⇧S", t.file_save_as)).clicked() {
                        if let Err(e) = self.save_file_as() {
                            self.error_msg = Some(e);
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button(t.menu_edit, |ui| {
                    let can_undo = !self.undo_stack.is_empty();
                    if ui.add_enabled(can_undo, egui::Button::new(format!("{}  ⌘Z / Ctrl+Z", t.edit_undo))).clicked() {
                        self.do_undo();
                        ui.close_menu();
                    }
                });
                ui.menu_button(t.menu_view, |ui| {
                    if ui.checkbox(&mut self.dark_mode, t.view_dark_mode).clicked() {
                        ui.close_menu();
                    }
                    ui.separator();
                    let mut is_en = self.lang == Lang::En;
                    if ui.checkbox(&mut is_en, t.lang_english).clicked() {
                        self.lang = if is_en { Lang::En } else { Lang::Ja };
                        ui.close_menu();
                    }
                });
                ui.menu_button(t.menu_palette, |ui| {
                    if ui.button(t.pal_open_pal).clicked() {
                        self.load_pal_file();
                        ui.close_menu();
                    }
                    if ui.button(t.pal_open_dat).clicked() {
                        self.load_dat_file();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(t.pal_save_dat).clicked() {
                        self.save_dat_file();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(t.pal_reset).clicked() {
                        self.master_palette = MasterPalette::from_pal_bytes(NES_PAL)
                            .unwrap_or_default();
                        self.texture_dirty = true;
                        self.status_msg = Some(self.t().status_pal_reset.into());
                        ui.close_menu();
                    }
                });
            });
        });
    }
}