//! バンクビュー（CHR 全体ビュー）
use eframe::egui;
use super::app::RChrApp;
use super::theme;

// ── フォーカスサイズ ────────────────────────────────────────────────

/// バンクビューの選択ブロックサイズ（ピクセル単位 = タイル数 × 8）
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum FocusSize {
    S8   = 8,
    S16  = 16,
    S32  = 32,
    S64  = 64,
    S128 = 128,
}

impl FocusSize {
    /// 1 辺のタイル数（例: S32 → 4）
    pub(super) fn tile_count(self) -> usize { self as usize / 8 }

    /// UI ラベル文字列
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::S8   => "8",
            Self::S16  => "16",
            Self::S32  => "32",
            Self::S64  => "64",
            Self::S128 => "128",
        }
    }
}

// ── バンクビュー ──────────────────────────────────────────────────

impl RChrApp {
    pub(super) fn show_bank_view(&mut self, ui: &mut egui::Ui) {
        // 利用可能幅から表示スケールを計算（整数スケール）
        let available_w = ui.available_width();
        let scale = (available_w / 128.0).floor().max(1.0);
        let tile_px = 8.0 * scale; // 1 タイルの表示サイズ（px）

        // ── ツールバー（アドレスジャンプ + フォーカスサイズ）
        let toolbar_resp = egui::Frame::new()
            .fill(theme::COL_HEADER_BG)
            .inner_margin(egui::Margin { left: theme::PANEL_PADDING as _, right: theme::PANEL_PADDING as _, top: 5, bottom: 5 })
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(24.0);
                ui.visuals_mut().widgets.noninteractive.bg_stroke =
                    egui::Stroke::new(1.0, theme::COL_SEPARATOR);
                let _ = ui.horizontal(|ui| {
                    // アドレスジャンプ入力
                    ui.label(
                        egui::RichText::new("アドレス")
                            .font(theme::font_label())
                            .color(theme::COL_TEXT),
                    );
                    ui.visuals_mut().extreme_bg_color = theme::COL_INPUT_BG;
                    ui.visuals_mut().override_text_color = Some(theme::COL_TEXT);
                    let addr_resp = ui.add(
                        egui::TextEdit::singleline(&mut self.address_input)
                            .desired_width(70.0)
                            .font(theme::font_small())
                            .hint_text("001000"),
                    );

                    let enter_pressed = addr_resp.lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let button_clicked = {
                        let cr = egui::CornerRadius::same(theme::CR_BTN);
                        {
                            let v = ui.visuals_mut();
                            v.override_text_color = Some(egui::Color32::WHITE);
                            for state in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
                                state.bg_fill = theme::COL_BTN_BG;
                                state.weak_bg_fill = theme::COL_BTN_BG;
                                state.bg_stroke = egui::Stroke::new(1.0, theme::COL_BTN_BORDER);
                                state.corner_radius = cr;
                            }
                        }
                        ui.add(
                            egui::Button::new(
                                egui::RichText::new("移動")
                                    .font(theme::font_small()),
                            )
                            .min_size(egui::vec2(46.0, 20.0)),
                        )
                        .clicked()
                    };

                    if enter_pressed || button_clicked {
                        self.jump_to_address();
                        ui.ctx().memory_mut(|m| m.surrender_focus(addr_resp.id));
                    } else if !addr_resp.has_focus() {
                        self.address_input = match self.selected_tile {
                            Some(idx) => format!("{:06X}", idx * 16),
                            None      => format!("{:06X}", self.scroll_addr),
                        };
                    }

                    ui.separator();

                    // フォーカスサイズ切り替えボタン
                    for &fs in &[FocusSize::S8, FocusSize::S16, FocusSize::S32, FocusSize::S64, FocusSize::S128] {
                        let is_active = self.focus_size == fs;
                        let (fg, bg) = if is_active {
                            (theme::COL_HEADER_BG, theme::COL_ACTIVE_BG)
                        } else {
                            (theme::COL_ACTIVE_BG, egui::Color32::TRANSPARENT)
                        };
                        let label = egui::RichText::new(fs.label())
                            .font(theme::font_small_bold())
                            .color(fg);
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
                        let btn = egui::Button::new(label)
                            .min_size(egui::vec2(theme::ICON_BTN_PX, theme::ICON_BTN_PX))
                            .frame(true);
                        if ui.add(btn).clicked() {
                            self.focus_size = fs;
                        }
                    }
                }); // horizontal
            }); // Frame
        {
            let r = toolbar_resp.response.rect;
            ui.painter().hline(r.x_range(), r.bottom(), egui::Stroke::new(1.0, theme::COL_BORDER_DARK));
        }

        // ── スクロールビューの準備（self からコピーが必要な値を事前抽出）
        let total_tiles = self.rom.as_ref().map_or(0, |r| r.chr_data().len() / 16);
        if total_tiles == 0 { return; }
        let total_rows = (total_tiles + 15) / 16;
        let texture_id = match &self.bank_texture {
            Some(t) => t.id(),
            None => return,
        };
        let n = self.focus_size.tile_count();
        let selected_tile_snap = self.selected_tile;
        let display_w = 128.0 * scale;
        let display_h = total_rows as f32 * tile_px;

        // ── ScrollArea のスクロール位置をアドレスジャンプで制御
        let mut scroll_area = egui::ScrollArea::vertical()
            .id_salt("bank_scroll")
            .auto_shrink([false, false]);

        if let Some(addr) = self.pending_scroll_addr.take() {
            let row = addr / 0x100;
            scroll_area = scroll_area.vertical_scroll_offset(row as f32 * tile_px);
        }

        let frame_out = egui::Frame::new()
            .inner_margin(egui::Margin { left: theme::PANEL_PADDING as _, right: theme::PANEL_PADDING as _, top: theme::PANEL_PADDING as _, bottom: 0 })
            .show(ui, |ui| { scroll_area.show(ui, |ui| {
                let (rect, response) = ui.allocate_exact_size(
                    egui::vec2(display_w, display_h),
                    egui::Sense::click(),
                );

                // テクスチャ描画
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter().image(texture_id, rect, uv, egui::Color32::WHITE);

                let painter = ui.painter();

                // マイナーグリッド（タイル単位）
                let minor = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25);
                for col in 1..16 {
                    let x = rect.left() + tile_px * col as f32;
                    painter.line_segment(
                        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                        egui::Stroke::new(0.5, minor),
                    );
                }
                for row in 1..total_rows {
                    let y = rect.top() + tile_px * row as f32;
                    painter.line_segment(
                        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                        egui::Stroke::new(0.5, minor),
                    );
                }

                // メジャーグリッド（フォーカスブロック単位）
                if n > 1 {
                    let major = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 70);
                    for col in (0..=16).step_by(n) {
                        let x = rect.left() + tile_px * col as f32;
                        painter.line_segment(
                            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                            egui::Stroke::new(1.0, major),
                        );
                    }
                    for row in (0..=total_rows).step_by(n) {
                        let y = rect.top() + tile_px * row as f32;
                        painter.line_segment(
                            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                            egui::Stroke::new(1.0, major),
                        );
                    }
                }

                // タイルクリック検出（フォーカスグリッドに snap）
                let new_tile = if response.clicked() {
                    response.interact_pointer_pos().and_then(|pos| {
                        let rel_x = pos.x - rect.left();
                        let rel_y = pos.y - rect.top();
                        if rel_x < 0.0 || rel_y < 0.0 { return None; }
                        let col = (rel_x / tile_px) as usize;
                        let row = (rel_y / tile_px) as usize;
                        if col >= 16 { return None; }
                        let global_tile = row * 16 + col;
                        (global_tile < total_tiles).then_some(global_tile)
                    })
                } else {
                    None
                };

                // 選択ブロックのハイライト
                if let Some(tile_idx) = selected_tile_snap {
                    let t_row = tile_idx / 16;
                    let t_col = tile_idx % 16;
                    let bx = rect.left() + t_col as f32 * tile_px;
                    let by_ = rect.top()  + t_row as f32 * tile_px;
                    let bs  = tile_px * n as f32;
                    let hl  = egui::Rect::from_min_size(egui::pos2(bx, by_), egui::vec2(bs, bs));
                    painter.rect_stroke(
                        hl, 0.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 80, 80)),
                        egui::StrokeKind::Outside,
                    );
                }

                new_tile
            }) }); // scroll_area.show / Frame

        let scroll_out = frame_out.inner;
        // スクロール結果を self に反映
        if let Some(tile) = scroll_out.inner {
            self.selected_tile = Some(tile);
        }
        let scroll_y = scroll_out.state.offset.y;
        self.scroll_addr = (scroll_y / tile_px) as usize * 0x100;
        // 矢印キーのスクロール判定用にビューポート情報を保存
        self.scroll_top_row = (scroll_y / tile_px) as usize;
        self.visible_tile_rows = (scroll_out.inner_rect.height() / tile_px).floor() as usize;
    }
}