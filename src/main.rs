mod app;
mod chr;
mod nes;
mod palette;
mod png_import;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("R-CHR")
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "R-CHR",
        options,
        Box::new(|cc| {
            app::setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::RChrApp::default()))
        }),
    )
}