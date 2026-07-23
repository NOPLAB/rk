//! Dark theme visuals for egui

use egui::{Color32, CornerRadius, Shadow, Stroke, Visuals, style};

use super::palette;

/// Create dark theme visuals
pub fn visuals() -> Visuals {
    let mut v = Visuals::dark();

    // Background colors
    v.panel_fill = palette::BG_PANEL;
    v.window_fill = palette::BG_ELEVATED;
    v.extreme_bg_color = palette::BG_BASE;
    v.faint_bg_color = palette::BG_INPUT;

    // Selection
    v.selection.bg_fill = palette::ACCENT_SUBTLE;
    v.selection.stroke = Stroke::new(1.0, palette::ACCENT_PRIMARY);

    // Hyperlink
    v.hyperlink_color = palette::ACCENT_PRIMARY;

    // Text colors
    v.override_text_color = Some(palette::TEXT_PRIMARY);

    // Widget colors
    v.widgets.noninteractive.bg_fill = palette::BG_INPUT;
    v.widgets.noninteractive.weak_bg_fill = palette::BG_PANEL;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette::BORDER_SUBTLE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette::TEXT_SECONDARY);
    v.widgets.noninteractive.corner_radius = CornerRadius::same(4);

    v.widgets.inactive.bg_fill = palette::BG_INPUT;
    v.widgets.inactive.weak_bg_fill = palette::BG_PANEL;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, palette::BORDER_SUBTLE);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, palette::TEXT_PRIMARY);
    v.widgets.inactive.corner_radius = CornerRadius::same(4);

    v.widgets.hovered.bg_fill = palette::BG_HOVER;
    v.widgets.hovered.weak_bg_fill = palette::BG_HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, palette::BORDER_NORMAL);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, palette::TEXT_PRIMARY);
    v.widgets.hovered.corner_radius = CornerRadius::same(4);

    v.widgets.active.bg_fill = palette::ACCENT_PRIMARY;
    v.widgets.active.weak_bg_fill = palette::BG_HOVER;
    v.widgets.active.bg_stroke = Stroke::new(1.0, palette::ACCENT_PRIMARY);
    v.widgets.active.fg_stroke = Stroke::new(2.0, palette::TEXT_PRIMARY);
    v.widgets.active.corner_radius = CornerRadius::same(4);

    v.widgets.open.bg_fill = palette::BG_ELEVATED;
    v.widgets.open.weak_bg_fill = palette::BG_ELEVATED;
    v.widgets.open.bg_stroke = Stroke::new(1.0, palette::BORDER_NORMAL);
    v.widgets.open.fg_stroke = Stroke::new(1.0, palette::TEXT_PRIMARY);
    v.widgets.open.corner_radius = CornerRadius::same(4);

    // Window styling
    v.window_corner_radius = CornerRadius::same(6);
    v.window_shadow = Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(80),
    };
    v.window_stroke = Stroke::new(1.0, palette::BORDER_SUBTLE);

    // Popup styling
    v.popup_shadow = Shadow {
        offset: [0, 2],
        blur: 8,
        spread: 0,
        color: Color32::from_black_alpha(60),
    };

    // Menu styling
    v.menu_corner_radius = CornerRadius::same(4);

    // Striped table background
    v.striped = true;

    // Slider rail
    v.slider_trailing_fill = true;

    // Handle styling
    v.handle_shape = style::HandleShape::Rect { aspect_ratio: 0.5 };

    // Indent
    v.indent_has_left_vline = true;

    // Resize corner
    v.resize_corner_size = 12.0;

    // Clip rectangles
    v.clip_rect_margin = 3.0;

    // Button frame
    v.button_frame = true;

    // Collapsing header frame
    v.collapsing_header_frame = false;

    // Text cursor
    v.text_cursor.stroke = Stroke::new(2.0, palette::TEXT_PRIMARY);

    // Image loading spinners
    v.image_loading_spinners = true;

    // Numeric color space
    v.numeric_color_space = style::NumericColorSpace::GammaByte;

    v
}
