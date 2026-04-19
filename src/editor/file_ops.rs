use crate::io::nes::{RomData, parse_nes};
use crate::model::palette::{DatPalette, MasterPalette};
use super::app::RChrApp;

// ── ZIP ユーティリティ ─────────────────────────────────────────────

fn extract_nes_from_zip(zip_data: &[u8]) -> Result<(String, Vec<u8>), String> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("ZIP read failed: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("ZIP entry read failed: {e}"))?;
        if entry.name().to_ascii_lowercase().ends_with(".nes") {
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data)
                .map_err(|e| format!("ZIP extract failed: {e}"))?;
            return Ok((name, data));
        }
    }
    Err("no_nes_in_zip".into())   // i18n キーとして app 側で t() に変換
}

// ── ファイル操作 ───────────────────────────────────────────────────

impl RChrApp {
    pub(super) fn new_file(&mut self) {
        let chr_data = vec![0u8; 0x4000];
        self.error_msg = None;
        self.file_name = Some("newfile".to_string());
        self.file_path = None;
        self.raw_file_data = None;
        self.scroll_addr = 0;
        self.pending_scroll_addr = Some(0);
        self.selected_tile = None;
        self.undo_stack.clear();
        self.drag_undo_tiles.clear();
        self.is_modified = false;
        self.rom = Some(RomData::Bin(chr_data));
        self.texture_dirty = true;
    }

    pub(super) fn open_file(&mut self) {
        let t = self.t();
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NES / BIN / ZIP", &["nes", "bin", "zip"])
            .add_filter(t.filter_all, &["*"])
            .pick_file()
        else {
            return;
        };
        self.open_file_from_path(&path);
    }

    pub(super) fn open_file_from_path(&mut self, path: &std::path::Path) {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        let data = match std::fs::read(path) {
            Err(e) => {
                self.error_msg = Some(format!("Load failed: {e}"));
                return;
            }
            Ok(d) => d,
        };

        let (nes_data, display_name, save_path) = if ext == "zip" {
            match extract_nes_from_zip(&data) {
                Err(e) => {
                    let msg = if e == "no_nes_in_zip" {
                        self.t().err_zip_no_nes.to_string()
                    } else {
                        e
                    };
                    self.error_msg = Some(msg);
                    return;
                }
                Ok((inner_name, inner_data)) => (inner_data, inner_name, None),
            }
        } else {
            (data, file_name, Some(path.to_path_buf()))
        };

        let rom_data = if save_path.as_ref().map_or(false, |p| {
            p.extension().and_then(|e| e.to_str()).map_or(false, |e| e.eq_ignore_ascii_case("bin"))
        }) || (ext == "bin") {
            if nes_data.is_empty() {
                self.error_msg = Some(self.t().err_bin_empty.to_string());
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
        self.drag_undo_tiles.clear();
        self.file_path = save_path;
        self.is_modified = false;
        self.rom = Some(rom_data);
        self.texture_dirty = true;
    }

    // ── 保存 ──────────────────────────────────────────────────────

    pub(super) fn save_file(&mut self) -> Result<(), String> {
        let path = self.file_path.clone().ok_or_else(|| self.t().err_no_save_path.to_string())?;
        self.write_to_path(&path)
    }

    pub(super) fn save_file_as(&mut self) -> Result<(), String> {
        let t = self.t();
        let is_bin = self.rom.as_ref().map_or(false, |r| !r.is_nes());
        let default_name = self.file_name.clone().unwrap_or_else(|| {
            if is_bin { "output.bin".into() } else { "output.nes".into() }
        });
        let mut dialog = rfd::FileDialog::new().set_file_name(&default_name);
        dialog = if is_bin {
            dialog.add_filter(t.filter_chr, &["bin"])
        } else {
            dialog.add_filter("NES ROM", &["nes"])
        };
        let Some(path) = dialog.save_file() else {
            return Ok(());
        };
        self.write_to_path(&path)?;
        self.file_name = Some(
            path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        );
        self.file_path = Some(path);
        Ok(())
    }

    pub(super) fn write_to_path(&mut self, path: &std::path::Path) -> Result<(), String> {
        let t = self.t();
        let rom = self.rom.as_ref().ok_or(t.err_no_rom)?;

        match rom {
            RomData::Nes(nes_rom) => {
                let raw = self.raw_file_data.as_mut().ok_or(t.err_no_raw)?;
                let start = nes_rom.chr_data_offset;
                let end   = start + nes_rom.chr_rom.len();
                if end > raw.len() {
                    return Err(t.err_filesize.to_string());
                }
                raw[start..end].copy_from_slice(&nes_rom.chr_rom);
                std::fs::write(path, raw as &[u8]).map_err(|e| format!("Save failed: {e}"))?;
            }
            RomData::Bin(chr_data) => {
                std::fs::write(path, chr_data).map_err(|e| format!("Save failed: {e}"))?;
            }
        }

        self.is_modified = false;
        Ok(())
    }

    // ── PAL / DAT パレットファイル操作 ───────────────────────────

    pub(super) fn load_pal_file(&mut self) {
        let t = self.t();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(t.filter_pal, &["pal"])
            .add_filter(t.filter_all, &["*"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Err(e) => {
                self.error_msg = Some(format!("Load failed: {e}"));
            }
            Ok(data) => match MasterPalette::from_pal_bytes(&data) {
                None => {
                    self.error_msg = Some(self.lang.fmt_pal_too_short(data.len()));
                }
                Some(master) => {
                    self.master_palette = master;
                    self.texture_dirty = true;
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    self.status_msg = Some(self.lang.fmt_pal_loaded(&name));
                }
            },
        }
    }

    pub(super) fn load_dat_file(&mut self) {
        let t = self.t();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(t.filter_dat, &["dat"])
            .add_filter(t.filter_all, &["*"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read(&path) {
            Err(e) => {
                self.error_msg = Some(format!("Load failed: {e}"));
            }
            Ok(data) => match DatPalette::from_dat_bytes(&data) {
                None => {
                    self.error_msg = Some(format!("DAT file too short ({} bytes, need 16)", data.len()));
                }
                Some(palette) => {
                    self.dat_palette = palette;
                    self.texture_dirty = true;
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    self.status_msg = Some(self.lang.fmt_dat_loaded(&name));
                }
            },
        }
    }

    pub(super) fn save_dat_file(&mut self) {
        let t = self.t();
        let Some(path) = rfd::FileDialog::new()
            .add_filter(t.filter_dat, &["dat"])
            .set_file_name("palette.dat")
            .save_file()
        else {
            return;
        };
        let bytes = self.dat_palette.to_dat_bytes();
        match std::fs::write(&path, &bytes) {
            Err(e) => {
                self.error_msg = Some(format!("Save failed: {e}"));
            }
            Ok(()) => {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                self.status_msg = Some(self.lang.fmt_dat_saved(&name));
            }
        }
    }
}