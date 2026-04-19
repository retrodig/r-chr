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
    /// 線ツール：確定したドット列 (tile_offset, px_in_tile, py_in_tile) を一括適用
    ApplyLine { pixels: Vec<(usize, usize, usize)> },
}

// ── ユーティリティ ────────────────────────────────────────────────

/// Bresenham の直線アルゴリズム（ドット座標リストを返す）
fn bresenham(x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<(usize, usize)> {
    let (mut x, mut y) = (x0 as i32, y0 as i32);
    let (x1, y1) = (x1 as i32, y1 as i32);
    let dx = (x1 - x).abs();
    let dy = -(y1 - y).abs();
    let sx: i32 = if x < x1 { 1 } else { -1 };
    let sy: i32 = if y < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut pts = Vec::new();
    loop {
        pts.push((x as usize, y as usize));
        if x == x1 && y == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
    pts
}

/// 矩形ドット生成（kind: 3=枠線, 4=塗り, 5=パターン）
fn rect_dots(sx: usize, sy: usize, ex: usize, ey: usize, kind: usize) -> Vec<(usize, usize)> {
    let (x0, x1) = (sx.min(ex), sx.max(ex));
    let (y0, y1) = (sy.min(ey), sy.max(ey));
    let parity = (sx + sy) % 2;
    let mut pts = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let draw = match kind {
                3 => x == x0 || x == x1 || y == y0 || y == y1,
                4 => true,
                5 => (x + y) % 2 == parity,
                _ => false,
            };
            if draw { pts.push((x, y)); }
        }
    }
    pts
}

/// ツール種別からドット座標列を生成（線・矩形共通）
fn shape_dots(sx: usize, sy: usize, ex: usize, ey: usize, tool: usize) -> Vec<(usize, usize)> {
    match tool {
        2 => bresenham(sx, sy, ex, ey),
        3 | 4 | 5 => rect_dots(sx, sy, ex, ey, tool),
        _ => vec![],
    }
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

        let n = self.focus_size.tile_count(); // ブロック 1 辺のタイル数
        let block_px = n * 8;                // ブロック 1 辺のドット数

        // フォーカスブロック全体をデコード（rom の借用をここで解放）
        let (block, chr_len) = {
            let Some(rom) = self.rom.as_ref() else { return None };
            (decode_block(rom.chr_data(), top_left_tile, 16, n), rom.chr_data().len())
        };

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

        // ── 線・矩形ツール: プレビュー描画（ドラッグ中、tool=2-5）
        if matches!(self.drawing_tool, 2..=5) {
            if let Some((sx, sy)) = self.line_start_dot {
                let cur = response.interact_pointer_pos().or_else(|| response.hover_pos());
                if let Some(p) = cur {
                    let rx = p.x - rect.left();
                    let ry = p.y - rect.top();
                    if rx >= 0.0 && ry >= 0.0 {
                        let cx = ((rx / dot_size) as usize).min(block_px.saturating_sub(1));
                        let cy = ((ry / dot_size) as usize).min(block_px.saturating_sub(1));
                        let preview_color = self.dat_palette.color32(
                            self.selected_palette_set,
                            self.drawing_color_idx as usize,
                            &self.master_palette,
                        );
                        for (dx, dy) in shape_dots(sx, sy, cx, cy, self.drawing_tool) {
                            if dx >= block_px || dy >= block_px { continue; }
                            let dot_rect = egui::Rect::from_min_size(
                                egui::pos2(rect.left() + dx as f32 * dot_size, rect.top() + dy as f32 * dot_size),
                                egui::vec2(dot_size, dot_size),
                            );
                            painter.rect_filled(dot_rect, 0.0, preview_color);
                        }
                    }
                }
            }
        }

        // ── 線・矩形ツール: ドラッグ終了検出（tool=2-5）
        if matches!(self.drawing_tool, 2..=5) && self.line_start_dot.is_some() {
            let just_released = ui.ctx().input(|i| i.pointer.button_released(egui::PointerButton::Primary));
            if just_released {
                let end = ui.ctx().input(|i| i.pointer.hover_pos());
                let top_col = top_left_tile % 16;
                let top_row = top_left_tile / 16;
                let (sx, sy) = self.line_start_dot.take().unwrap();
                let (ex, ey) = end.and_then(|p| {
                    let rx = p.x - rect.left();
                    let ry = p.y - rect.top();
                    (rx >= 0.0 && ry >= 0.0).then(|| {
                        let ex = ((rx / dot_size) as usize).min(block_px.saturating_sub(1));
                        let ey = ((ry / dot_size) as usize).min(block_px.saturating_sub(1));
                        (ex, ey)
                    })
                }).unwrap_or((sx, sy));
                let tool = self.drawing_tool;
                let pixels = shape_dots(sx, sy, ex, ey, tool)
                    .into_iter()
                    .filter_map(|(dx, dy)| {
                        if dx >= block_px || dy >= block_px { return None; }
                        let off = ((top_row + dy / 8) * 16 + (top_col + dx / 8)) * 16;
                        (off + 16 <= chr_len).then_some((off, dx % 8, dy % 8))
                    })
                    .collect();
                return Some(EditorAction::ApplyLine { pixels });
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

        if tile_offset + 16 > chr_len { return None; }

        // ── 線・矩形ツール: ドラッグ開始 / 中 / クリック（tool=2-5）
        if matches!(self.drawing_tool, 2..=5) {
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.line_start_dot = Some((px, py));
                return None;
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                return None; // プレビューは上で描画済み
            }
            // クリック（ドラッグなし）: 1 点として確定
            if response.clicked_by(egui::PointerButton::Primary) {
                let pixels = vec![(tile_offset, dot_px, dot_py)];
                return Some(EditorAction::ApplyLine { pixels });
            }
            return None;
        }

        // 左クリック / 左ドラッグ → 描画（ペン系ツール）
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
            EditorAction::ApplyLine { pixels } => {
                self.apply_line(pixels);
            }
            EditorAction::PaintDot { tile_offset, px, py, color, push_undo } => {
                let chr_len = match &self.rom {
                    Some(r) => r.chr_data().len(),
                    None => return,
                };
                if tile_offset + 16 > chr_len { return }

                // ペン（パターン）ツール: グローバルピクセル座標でチェッカーボードパリティを計算
                let tile_global = tile_offset / 16;
                let global_x = (tile_global % 16) * 8 + px;
                let global_y = (tile_global / 16) * 8 + py;
                let parity = ((global_x + global_y) % 2) as u8;

                if push_undo {
                    // ドラッグ開始 or クリック: 起点パリティを記録して新規バッチ開始
                    self.drag_pattern_parity = parity;
                    self.drag_undo_tiles.clear();
                    let saved: [u8; 16] = self.rom.as_ref().unwrap().chr_data()
                        [tile_offset..tile_offset + 16].try_into().unwrap();
                    self.push_undo_batch(vec![(tile_offset, saved)]);
                    self.drag_undo_tiles.insert(tile_offset);
                } else {
                    // ペン（パターン）: 起点と異なるパリティのドットはスキップ
                    if self.drawing_tool == 1 && parity != self.drag_pattern_parity {
                        return;
                    }
                    // ドラッグ中に初めて触れたタイル: 現在バッチに追記
                    if !self.drag_undo_tiles.contains(&tile_offset) {
                        let saved: [u8; 16] = self.rom.as_ref().unwrap().chr_data()
                            [tile_offset..tile_offset + 16].try_into().unwrap();
                        if let Some(batch) = self.undo_stack.last_mut() {
                            batch.push((tile_offset, saved));
                        }
                        self.drag_undo_tiles.insert(tile_offset);
                    }
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

    pub(super) fn apply_line(&mut self, pixels: Vec<(usize, usize, usize)>) {
        if pixels.is_empty() { return; }
        let chr_len = match &self.rom {
            Some(r) => r.chr_data().len(),
            None => return,
        };
        // undo バッチ: 影響タイルを重複なく保存
        let mut seen = std::collections::HashSet::new();
        let mut batch: Vec<(usize, [u8; 16])> = Vec::new();
        for &(off, _, _) in &pixels {
            if off + 16 <= chr_len && seen.insert(off) {
                let saved: [u8; 16] = self.rom.as_ref().unwrap().chr_data()
                    [off..off + 16].try_into().unwrap();
                batch.push((off, saved));
            }
        }
        self.push_undo_batch(batch);
        // ドット書き込み
        let color = self.drawing_color_idx;
        for (off, px, py) in pixels {
            if off + 16 <= chr_len {
                if let Some(rom) = &mut self.rom {
                    encode_dot(&mut rom.chr_data_mut()[off..off + 16], px, py, color);
                }
            }
        }
        self.is_modified = true;
        self.texture_dirty = true;
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