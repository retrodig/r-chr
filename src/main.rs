mod app;
mod chr;
mod nes;
mod palette;
mod png_import;
#[cfg(target_os = "macos")]
mod native_menu;

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
            // macOS: NSApp が初期化された後（ここ）でネイティブメニューを構築する
            #[cfg(target_os = "macos")]
            native_menu::init();

            app::setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::RChrApp::default()))
        }),
    )
}