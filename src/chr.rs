use egui::ColorImage;
use crate::palette::{DatPalette, MasterPalette};

/// 2BPP NES 形式で 16バイトを 8×8 ピクセル（インデックス 0〜3）にデコード
///
/// NES の 2BPP 形式:
///   先頭 8バイト = プレーン 0（LSB）
///   後続 8バイト = プレーン 1（MSB）
///   各行の左端ピクセルがビット 7、右端がビット 0
pub fn decode_tile(data: &[u8]) -> [[u8; 8]; 8] {
    let mut pixels = [[0u8; 8]; 8];
    for row in 0..8 {
        let plane0 = data[row];
        let plane1 = data[row + 8];
        for col in 0..8 {
            let bit = 7 - col;
            let lo = (plane0 >> bit) & 1;
            let hi = (plane1 >> bit) & 1;
            pixels[row][col] = (hi << 1) | lo;
        }
    }
    pixels
}

/// 指定タイル（左上）から N×N タイルをデコードして (N*8)×(N*8) のカラーインデックス配列を返す
///
/// - `top_left_tile`  : 左上タイルのグローバルインデックス
/// - `tiles_per_row`  : CHR レイアウトの 1 行タイル数（通常 16）
/// - `n`              : ブロック 1 辺のタイル数
pub fn decode_block(
    chr_data: &[u8],
    top_left_tile: usize,
    tiles_per_row: usize,
    n: usize,
) -> Vec<Vec<u8>> {
    let block_px = n * 8;
    let mut pixels = vec![vec![0u8; block_px]; block_px];

    let top_row = top_left_tile / tiles_per_row;
    let top_col = top_left_tile % tiles_per_row;

    for by in 0..n {
        for bx in 0..n {
            let tile_idx = (top_row + by) * tiles_per_row + (top_col + bx);
            let tile_offset = tile_idx * 16;
            if tile_offset + 16 > chr_data.len() {
                continue; // 範囲外はスキップ（黒のまま）
            }
            let tile = decode_tile(&chr_data[tile_offset..tile_offset + 16]);
            for py in 0..8 {
                for px in 0..8 {
                    pixels[by * 8 + py][bx * 8 + px] = tile[py][px];
                }
            }
        }
    }
    pixels
}

/// タイルデータの指定ドット (px, py) にカラーインデックスを書き込む（2BPP NES 形式）
///
/// data は 16バイト（プレーン 0: 0〜7, プレーン 1: 8〜15）
pub fn encode_dot(data: &mut [u8], px: usize, py: usize, color_idx: u8) {
    let bit = 7 - px;
    data[py]     = (data[py]     & !(1 << bit)) | ((color_idx & 1)        << bit);
    data[py + 8] = (data[py + 8] & !(1 << bit)) | (((color_idx >> 1) & 1) << bit);
}

/// CHR データ全体を 16 タイル幅の縦長 ColorImage としてレンダリングする
///
/// 幅: 128 px（16 タイル × 8 px）
/// 高: 8 × ceil(total_tiles / 16) px
pub fn render_full_image(
    chr_data: &[u8],
    palette: &DatPalette,
    palette_set: usize,
    master: &MasterPalette,
) -> ColorImage {
    let total_tiles = chr_data.len() / 16;
    let total_rows = (total_tiles + 15) / 16;
    const W: usize = 128;
    let h = (total_rows * 8).max(8);

    let mut rgba = vec![0u8; W * h * 4];

    for tile_idx in 0..total_tiles {
        let tile_offset = tile_idx * 16;
        let tile = decode_tile(&chr_data[tile_offset..tile_offset + 16]);
        let tile_col = tile_idx % 16;
        let tile_row = tile_idx / 16;

        for py in 0..8 {
            for px in 0..8 {
                let img_x = tile_col * 8 + px;
                let img_y = tile_row * 8 + py;
                let color_idx = tile[py][px] as usize;
                let [r, g, b] = palette.color_rgb(palette_set, color_idx, master);
                let i = (img_y * W + img_x) * 4;
                rgba[i]     = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = 255;
            }
        }
    }

    ColorImage::from_rgba_unmultiplied([W, h], &rgba)
}

/// 指定 CHR データに対して有効なバンク数を返す（1バンク = 0x1000バイト）
pub fn bank_count(chr_data: &[u8]) -> usize {
    chr_data.len() / 0x1000
}