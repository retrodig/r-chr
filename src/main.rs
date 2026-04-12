use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("r-chr")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "r-chr",
        options,
        Box::new(|_cc| Ok(Box::new(RChrApp::default()))),
    )
}

#[derive(Default)]
struct RChrApp {}

impl eframe::App for RChrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("r-chr");
            ui.label("NES CHR エディタ");
        });
    }
}