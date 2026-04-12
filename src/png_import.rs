//! PNG ファイルを CHR カラーインデックス（0〜3）に変換するモジュール

use crate::palette::{DatPalette, MasterPalette};

// ── マッピング戦略 ────────────────────────────────────────────────

/// ピクセルを CHR カラーインデックス（0〜3）へ変換する戦略
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MappingStrategy {
    /// インデックス PNG のピクセル値を mod 4 する（パレット先頭 4 色 = CHR 0〜3）
    IndexDirect,
    /// インデックス PNG の PLTE を NES マスターパレットに照合し DAT セットの 0〜3 に変換
    PaletteMatch,
    /// フルカラー PNG の各ピクセルを DAT パレットセット 4 色に RGB 近似マッピング
    RgbApprox,
}

impl MappingStrategy {
    pub fn label(self) -> &'static str {
        match self {
            Self::IndexDirect  => "インデックス直接 (mod 4)",
            Self::PaletteMatch => "パレット照合 (推奨)",
            Self::RgbApprox    => "RGB 近似",
        }
    }
}

// ── インポート結果 ────────────────────────────────────────────────

pub struct PngImportResult {
    /// 画像幅（ピクセル）
    pub width: usize,
    /// 画像高さ（ピクセル）
    pub height: usize,
    /// CHR カラーインデックス [y][x] = 0〜3
    pub pixels: Vec<Vec<u8>>,
    /// 実際に使用したマッピング戦略
    pub strategy: MappingStrategy,
    /// 警告メッセージ
    pub warnings: Vec<String>,
}

impl PngImportResult {
    /// 画像が占めるタイル数（横）
    pub fn tile_width(&self) -> usize  { (self.width  + 7) / 8 }
    /// 画像が占めるタイル数（縦）
    pub fn tile_height(&self) -> usize { (self.height + 7) / 8 }
}

// ── カラー距離 ────────────────────────────────────────────────────

fn color_distance(a: [u8; 3], b: [u8; 3]) -> u32 {
    let dr = a[0] as i32 - b[0] as i32;
    let dg = a[1] as i32 - b[1] as i32;
    let db = a[2] as i32 - b[2] as i32;
    (dr * dr + dg * dg + db * db) as u32
}

/// RGB を DAT パレットセット 4 色の中で最近傍の色インデックス（0〜3）に変換
fn nearest_dat_index(rgb: [u8; 3], dat: &DatPalette, set: usize, master: &MasterPalette) -> u8 {
    (0u8..4)
        .min_by_key(|&i| color_distance(rgb, dat.color_rgb(set as usize, i as usize, master)))
        .unwrap_or(0)
}

// ── PNG 読み込み ──────────────────────────────────────────────────

/// PNG ファイルのメタ情報（インデックスカラーかどうか、PLTE など）
struct PngMeta {
    width: u32,
    height: u32,
    is_indexed: bool,
    /// インデックスカラー時の PLTE（最大 256 色の RGB）
    palette: Vec<[u8; 3]>,
    /// ピクセルデータ（インデックスカラー: 1バイト/px 展開済み / 非インデックス: 生バイト列）
    raw_pixels: Vec<u8>,
    /// インデックス PNG の tRNS チャンク: [i] = パレットエントリ i のアルファ値（0=完全透明）
    /// 記載のないエントリは不透明（255）扱い
    transparency: Vec<u8>,
}

fn read_png_meta(data: &[u8]) -> Result<PngMeta, String> {
    use png::{BitDepth, ColorType};

    let decoder = png::Decoder::new(std::io::Cursor::new(data));
    let mut reader = decoder.read_info().map_err(|e| format!("PNG 解析失敗: {e}"))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("PNG フレーム読み込み失敗: {e}"))?;

    let is_indexed = matches!(info.color_type, ColorType::Indexed);
    let palette = reader
        .info()
        .palette
        .as_deref()
        .unwrap_or(&[])
        .chunks_exact(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect();
    // tRNS チャンク: インデックス PNG では各パレットエントリのアルファ値
    let transparency = reader
        .info()
        .trns
        .as_deref()
        .unwrap_or(&[])
        .to_vec();

    let w = info.width as usize;
    let h = info.height as usize;
    let raw = &buf[..info.buffer_size()];

    // インデックス PNG は bit_depth によって 1バイトに複数ピクセルが詰まっている場合がある。
    // 後段のコードが [y * w + x] で安全にアクセスできるよう、1バイト/ピクセルに展開する。
    let raw_pixels = if is_indexed {
        match info.bit_depth {
            BitDepth::Eight => raw.to_vec(),
            BitDepth::Four => {
                let mut pixels = Vec::with_capacity(w * h);
                for y in 0..h {
                    let row = &raw[y * info.line_size..];
                    for x in 0..w {
                        let byte = row[x / 2];
                        let px = if x % 2 == 0 { (byte >> 4) & 0x0F } else { byte & 0x0F };
                        pixels.push(px);
                    }
                }
                pixels
            }
            BitDepth::Two => {
                let mut pixels = Vec::with_capacity(w * h);
                for y in 0..h {
                    let row = &raw[y * info.line_size..];
                    for x in 0..w {
                        let byte = row[x / 4];
                        let shift = (3 - (x % 4)) * 2;
                        pixels.push((byte >> shift) & 0x03);
                    }
                }
                pixels
            }
            BitDepth::One => {
                let mut pixels = Vec::with_capacity(w * h);
                for y in 0..h {
                    let row = &raw[y * info.line_size..];
                    for x in 0..w {
                        let byte = row[x / 8];
                        let shift = 7 - (x % 8);
                        pixels.push((byte >> shift) & 0x01);
                    }
                }
                pixels
            }
            _ => return Err(format!("未対応のビット深度: {:?}", info.bit_depth)),
        }
    } else {
        // RGB / RGBA はそのまま保持（RgbApprox 戦略では image クレートで再デコードする）
        raw.to_vec()
    };

    Ok(PngMeta {
        width: info.width,
        height: info.height,
        is_indexed,
        palette,
        raw_pixels,
        transparency,
    })
}

// ── マッピング本体 ────────────────────────────────────────────────

/// PNG データ（バイト列）を CHR カラーインデックスに変換する
pub fn import_png(
    png_data: &[u8],
    dat: &DatPalette,
    palette_set: usize,
    master: &MasterPalette,
    strategy_hint: Option<MappingStrategy>,
) -> Result<PngImportResult, String> {
    let meta = read_png_meta(png_data)?;
    let w = meta.width as usize;
    let h = meta.height as usize;
    let mut warnings: Vec<String> = Vec::new();

    // 戦略を自動選択（ヒントがあればそれを使う）
    let strategy = match strategy_hint {
        Some(s) => s,
        None => {
            if meta.is_indexed && !meta.palette.is_empty() {
                MappingStrategy::PaletteMatch
            } else if meta.is_indexed {
                MappingStrategy::IndexDirect
            } else {
                MappingStrategy::RgbApprox
            }
        }
    };

    let mut pixels = vec![vec![0u8; w]; h];

    // tRNS: パレットエントリのアルファ値（未記載エントリは 255 = 不透明）
    let transparency = &meta.transparency;
    let is_transparent_entry = |idx: usize| -> bool {
        transparency.get(idx).copied().unwrap_or(255) == 0
    };

    match strategy {
        // ── インデックス直接 ──────────────────────────────────────
        MappingStrategy::IndexDirect => {
            if !meta.is_indexed {
                return Err("インデックス直接モードはインデックスカラー PNG 専用です".into());
            }

            // 透明エントリを除いた実質的なインデックス最大値を確認
            let max_opaque_idx = meta.raw_pixels.iter().copied()
                .filter(|&i| !is_transparent_entry(i as usize))
                .max()
                .unwrap_or(0);
            if max_opaque_idx >= 4 {
                warnings.push(format!(
                    "インデックス値の最大が {} です。mod 4 で変換します。",
                    max_opaque_idx
                ));
            }

            let mut transparent_count = 0usize;
            for y in 0..h {
                for x in 0..w {
                    let idx = meta.raw_pixels[y * w + x] as usize;
                    if is_transparent_entry(idx) {
                        pixels[y][x] = 0; // 透明 → CHR インデックス 0
                        transparent_count += 1;
                    } else {
                        pixels[y][x] = (idx % 4) as u8;
                    }
                }
            }
            if transparent_count > 0 {
                warnings.push(format!("透明ピクセル {} px → インデックス 0 に変換", transparent_count));
            }
        }

        // ── パレット照合 ──────────────────────────────────────────
        MappingStrategy::PaletteMatch => {
            if !meta.is_indexed {
                return Err("パレット照合モードはインデックスカラー PNG 専用です".into());
            }
            if meta.palette.is_empty() {
                return Err("PLTE（パレット）チャンクが見つかりません".into());
            }

            // PLTE の各エントリ → NES マスターパレット最近傍 → DAT セット色インデックス
            // 透明エントリは CHR 0 に強制マッピング
            let plte_len = meta.palette.len();
            let mut plte_to_chr = vec![0u8; plte_len];
            let mut unmatched_count = 0usize;
            let mut transparent_entries = 0usize;

            for (i, &rgb) in meta.palette.iter().enumerate() {
                if is_transparent_entry(i) {
                    plte_to_chr[i] = 0; // 透明 → CHR 0
                    transparent_entries += 1;
                    continue;
                }

                // NES マスターパレット（64 色）に最近傍マッチ
                let nes_idx = (0usize..64)
                    .min_by_key(|&j| color_distance(rgb, master.colors[j]))
                    .unwrap_or(0);
                let nes_rgb = master.colors[nes_idx];

                // DAT パレットセット内で最近傍の色インデックスを探す
                let mut best_chr = 0u8;
                let mut best_dist = u32::MAX;
                for c in 0u8..4 {
                    let dat_rgb = dat.color_rgb(palette_set, c as usize, master);
                    let d = color_distance(nes_rgb, dat_rgb);
                    if d < best_dist {
                        best_dist = d;
                        best_chr = c;
                    }
                }
                plte_to_chr[i] = best_chr;
                if best_dist > 0 {
                    unmatched_count += 1;
                }
            }

            if transparent_entries > 0 {
                warnings.push(format!("透明パレットエントリ {} 色 → インデックス 0 に変換", transparent_entries));
            }
            if unmatched_count > 0 {
                warnings.push(format!(
                    "{} 色がパレットに完全一致せず近似されました",
                    unmatched_count
                ));
            }

            for y in 0..h {
                for x in 0..w {
                    let idx = meta.raw_pixels[y * w + x] as usize;
                    pixels[y][x] = if idx < plte_len { plte_to_chr[idx] } else { 0 };
                }
            }
        }

        // ── RGB 近似 ──────────────────────────────────────────────
        MappingStrategy::RgbApprox => {
            // image クレートで RGBA に展開（アルファチャンネルを保持するため）
            let img = image::load_from_memory(png_data)
                .map_err(|e| format!("画像デコード失敗: {e}"))?
                .into_rgba8();

            if img.width() as usize != w || img.height() as usize != h {
                return Err("画像サイズ不一致".into());
            }

            let mut approx_count = 0usize;
            let mut transparent_count = 0usize;
            for y in 0..h {
                for x in 0..w {
                    let px = img.get_pixel(x as u32, y as u32);
                    // アルファ < 128 は透明扱い → CHR インデックス 0
                    if px[3] < 128 {
                        pixels[y][x] = 0;
                        transparent_count += 1;
                        continue;
                    }
                    let rgb = [px[0], px[1], px[2]];
                    let chr_idx = nearest_dat_index(rgb, dat, palette_set, master);
                    let expected_rgb = dat.color_rgb(palette_set, chr_idx as usize, master);
                    if expected_rgb != rgb { approx_count += 1; }
                    pixels[y][x] = chr_idx;
                }
            }
            if transparent_count > 0 {
                warnings.push(format!("透明ピクセル {} px → インデックス 0 に変換", transparent_count));
            }
            if approx_count > 0 {
                warnings.push(format!(
                    "{} ピクセルが DAT パレット 4 色に完全一致せず近似されました",
                    approx_count
                ));
            }
        }
    }

    Ok(PngImportResult { width: w, height: h, pixels, strategy, warnings })
}

// ── CHR への書き込み ──────────────────────────────────────────────

/// PngImportResult のピクセルを CHR データに書き込む
///
/// - `top_left_tile`: 左上タイルのグローバルインデックス
/// - `tiles_per_row`: 通常 16
pub fn write_to_chr(
    chr_data: &mut [u8],
    result: &PngImportResult,
    top_left_tile: usize,
    tiles_per_row: usize,
) {
    use crate::chr::encode_dot;

    let top_row = top_left_tile / tiles_per_row;
    let top_col = top_left_tile % tiles_per_row;

    for py in 0..result.height {
        for px in 0..result.width {
            let tile_col = top_col + px / 8;
            let tile_row = top_row + py / 8;
            let tile_global = tile_row * tiles_per_row + tile_col;
            let tile_offset = tile_global * 16;
            if tile_offset + 16 > chr_data.len() {
                continue; // CHR 末尾を超えたらスキップ
            }
            encode_dot(
                &mut chr_data[tile_offset..tile_offset + 16],
                px % 8,
                py % 8,
                result.pixels[py][px],
            );
        }
    }
}