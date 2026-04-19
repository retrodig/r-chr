use crate::io::nes::{RomData, parse_nes};
use crate::model::palette::{DatPalette, MasterPalette};
use super::app::RChrApp;

// ── ZIP ユーティリティ ─────────────────────────────────────────────

/// ZIP バイト列から最初の .nes ファイルを取り出す。
/// 戻り値: (ファイル名, NES バイト列)
fn extract_nes_from_zip(zip_data: &[u8]) -> Result<(String, Vec<u8>), String> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(zip_data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| format!("ZIP 読み込み失敗: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("ZIP エントリ読み込み失敗: {e}"))?;
        if entry.name().to_ascii_lowercase().ends_with(".nes") {
            let name = entry.name().to_string();
            let mut data = Vec::new();
            entry.read_to_end(&mut data)
                .map_err(|e| format!("ZIP 展開失敗: {e}"))?;
            return Ok((name, data));
        }
    }
    Err("ZIP 内に .nes ファイルが見つかりませんでした".into())
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
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NES / BIN / ZIP", &["nes", "bin", "zip"])
            .add_filter("すべてのファイル", &["*"])
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
                self.error_msg = Some(format!("読み込み失敗: {e}"));
                return;
            }
            Ok(d) => d,
        };

        // ZIP の場合: 内部の最初の .nes を取り出して処理
        let (nes_data, display_name, save_path) = if ext == "zip" {
            match extract_nes_from_zip(&data) {
                Err(e) => {
                    self.error_msg = Some(e);
                    return;
                }
                Ok((inner_name, inner_data)) => {
                    // ZIP から展開した場合は保存先が確定しないので file_path は None
                    (inner_data, inner_name, None)
                }
            }
        } else {
            (data, file_name, Some(path.to_path_buf()))
        };

        let rom_data = if save_path.as_ref().map_or(false, |p| {
            p.extension().and_then(|e| e.to_str()).map_or(false, |e| e.eq_ignore_ascii_case("bin"))
        }) || (ext == "bin") {
            if nes_data.is_empty() {
                self.error_msg = Some("BIN ファイルが空です".into());
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

    /// 現在のパスに上書き保存する
    pub(super) fn save_file(&mut self) -> Result<(), String> {
        let path = self.file_path.clone().ok_or("保存先パスがありません")?;
        self.write_to_path(&path)
    }

    /// ダイアログで保存先を選んで保存する
    pub(super) fn save_file_as(&mut self) -> Result<(), String> {
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
    pub(super) fn write_to_path(&mut self, path: &std::path::Path) -> Result<(), String> {
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
    pub(super) fn load_pal_file(&mut self) {
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
    pub(super) fn load_dat_file(&mut self) {
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
    pub(super) fn save_dat_file(&mut self) {
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
}
