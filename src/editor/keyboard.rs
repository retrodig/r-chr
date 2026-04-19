use super::app::RChrApp;

// ── キーボード操作 ────────────────────────────────────────────────

impl RChrApp {
    pub(super) fn handle_keyboard(&mut self, ctx: &egui::Context) {
        let chr_empty = self.rom.as_ref().map_or(true, |r| r.chr_data().is_empty());
        if chr_empty { return }

        let mut new_palette_set: Option<usize> = None;
        let mut do_undo = false;
        let mut do_copy = false;
        let mut do_paste = false;
        let mut do_save = false;
        let mut do_save_as = false;
        let mut d_col: i32 = 0; // 矢印キーによる列移動量（ブロック単位）
        let mut d_row: i32 = 0; // 矢印キーによる行移動量（ブロック単位）

        ctx.input(|i| {
            let cmd = i.modifiers.ctrl || i.modifiers.mac_cmd;

            // macOS では NSMenu が Cmd+Z / Cmd+S をインターセプトするため egui には届かない。
            // NSMenu が機能しない場合のフォールバックとして egui 側にも残す（二重実行にはならない）。
            if cmd && i.key_pressed(egui::Key::Z) {
                do_undo = true;
            } else if i.key_pressed(egui::Key::Z) {
                new_palette_set = Some(0);
            }
            if cmd && i.key_pressed(egui::Key::C) {
                do_copy = true;
            } else if !cmd && i.key_pressed(egui::Key::C) {
                new_palette_set = Some(2);
            }
            if cmd && i.key_pressed(egui::Key::V) {
                do_paste = true;
            } else if !cmd && i.key_pressed(egui::Key::V) {
                new_palette_set = Some(3);
            }
            if !cmd && i.key_pressed(egui::Key::X) { new_palette_set = Some(1); }

            if cmd && i.key_pressed(egui::Key::S) {
                if i.modifiers.shift { do_save_as = true; } else { do_save = true; }
            }

            // 矢印キー（フォーカスブロック単位で移動）
            if i.key_pressed(egui::Key::ArrowRight) { d_col += 1; }
            if i.key_pressed(egui::Key::ArrowLeft)  { d_col -= 1; }
            if i.key_pressed(egui::Key::ArrowDown)  { d_row += 1; }
            if i.key_pressed(egui::Key::ArrowUp)    { d_row -= 1; }
        });

        if let Some(set) = new_palette_set {
            self.selected_palette_set = set;
            self.texture_dirty = true;
        }
        if do_undo  { self.do_undo(); }
        if do_copy  { self.copy_tiles(); }
        if do_paste { self.paste_tiles(); }
        if do_save {
            if let Err(e) = self.save_file() { self.error_msg = Some(e); }
        }
        if do_save_as {
            if let Err(e) = self.save_file_as() { self.error_msg = Some(e); }
        }

        // 矢印キーによるタイル選択移動（起点は常に 1 タイル単位）
        if d_col != 0 || d_row != 0 {
            let total_tiles = self.rom.as_ref().map_or(0, |r| r.chr_data().len() / 16);
            if total_tiles == 0 { return; }
            let total_rows = ((total_tiles + 15) / 16) as i32;

            let current = self.selected_tile.unwrap_or(0);
            let cur_col = (current % 16) as i32;
            let cur_row = (current / 16) as i32;

            // 1 タイル単位で移動し、端でクランプ
            let new_col = (cur_col + d_col).clamp(0, 15) as usize;
            let new_row = (cur_row + d_row).clamp(0, total_rows - 1) as usize;
            let new_tile = new_row * 16 + new_col;

            if new_tile < total_tiles {
                self.selected_tile = Some(new_tile);
                // 選択タイルが可視範囲外に出た場合のみスクロール
                let visible_end = self.scroll_top_row + self.visible_tile_rows.max(1);
                if new_row < self.scroll_top_row {
                    // 上に出た → 選択行を先頭に
                    self.pending_scroll_addr = Some(new_row * 0x100);
                } else if new_row >= visible_end {
                    // 下に出た → 選択行が末尾に来るよう調整
                    let start_row = new_row + 1 - self.visible_tile_rows.max(1);
                    self.pending_scroll_addr = Some(start_row * 0x100);
                }
                // 可視範囲内なら scroll しない（チラツキ防止）
            }
        }
    }
}