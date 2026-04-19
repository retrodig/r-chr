//! ドットエディタ
use eframe::egui;
use crate::io::chr::{decode_block, encode_dot};
use super::app::RChrApp;
use super::theme;

// ── アクション ──────────────────────────────────────────────────────

/// ドットエディタが発行するアクション（UI 描画とデータ変更を分離するため）
pub(super) enum EditorAction {
    /// ドットを塗る。push_undo=true のときは変更前のタイルを undo スタックへ
    PaintDot { tile_offset: usize, px: usize, py: usize, color: u8, push_undo: bool },
    /// スポイト：ドットの色を描画色として取得
    Eyedrop { color_idx: u8 },
    /// 描画色の選択
    SelectDrawingColor { color_idx: u8 },
}

// ── ドットエディタ ────────────────────────────────────────────────

impl RChrApp {
    pub(super) fn show_dot_editor(&mut self, ui: &mut egui::Ui) -> Option<EditorAction> {
        const ICON_NAMES: &[&str] = &[
            "pencil", "pencil_pattern", "slash",
            "square", "square_fill", "square_pattern",
            "circle", "circle_fill", "paint-bucket", "stamp",
        ];
        const ICON_BYTES: &[&[u8]] = &[
            include_bytes!("../../assets/icons/pencil.svg"),
            include_bytes!("../../assets/icons/pencil_pattern.svg"),
            include_bytes!("../../assets/icons/slash.svg"),
            include_bytes!("../../assets/icons/square.svg"),
            include_bytes!("../../assets/icons/square_fill.svg"),
            include_bytes!("../../assets/icons/square_pattern.svg"),
            include_bytes!("../../assets/icons/circle.svg"),
            include_bytes!("../../assets/icons/circle_fill.svg"),
            include_bytes!("../../assets/icons/paint-bucket.svg"),
            include_bytes!("../../assets/icons/stamp.svg"),
        ];
        let current_tool = self.drawing_tool;
        let mut clicked_tool: Option<usize> = None;

        let header_resp = egui::Frame::new()
            .fill(theme::COL_HEADER_BG)
            .inner_margin(egui::Margin { left: theme::PANEL_PADDING as _, right: theme::PANEL_PADDING as _, top: 6, bottom: 6 })
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for (i, (name, bytes)) in ICON_NAMES.iter().zip(ICON_BYTES.iter()).enumerate() {
                        let is_active = current_tool == i;
                        let tint = if is_active { theme::COL_HEADER_BG } else { theme::COL_TEXT };
                        let bg   = if is_active { theme::COL_ACTIVE_BG } else { egui::Color32::TRANSPARENT };
                        let img = egui::Image::from_bytes(
                            format!("bytes://icon_{name}.svg"),
                            bytes.to_vec(),
                        )
                        .fit_to_exact_size(egui::vec2(16.0, 16.0))
                        .tint(tint);
                        let cr = egui::CornerRadius::same(theme::CR_SM);
                        {
                            let v = ui.visuals_mut();
                            for state in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
                                state.bg_fill = bg;
                                state.weak_bg_fill = bg;
                                state.bg_stroke = egui::Stroke::NONE;
                                state.corner_radius = cr;
                            }
                        }
                        if ui.add(egui::Button::image(img).min_size(egui::vec2(theme::ICON_BTN_PX, theme::ICON_BTN_PX))).clicked() {
                            clicked_tool = Some(i);
                        }
                    }
                });
            });
        if let Some(t) = clicked_tool { self.drawing_tool = t; }
        {
            let r = header_resp.response.rect;
            ui.painter().hline(r.x_range(), r.bottom(), egui::Stroke::new(1.0, theme::COL_BORDER_DARK));
        }

        ui.add_space(theme::PANEL_PADDING);

        // ── タイルが未選択
        let Some(top_left_tile) = self.selected_tile else {
            ui.label("← タイルをクリックして選択");
            return None;
        };
        let Some(rom) = &self.rom else { return None };

        let n = self.focus_size.tile_count(); // ブロック 1 辺のタイル数
        let block_px = n * 8;                // ブロック 1 辺のドット数

        // フォーカスブロック全体をデコード（N×N タイル → block_px × block_px ドット）
        let block = decode_block(rom.chr_data(), top_left_tile, 16, n);

        // ── ドットキャンバス（左右16pxのpadding）
        let pad = theme::PANEL_PADDING;
        ui.style_mut().spacing.indent = pad;
        ui.visuals_mut().indent_has_left_vline = false;
        let canvas_resp = ui.indent("dot_canvas", |ui| {
            let available = ui.available_size() - egui::vec2(pad, 0.0); // 右pad分だけ引く
            let dot_size = (available.x.min(available.y) / block_px as f32).floor().max(1.0);
            let canvas = dot_size * block_px as f32;
            let alloc = ui.allocate_exact_size(egui::vec2(canvas, canvas), egui::Sense::click_and_drag());
            (alloc, dot_size)
        });
        let ((rect, response), dot_size) = canvas_resp.inner;
        let painter = ui.painter();

        for py in 0..block_px {
            for px in 0..block_px {
                let fill = self.dat_palette.color32(
                    self.selected_palette_set,
                    block[py][px] as usize,
                    &self.master_palette,
                );
                let dot_rect = egui::Rect::from_min_size(
                    egui::pos2(rect.left() + px as f32 * dot_size, rect.top() + py as f32 * dot_size),
                    egui::vec2(dot_size, dot_size),
                );
                painter.rect_filled(dot_rect, 0.0, fill);
                // ドットグリッド線（dot_size が十分大きいときのみ描画）
                if dot_size >= 4.0 {
                    painter.rect_stroke(
                        dot_rect, 0.0,
                        egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }

        // タイル境界線（フォーカスが 2 タイル以上のとき）
        if n > 1 && dot_size >= 2.0 {
            let tile_line_color = egui::Color32::from_rgba_unmultiplied(200, 200, 200, 80);
            let tile_step = dot_size * 8.0;
            for t in 1..n {
                let x = rect.left() + tile_step * t as f32;
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(1.0, tile_line_color),
                );
                let y = rect.top() + tile_step * t as f32;
                painter.line_segment(
                    [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                    egui::Stroke::new(1.0, tile_line_color),
                );
            }
        }

        // ── クリック / ドラッグ検出
        let Some(pos) = response.interact_pointer_pos() else { return None };
        let rel_x = pos.x - rect.left();
        let rel_y = pos.y - rect.top();
        if rel_x < 0.0 || rel_y < 0.0 { return None }

        let px = (rel_x / dot_size) as usize;
        let py = (rel_y / dot_size) as usize;
        if px >= block_px || py >= block_px { return None }

        // 右クリック → スポイト
        if response.secondary_clicked() {
            return Some(EditorAction::Eyedrop { color_idx: block[py][px] });
        }

        // クリック / ドラッグしたドットが属するタイルのオフセットを計算
        let block_col = px / 8;
        let block_row = py / 8;
        let top_col = top_left_tile % 16;
        let top_row = top_left_tile / 16;
        let tile_global = (top_row + block_row) * 16 + (top_col + block_col);
        let tile_offset = tile_global * 16;
        let dot_px = px % 8;
        let dot_py = py % 8;

        if tile_offset + 16 > rom.chr_data().len() { return None; }

        // 左クリック / 左ドラッグ → 描画
        let drag_started = response.drag_started_by(egui::PointerButton::Primary);
        let dragging     = response.dragged_by(egui::PointerButton::Primary);
        let clicked      = response.clicked_by(egui::PointerButton::Primary);

        if drag_started || dragging || clicked {
            let push_undo = drag_started || clicked;
            return Some(EditorAction::PaintDot {
                tile_offset, px: dot_px, py: dot_py,
                color: self.drawing_color_idx,
                push_undo,
            });
        }

        None
    }

    // ── アクション適用 ────────────────────────────────────────────

    pub(super) fn apply_action(&mut self, action: EditorAction) {
        match action {
            EditorAction::SelectDrawingColor { color_idx } => {
                self.drawing_color_idx = color_idx;
            }
            EditorAction::Eyedrop { color_idx } => {
                self.drawing_color_idx = color_idx;
            }
            EditorAction::PaintDot { tile_offset, px, py, color, push_undo } => {
                let chr_len = match &self.rom {
                    Some(r) => r.chr_data().len(),
                    None => return,
                };
                if tile_offset + 16 > chr_len { return }

                if push_undo {
                    // ドラッグ開始 or クリック: 新規バッチを開始
                    self.drag_undo_tiles.clear();
                    let saved: [u8; 16] = self.rom.as_ref().unwrap().chr_data()
                        [tile_offset..tile_offset + 16].try_into().unwrap();
                    self.push_undo_batch(vec![(tile_offset, saved)]);
                    self.drag_undo_tiles.insert(tile_offset);
                } else if !self.drag_undo_tiles.contains(&tile_offset) {
                    // ドラッグ中に初めて触れたタイル: 現在バッチに追記
                    let saved: [u8; 16] = self.rom.as_ref().unwrap().chr_data()
                        [tile_offset..tile_offset + 16].try_into().unwrap();
                    if let Some(batch) = self.undo_stack.last_mut() {
                        batch.push((tile_offset, saved));
                    }
                    self.drag_undo_tiles.insert(tile_offset);
                }

                if let Some(rom) = &mut self.rom {
                    encode_dot(&mut rom.chr_data_mut()[tile_offset..tile_offset + 16], px, py, color);
                }
                self.is_modified = true;
                self.texture_dirty = true;
            }
        }
    }

    /// アドレス入力フィールドの内容をパースして該当タイルへスクロール・フォーカス
    pub(super) fn jump_to_address(&mut self) {
        let raw = self.address_input.trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        if let Ok(addr) = usize::from_str_radix(raw, 16) {
            let total_tiles = self.rom.as_ref().map_or(0, |r| r.chr_data().len() / 16);
            if total_tiles > 0 {
                let tile_idx = (addr / 16).min(total_tiles.saturating_sub(1));
                let n = self.focus_size.tile_count();
                let snap_col = (tile_idx % 16 / n) * n;
                let snap_row = (tile_idx / 16 / n) * n;
                let snapped  = snap_row * 16 + snap_col;

                self.selected_tile       = Some(snapped);
                self.pending_scroll_addr = Some(snap_row * 0x100);
                self.address_input       = format!("{:06X}", snapped * 16);
                return;
            }
        }
        // パース失敗・範囲外の場合は現在値に戻す
        self.address_input = match self.selected_tile {
            Some(idx) => format!("{:06X}", idx * 16),
            None      => format!("{:06X}", self.scroll_addr),
        };
    }

    pub(super) fn push_undo_batch(&mut self, batch: Vec<(usize, [u8; 16])>) {
        if batch.is_empty() { return; }
        const UNDO_LIMIT: usize = 100;
        if self.undo_stack.len() >= UNDO_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(batch);
    }

    pub(super) fn do_undo(&mut self) {
        let Some(batch) = self.undo_stack.pop() else { return };
        let Some(rom) = &mut self.rom else { return };
        for (offset, saved) in batch {
            if offset + 16 <= rom.chr_data().len() {
                rom.chr_data_mut()[offset..offset + 16].copy_from_slice(&saved);
            }
        }
        self.texture_dirty = true;
    }
}