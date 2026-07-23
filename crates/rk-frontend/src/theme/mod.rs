//! Theme module for UI styling
//!
//! This module provides a unified theming system for the application,
//! allowing hot-reload of themes without application restart.

mod dark;
mod light;
pub mod palette;

use crate::config::{SharedConfig, UiTheme};

/// Apply the current theme to egui context
pub fn apply_theme(ctx: &egui::Context, config: &SharedConfig) {
    let theme = config.read().config().ui.theme;
    let visuals = match theme {
        UiTheme::Dark => dark::visuals(),
        UiTheme::Light => light::visuals(),
    };
    ctx.set_visuals(visuals);
}

/// Create an overlay frame with standard styling
pub fn overlay_frame(is_dark: bool) -> egui::Frame {
    if is_dark {
        egui::Frame::popup(&egui::Style::default())
            .fill(palette::overlay_bg(220))
            .corner_radius(4.0)
            .stroke(egui::Stroke::new(1.0, palette::BORDER_NORMAL))
    } else {
        egui::Frame::popup(&egui::Style::default())
            .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 240))
            .corner_radius(4.0)
            .stroke(egui::Stroke::new(1.0, palette::light::BORDER_NORMAL))
    }
}

/// Get whether the current theme is dark
pub fn is_dark_theme(config: &SharedConfig) -> bool {
    config.read().config().ui.theme == UiTheme::Dark
}
