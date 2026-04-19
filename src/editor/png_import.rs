use eframe::egui;
use crate::io::png::{MappingStrategy, PngImportResult};
use super::app::RChrApp;

// ── PNG インポートダイアログ状態 ───────────────────────────────────

pub(super) struct PngImportDialog {
    /// 読み込んだ画像の生バイト（再マッピング用）
    png_bytes: Vec<u8>,
    /// ファイル名（表示用）
    file_name: String,
    /// PNG なら true、BMP なら false（false の場合は RgbApprox のみ使用可）
    is_png: bool,
    /// 現在のマッピング戦略
    strategy: MappingStrategy,
    /// 現在の変換結果
    result: PngImportResult,
    /// プレビューテクスチャ（変換後 CHR 色でレンダリング）
    preview_texture: Option<egui::TextureHandle>,
    /// プレビューテクスチャが古くなっているか
    preview_dirty: bool,
}

impl PngImportDialog {
    pub(super) fn new(png_bytes: Vec<u8>, file_name: String, is_png: bool, result: PngImportResult) -> Self {
        let strategy = result.strategy;
        Self {
            png_bytes,
            file_name,
            is_png,
            strategy,
            result,
            preview_texture: None,
            preview_dirty: true,
        }
    }
}

// ── PNG インポート ────────────────────────────────────────────────

impl RChrApp {
    /// メニューから PNG / BMP ファイルを選択して開く
    pub(super) fn open_png_import(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG / BMP 画像", &["png", "bmp"])
            .add_filter("すべてのファイル", &["*"])
            .pick_file()
        else {
            return;
        };
        self.open_png_import_from_path(&path);
    }

    /// パスを直接指定して画像インポートダイアログを開く（D&D 用）
    pub(super) fn open_png_import_from_path(&mut self, path: &std::path::Path) {
        if self.rom.is_none() {
            self.error_msg = Some("先に NES / BIN ファイルを開いてください".into());
            return;
        }
        let img_bytes = match std::fs::read(path) {
            Err(e) => {
                self.error_msg = Some(format!("画像読み込み失敗: {e}"));
                return;
            }
            Ok(b) => b,
        };
        let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        let is_png = ext != "bmp";
        self.open_image_import_with_bytes(img_bytes, file_name, is_png);
    }

    pub(super) fn open_image_import_with_bytes(&mut self, img_bytes: Vec<u8>, file_name: String, is_png: bool) {
        let result = if is_png {
            crate::io::png::import_png(
                &img_bytes,
                &self.dat_palette,
                self.selected_palette_set,
                &self.master_palette,
                None,
            )
        } else {
            crate::io::png::import_bmp(
                &img_bytes,
                &self.dat_palette,
                self.selected_palette_set,
                &self.master_palette,
            )
        };
        match result {
            Err(e) => self.error_msg = Some(format!("変換失敗: {e}")),
            Ok(r) => self.png_import_dialog = Some(PngImportDialog::new(img_bytes, file_name, is_png, r)),
        }
    }

    /// PNG インポートダイアログを表示する
    pub(super) fn show_png_import_dialog(&mut self, ctx: &egui::Context) {
        let dialog = match &mut self.png_import_dialog {
            Some(d) => d,
            None => return,
        };

        // プレビューテクスチャの更新
        if dialog.preview_dirty {
            let w = dialog.result.width;
            let h = dialog.result.height;
            let mut rgba = vec![0u8; w * h * 4];
            for y in 0..h {
                for x in 0..w {
                    let ci = dialog.result.pixels[y][x] as usize;
                    let [r, g, b] = self.dat_palette.color_rgb(
                        self.selected_palette_set, ci, &self.master_palette,
                    );
                    let i = (y * w + x) * 4;
                    rgba[i]     = r;
                    rgba[i + 1] = g;
                    rgba[i + 2] = b;
                    rgba[i + 3] = 255;
                }
            }
            let image = egui::ColorImage::from_rgba_unmultiplied([w.max(1), h.max(1)], &rgba);
            dialog.preview_texture = Some(ctx.load_texture(
                "png_preview",
                image,
                egui::TextureOptions::NEAREST,
            ));
            dialog.preview_dirty = false;
        }

        // ── ダイアログウィンドウ
        let mut do_import = false;
        let mut do_close  = false;
        let mut new_strategy: Option<MappingStrategy> = None;

        egui::Window::new("画像インポート")
            .resizable(true)
            .min_width(360.0)
            .show(ctx, |ui| {
                // ファイル情報
                let tw = dialog.result.tile_width();
                let th = dialog.result.tile_height();
                ui.label(format!(
                    "ファイル: {}  ({}×{} px = {}×{} タイル)",
                    dialog.file_name,
                    dialog.result.width, dialog.result.height,
                    tw, th,
                ));
                ui.add_space(6.0);

                // マッピング戦略選択（BMP は RgbApprox のみ）
                ui.label("マッピング戦略:");
                ui.horizontal(|ui| {
                    for s in [MappingStrategy::PaletteMatch, MappingStrategy::IndexDirect, MappingStrategy::RgbApprox] {
                        let enabled = dialog.is_png || s == MappingStrategy::RgbApprox;
                        let resp = ui.add_enabled(enabled, egui::RadioButton::new(dialog.strategy == s, s.label()));
                        if resp.clicked() && dialog.strategy != s {
                            new_strategy = Some(s);
                        }
                    }
                });
                if !dialog.is_png {
                    ui.colored_label(egui::Color32::GRAY, "BMP はインデックスカラー情報がないため RGB 近似のみ使用できます");
                }
                ui.add_space(6.0);

                // 警告表示
                if !dialog.result.warnings.is_empty() {
                    for w in &dialog.result.warnings {
                        ui.colored_label(egui::Color32::YELLOW, format!("⚠ {w}"));
                    }
                    ui.add_space(4.0);
                }

                // プレビュー
                ui.label("プレビュー（変換後）:");
                if let Some(tex) = &dialog.preview_texture {
                    let pw = (dialog.result.width  * 2).min(512) as f32;
                    let ph = (dialog.result.height * 2).min(512) as f32;
                    let ratio = dialog.result.width as f32 / dialog.result.height.max(1) as f32;
                    let (pw, ph) = if pw / ph > ratio {
                        (ph * ratio, ph)
                    } else {
                        (pw, pw / ratio.max(0.01))
                    };
                    ui.image(egui::load::SizedTexture::new(tex.id(), egui::vec2(pw, ph)));
                }
                ui.add_space(8.0);

                // 貼り付け先情報
                let dest_tile = self.selected_tile.unwrap_or(0);
                ui.label(format!("貼り付け先: タイル {} (0x{:06X}) から", dest_tile, dest_tile * 16));
                ui.add_space(8.0);

                // ボタン行
                ui.horizontal(|ui| {
                    if ui.button("貼り付け").clicked() {
                        do_import = true;
                    }
                    if ui.button("キャンセル").clicked() {
                        do_close = true;
                    }
                });
            });

        // 戦略変更時は再マッピング
        if let Some(s) = new_strategy {
            let (img_bytes, is_png) = {
                let d = self.png_import_dialog.as_ref().unwrap();
                (d.png_bytes.clone(), d.is_png)
            };
            let result = if is_png {
                crate::io::png::import_png(&img_bytes, &self.dat_palette, self.selected_palette_set, &self.master_palette, Some(s))
            } else {
                crate::io::png::import_bmp(&img_bytes, &self.dat_palette, self.selected_palette_set, &self.master_palette)
            };
            match result {
                Ok(result) => {
                    let dialog = self.png_import_dialog.as_mut().unwrap();
                    dialog.strategy = s;
                    dialog.result = result;
                    dialog.preview_dirty = true;
                }
                Err(e) => {
                    self.error_msg = Some(format!("変換失敗: {e}"));
                }
            }
        }

        // 貼り付け実行
        if do_import {
            self.apply_png_import();
            self.png_import_dialog = None;
            return;
        }

        if do_close {
            self.png_import_dialog = None;
        }
    }

    /// PNG インポート結果を CHR データに書き込む
    pub(super) fn apply_png_import(&mut self) {
        if self.png_import_dialog.is_none() || self.rom.is_none() { return }
        let top_left_tile = self.selected_tile.unwrap_or(0);
        let chr_len = self.rom.as_ref().unwrap().chr_data().len();

        // Undo 用: 影響範囲の全タイルを保存（rom の借用を先に解放）
        let (tw, th, top_row, top_col) = {
            let d = self.png_import_dialog.as_ref().unwrap();
            (d.result.tile_width(), d.result.tile_height(),
             top_left_tile / 16, top_left_tile % 16)
        };
        let mut batch: Vec<(usize, [u8; 16])> = Vec::new();
        for by in 0..th {
            for bx in 0..tw {
                let offset = ((top_row + by) * 16 + (top_col + bx)) * 16;
                if offset + 16 <= chr_len {
                    let saved: [u8; 16] = self.rom.as_ref().unwrap().chr_data()
                        [offset..offset + 16].try_into().unwrap();
                    batch.push((offset, saved));
                }
            }
        }
        self.push_undo_batch(batch);

        // CHR へ書き込み
        let result_tw = tw;
        let result_th = th;
        {
            let dialog = self.png_import_dialog.as_ref().unwrap();
            let result = &dialog.result;
            crate::io::png::write_to_chr(
                self.rom.as_mut().unwrap().chr_data_mut(),
                result,
                top_left_tile,
                16,
            );
        }

        self.is_modified = true;
        self.texture_dirty = true;
        self.status_msg = Some(format!("PNG インポート完了: {}×{} タイル", result_tw, result_th));
    }
}