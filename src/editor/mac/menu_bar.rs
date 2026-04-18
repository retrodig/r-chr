//! egui メニューバー（macOS はネイティブメニューを使うため非表示）
#![cfg(not(target_os = "macos"))]

use crate::model::palette::MasterPalette;
use super::super::app::{NES_PAL, RChrApp};

impl RChrApp {
    pub(in crate::editor) fn show_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("ファイル", |ui| {
                    if ui.button("新規作成  ⌘N").clicked() {
                        self.new_file();
                        ui.close_menu();
                    }
                    ui.separator();
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
    }
}