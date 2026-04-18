mod io;
mod model;
mod editor;
#[cfg(target_os = "macos")]
mod native_menu;

use eframe::egui;

fn load_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .expect("icon.png の読み込みに失敗")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData { rgba: image.into_raw(), width, height }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("R-CHR")
            .with_inner_size([1200.0, 720.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "R-CHR",
        options,
        Box::new(|cc| {
            // macOS: NSApp が初期化された後（ここ）でネイティブメニューを構築し外観を設定する
            #[cfg(target_os = "macos")]
            {
                native_menu::init();
                native_menu::set_app_appearance(true); // デフォルトはダークモード
            }

            egui_extras::install_image_loaders(&cc.egui_ctx);
            editor::setup::setup_fonts(&cc.egui_ctx);
            Ok(Box::new(editor::app::RChrApp::default()))
        }),
    )
}