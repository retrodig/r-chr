//! UI テーマ定数（カラー・フォント・サイズ）
use eframe::egui;

// ── カラー ─────────────────────────────────────────────────────────

/// 基本テキストカラー
pub(super) const COL_TEXT: egui::Color32 = egui::Color32::from_rgb(0xBF, 0xBF, 0xBF);

/// ヘッダーバー背景 / アクティブ要素の前景色
pub(super) const COL_HEADER_BG: egui::Color32 = egui::Color32::from_rgb(0x26, 0x26, 0x26);

/// 暗いボーダー（パネル外枠・ヘッダー下辺）
pub(super) const COL_BORDER_DARK: egui::Color32 = egui::Color32::from_rgb(0x0C, 0x0C, 0x0C);

/// 右パネル背景
pub(super) const COL_PANEL_BG: egui::Color32 = egui::Color32::from_rgb(0x28, 0x28, 0x28);

/// パネル内セパレーター色
pub(super) const COL_SEPARATOR: egui::Color32 = egui::Color32::from_rgb(0x48, 0x48, 0x48);

/// パレットスウォッチ枠線
pub(super) const COL_SWATCH_BORDER: egui::Color32 = egui::Color32::from_rgb(0x50, 0x50, 0x50);

/// アクティブボタン背景
pub(super) const COL_ACTIVE_BG: egui::Color32 = egui::Color32::from_rgb(0xB6, 0xB6, 0xB6);

/// テキスト入力フィールド背景
pub(super) const COL_INPUT_BG: egui::Color32 = egui::Color32::from_rgb(0x33, 0x33, 0x33);

/// 移動ボタン背景
pub(super) const COL_BTN_BG: egui::Color32 = egui::Color32::from_rgb(0x5E, 0x5E, 0x5E);

/// 移動ボタン枠線
pub(super) const COL_BTN_BORDER: egui::Color32 = egui::Color32::from_rgb(0x20, 0x20, 0x20);

// ── フォント ───────────────────────────────────────────────────────

/// ボールドフォントファミリー名
pub(super) const BOLD_FONT: &str = "bold_font";

/// セクションラベルフォントサイズ (bold_font)
pub(super) const FONT_SIZE_LABEL: f32 = 15.0;

/// パレットセットインデックスラベルフォントサイズ (bold_font)
pub(super) const FONT_SIZE_PALETTE_IDX: f32 = 14.0;

/// 小テキスト・ボタンフォントサイズ (Proportional / bold_font)
pub(super) const FONT_SIZE_SMALL: f32 = 13.0;

// ── フォント ID ヘルパー ───────────────────────────────────────────

/// セクションラベル用 FontId（bold, 15px）
pub(super) fn font_label() -> egui::FontId {
    egui::FontId::new(FONT_SIZE_LABEL, egui::FontFamily::Name(BOLD_FONT.into()))
}

/// パレットセットインデックス用 FontId（bold, 14px）
pub(super) fn font_palette_idx() -> egui::FontId {
    egui::FontId::new(FONT_SIZE_PALETTE_IDX, egui::FontFamily::Name(BOLD_FONT.into()))
}

/// 小テキスト通常用 FontId（Proportional, 13px）
pub(super) fn font_small() -> egui::FontId {
    egui::FontId::new(FONT_SIZE_SMALL, egui::FontFamily::Proportional)
}

/// 小テキストボールド用 FontId（bold, 13px）
pub(super) fn font_small_bold() -> egui::FontId {
    egui::FontId::new(FONT_SIZE_SMALL, egui::FontFamily::Name(BOLD_FONT.into()))
}

// ── サイズ ─────────────────────────────────────────────────────────

/// パレットスウォッチ 1 辺の長さ (px)
pub(super) const SWATCH_PX: f32 = 24.0;

/// アイコンボタン 1 辺の長さ (px)
pub(super) const ICON_BTN_PX: f32 = 22.0;

/// パネル左右パディング (px)
pub(super) const PANEL_PADDING: f32 = 16.0;

// ── 角丸半径 ───────────────────────────────────────────────────────

/// スウォッチ・アイコンボタンの標準角丸半径
pub(super) const CR_SM: u8 = 4;

/// 移動ボタンの角丸半径
pub(super) const CR_BTN: u8 = 5;