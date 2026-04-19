//! タイルのコピー & ペースト
use super::app::RChrApp;

impl RChrApp {
    /// 選択中ブロック（focus_size × focus_size タイル）をコピー
    pub(super) fn copy_tiles(&mut self) {
        let Some(tile_idx) = self.selected_tile else { return };
        let Some(rom) = &self.rom else { return };
        let chr = rom.chr_data();
        let n = self.focus_size.tile_count();
        let total_tiles = chr.len() / 16;

        let mut buf = Vec::with_capacity(n * n * 16);
        for dy in 0..n {
            for dx in 0..n {
                let t = tile_idx + dy * 16 + dx;
                let offset = t * 16;
                if t < total_tiles && offset + 16 <= chr.len() {
                    buf.extend_from_slice(&chr[offset..offset + 16]);
                } else {
                    buf.extend_from_slice(&[0u8; 16]);
                }
            }
        }
        self.tile_clipboard = Some((n, buf));
        self.status_msg = Some(format!("コピー: {}×{} タイル", n, n));
    }

    /// コピーバッファを選択位置にペースト（アンドゥ対応）
    pub(super) fn paste_tiles(&mut self) {
        let Some(tile_idx) = self.selected_tile else { return };
        let Some((n, src)) = self.tile_clipboard.clone() else { return };
        let Some(rom) = &mut self.rom else { return };
        let chr_len = rom.chr_data().len();
        let total_tiles = chr_len / 16;

        let mut batch: Vec<(usize, [u8; 16])> = Vec::new();
        for dy in 0..n {
            for dx in 0..n {
                let dst_tile = tile_idx + dy * 16 + dx;
                if dst_tile >= total_tiles { continue; }
                let dst_offset = dst_tile * 16;
                let src_offset = (dy * n + dx) * 16;

                let saved: [u8; 16] = rom.chr_data()[dst_offset..dst_offset + 16]
                    .try_into().unwrap();
                batch.push((dst_offset, saved));
                rom.chr_data_mut()[dst_offset..dst_offset + 16]
                    .copy_from_slice(&src[src_offset..src_offset + 16]);
            }
        }
        if !batch.is_empty() {
            if self.undo_stack.len() >= 100 {
                self.undo_stack.remove(0);
            }
            self.undo_stack.push(batch);
        }
        self.is_modified = true;
        self.texture_dirty = true;
        self.status_msg = Some(format!("ペースト: {}×{} タイル", n, n));
    }
}