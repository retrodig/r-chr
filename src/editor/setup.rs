//! アプリ起動時の初期化処理

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Noto Sans JP Regular — 本文フォント
    fonts.font_data.insert(
        "noto_regular".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Noto_Sans_JP/static/NotoSansJP-Regular.ttf"
        )).into(),
    );
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "noto_regular".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap().push("noto_regular".to_owned());

    // Noto Sans JP Bold — bold_font named family
    fonts.font_data.insert(
        "noto_bold".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../assets/fonts/Noto_Sans_JP/static/NotoSansJP-Bold.ttf"
        )).into(),
    );
    fonts.families.insert(
        egui::FontFamily::Name("bold_font".into()),
        vec!["noto_bold".to_owned()],
    );

    ctx.set_fonts(fonts);
}