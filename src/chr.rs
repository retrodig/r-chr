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

/// CHR データの指定バイトオフセットから 256タイル分を
/// 128×128 ピクセルの ColorImage としてレンダリングする
///
/// レイアウト: 16タイル × 16タイル（各タイル 8×8px）
/// カラーは DatPalette の指定セットを使用する
pub fn render_bank_image(
    chr_data: &[u8],
    bank_offset: usize,
    palette: &DatPalette,
    palette_set: usize,
    master: &MasterPalette,
) -> ColorImage {
    const TILES_PER_ROW: usize = 16;
    const IMG_SIZE: usize = 128; // 16 * 8

    let mut rgba = vec![0u8; IMG_SIZE * IMG_SIZE * 4];

    for tile_idx in 0..256usize {
        let tile_byte_offset = bank_offset + tile_idx * 16;
        if tile_byte_offset + 16 > chr_data.len() {
            break;
        }

        let tile = decode_tile(&chr_data[tile_byte_offset..tile_byte_offset + 16]);

        let tile_col = tile_idx % TILES_PER_ROW;
        let tile_row = tile_idx / TILES_PER_ROW;

        for py in 0..8 {
            for px in 0..8 {
                let img_x = tile_col * 8 + px;
                let img_y = tile_row * 8 + py;
                let color_idx = tile[py][px] as usize;
                let [r, g, b] = palette.color_rgb(palette_set, color_idx, master);
                let i = (img_y * IMG_SIZE + img_x) * 4;
                rgba[i]     = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = 255;
            }
        }
    }

    ColorImage::from_rgba_unmultiplied([IMG_SIZE, IMG_SIZE], &rgba)
}

/// タイルデータの指定ドット (px, py) にカラーインデックスを書き込む（2BPP NES 形式）
///
/// data は 16バイト（プレーン 0: 0〜7, プレーン 1: 8〜15）
pub fn encode_dot(data: &mut [u8], px: usize, py: usize, color_idx: u8) {
    let bit = 7 - px;
    data[py]     = (data[py]     & !(1 << bit)) | ((color_idx & 1)        << bit);
    data[py + 8] = (data[py + 8] & !(1 << bit)) | (((color_idx >> 1) & 1) << bit);
}

/// 指定 CHR データに対して有効なバンク数を返す（1バンク = 0x1000バイト）
pub fn bank_count(chr_data: &[u8]) -> usize {
    chr_data.len() / 0x1000
}