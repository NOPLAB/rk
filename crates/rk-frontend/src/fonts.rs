//! Font configuration for the application

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Configure application fonts with Noto Sans as default
pub fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // Add Noto Sans CJK JP as the primary font (supports Japanese and Latin)
    fonts.font_data.insert(
        "noto_sans_jp".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/NotoSansJP-Regular.otf"
        ))),
    );

    // Add Noto Sans Symbols 2 for Unicode symbols (arrows, geometric shapes, etc.)
    fonts.font_data.insert(
        "noto_symbols_2".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/NotoSansSymbols2-Regular.ttf"
        ))),
    );

    // Set Noto Sans JP as the primary font for Proportional family
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "noto_sans_jp".to_owned());
        family.push("noto_symbols_2".to_owned());
    }

    // Also add to Monospace for consistency
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push("noto_sans_jp".to_owned());
        family.push("noto_symbols_2".to_owned());
    }

    ctx.set_fonts(fonts);
}
