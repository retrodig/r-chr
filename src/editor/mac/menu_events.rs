//! macOS ネイティブメニューイベント処理
#![cfg(target_os = "macos")]

use crate::native_menu::{self, MenuAction};
use crate::model::palette::MasterPalette;
use super::super::app::{NES_PAL, RChrApp};
use super::super::i18n::Lang;

impl RChrApp {
    pub(in crate::editor) fn handle_native_menu(&mut self, ctx: &egui::Context) {
        let _ = ctx;
        while let Some(action) = native_menu::try_recv_action() {
            match action {
                MenuAction::About           => self.show_about = true,
                MenuAction::FileNew         => self.new_file(),
                MenuAction::FileOpen        => self.open_file(),
                MenuAction::FileImportPng   => self.open_png_import(),
                MenuAction::FileSave        => { if let Err(e) = self.save_file()    { self.error_msg = Some(e); } }
                MenuAction::FileSaveAs      => { if let Err(e) = self.save_file_as() { self.error_msg = Some(e); } }
                MenuAction::EditUndo        => self.do_undo(),
                MenuAction::EditCopy        => self.copy_tiles(),
                MenuAction::EditPaste       => self.paste_tiles(),
                MenuAction::ViewDarkMode(v) => {
                    self.dark_mode = v;
                    native_menu::set_app_appearance(v);
                }
                MenuAction::LangEnglish(en) => {
                    self.lang = if en { Lang::En } else { Lang::Ja };
                    native_menu::set_menu_lang(self.lang);
                }
                MenuAction::PaletteOpenPal  => self.load_pal_file(),
                MenuAction::PaletteOpenDat  => self.load_dat_file(),
                MenuAction::PaletteSaveDat  => self.save_dat_file(),
                MenuAction::PaletteReset    => {
                    self.master_palette = MasterPalette::from_pal_bytes(NES_PAL)
                        .unwrap_or_default();
                    self.texture_dirty = true;
                    self.status_msg = Some(self.t().status_pal_reset.into());
                }
            }
        }

        // enabled / checked 状態を毎フレーム同期
        let has_tile = self.selected_tile.is_some()
            && self.rom.as_ref().map_or(false, |r| !r.chr_data().is_empty());
        native_menu::sync_state(
            self.file_path.is_some() && self.is_modified,
            !self.undo_stack.is_empty(),
            has_tile,
            has_tile && self.tile_clipboard.is_some(),
            self.dark_mode,
            self.lang,
        );
    }
}