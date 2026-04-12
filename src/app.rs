use eframe::egui;
use crate::nes::{NesRom, parse_nes};

/// 起動時に日本語フォントをセットアップする
pub fn setup_fonts(ctx: &egui::Context) {
    let font_path = "/Library/Fonts/Microsoft/MS Gothic.ttf";
    if let Ok(font_data) = std::fs::read(font_path) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "jp_font".to_owned(),
            egui::FontData::from_owned(font_data).into(),
        );
        // 既存フォントの後ろに追加（日本語グリフのフォールバックとして使う）
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .push("jp_font".to_owned());
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("jp_font".to_owned());
        ctx.set_fonts(fonts);
    }
}

pub struct RChrApp {
    /// 読み込んだ ROM（None = 未読み込み）
    rom: Option<NesRom>,
    /// 表示中のファイル名
    file_name: Option<String>,
    /// エラーメッセージ
    error_msg: Option<String>,
}

impl Default for RChrApp {
    fn default() -> Self {
        Self {
            rom: None,
            file_name: None,
            error_msg: None,
        }
    }
}

impl eframe::App for RChrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // メニューバー
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("ファイル", |ui| {
                    if ui.button("開く…").clicked() {
                        self.open_file();
                        ui.close_menu();
                    }
                });
            });
        });

        // ステータスバー
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            if let Some(name) = &self.file_name {
                ui.label(format!("ファイル: {name}"));
            } else {
                ui.label("ファイルを開いてください");
            }
        });

        // メインパネル
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(err) = &self.error_msg.clone() {
                ui.colored_label(egui::Color32::RED, format!("エラー: {err}"));
                if ui.button("閉じる").clicked() {
                    self.error_msg = None;
                }
                return;
            }

            match &self.rom {
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.label("ファイルメニューから NES ROM を開いてください");
                    });
                }
                Some(rom) => {
                    self.show_rom_info(ui, rom);
                }
            }
        });
    }
}

impl RChrApp {
    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("NES ROM", &["nes"])
            .add_filter("すべてのファイル", &["*"])
            .pick_file()
        else {
            return;
        };

        self.file_name = Some(
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        );

        match std::fs::read(&path) {
            Err(e) => {
                self.error_msg = Some(format!("読み込み失敗: {e}"));
                self.rom = None;
            }
            Ok(data) => match parse_nes(&data) {
                Err(e) => {
                    self.error_msg = Some(e.to_string());
                    self.rom = None;
                }
                Ok(rom) => {
                    self.error_msg = None;
                    self.rom = Some(rom);
                }
            },
        }
    }

    fn show_rom_info(&self, ui: &mut egui::Ui, rom: &NesRom) {
        let h = &rom.header;

        egui::Grid::new("rom_info")
            .num_columns(2)
            .striped(true)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.strong("マッパー");
                ui.label(format!("{}", h.mapper));
                ui.end_row();

                ui.strong("PRG-ROM");
                ui.label(format!(
                    "{} バンク  ({} KB)",
                    h.prg_rom_banks,
                    h.prg_rom_size() / 1024
                ));
                ui.end_row();

                ui.strong("CHR-ROM");
                if h.chr_rom_banks == 0 {
                    ui.colored_label(egui::Color32::YELLOW, "CHR-RAM（ROM なし）");
                } else {
                    ui.label(format!(
                        "{} バンク  ({} KB)",
                        h.chr_rom_banks,
                        h.chr_rom_size() / 1024
                    ));
                }
                ui.end_row();

                ui.strong("ミラーリング");
                ui.label(if h.vertical_mirroring { "垂直" } else { "水平" });
                ui.end_row();

                ui.strong("バッテリー");
                ui.label(if h.has_battery { "あり" } else { "なし" });
                ui.end_row();

                ui.strong("ファイルサイズ");
                ui.label(format!(
                    "{} バイト ({} KB)",
                    rom.prg_rom.len() + rom.chr_rom.len(),
                    (rom.prg_rom.len() + rom.chr_rom.len()) / 1024
                ));
                ui.end_row();
            });

        ui.add_space(12.0);

        if h.chr_rom_banks == 0 {
            ui.label("※ CHR-RAM のため、CHR データは ROM に含まれていません。");
        } else {
            ui.label(format!(
                "CHR データ: {} バイト読み込み済み",
                rom.chr_rom.len()
            ));
        }
    }
}