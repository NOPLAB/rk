//! Constraint icon overlay for sketch mode
//!
//! Renders constraint icons as egui overlay on the viewport when in sketch mode.

use glam::{Mat4, Vec4};
use rk_renderer::{Camera, ConstraintIconData};

use crate::theme::palette;

/// Render constraint icons as egui overlay
pub fn render_constraint_icons(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    constraint_icons: &[ConstraintIconData],
    sketch_transform: Mat4,
    camera: &Camera,
    viewport_size: egui::Vec2,
) {
    if constraint_icons.is_empty() {
        return;
    }

    let view_proj = camera.projection_matrix() * camera.view_matrix();

    for icon in constraint_icons {
        // Convert sketch coordinate to screen coordinate
        if let Some(screen_pos) = sketch_to_screen(
            icon.position.x,
            icon.position.y,
            sketch_transform,
            view_proj,
            viewport_size,
        ) {
            // Only render if on screen
            if screen_pos.x >= 0.0
                && screen_pos.y >= 0.0
                && screen_pos.x <= viewport_size.x
                && screen_pos.y <= viewport_size.y
            {
                let screen_pos = egui::pos2(rect.left() + screen_pos.x, rect.top() + screen_pos.y);

                // Get icon text and color based on constraint type
                let (icon_text, color) = get_constraint_icon(&icon.constraint_type, icon.value);

                // Draw the icon with background
                draw_constraint_icon(ui, screen_pos, &icon_text, color);
            }
        }
    }
}

/// Convert sketch coordinate to screen coordinate
fn sketch_to_screen(
    sketch_x: f32,
    sketch_y: f32,
    sketch_transform: Mat4,
    view_proj: Mat4,
    viewport_size: egui::Vec2,
) -> Option<egui::Vec2> {
    // Sketch coordinate -> World coordinate
    let world_pos = sketch_transform * Vec4::new(sketch_x, sketch_y, 0.0, 1.0);

    // World coordinate -> Clip coordinate
    let clip_pos = view_proj * world_pos;

    // Check if behind camera
    if clip_pos.w <= 0.0 {
        return None;
    }

    // Clip coordinate -> NDC
    let ndc_x = clip_pos.x / clip_pos.w;
    let ndc_y = clip_pos.y / clip_pos.w;

    // NDC -> Screen coordinate
    let screen_x = (ndc_x + 1.0) * 0.5 * viewport_size.x;
    let screen_y = (1.0 - ndc_y) * 0.5 * viewport_size.y;

    Some(egui::vec2(screen_x, screen_y))
}

/// Get the icon text and color for a constraint type
fn get_constraint_icon(constraint_type: &str, value: Option<f32>) -> (String, egui::Color32) {
    match constraint_type {
        "Coincident" => ("\u{2299}".to_string(), palette::CONSTRAINT_GEOMETRIC), // ⊙
        "Horizontal" => ("H".to_string(), palette::CONSTRAINT_GEOMETRIC),
        "Vertical" => ("V".to_string(), palette::CONSTRAINT_GEOMETRIC),
        "Parallel" => ("\u{2225}".to_string(), palette::CONSTRAINT_GEOMETRIC), // ∥
        "Perpendicular" => ("\u{22A5}".to_string(), palette::CONSTRAINT_GEOMETRIC), // ⊥
        "Tangent" => ("T".to_string(), palette::CONSTRAINT_GEOMETRIC),
        "Equal Length" => ("=".to_string(), palette::CONSTRAINT_GEOMETRIC),
        "Equal Radius" => ("R=".to_string(), palette::CONSTRAINT_GEOMETRIC),
        "Point on Curve" => ("\u{00D7}".to_string(), palette::CONSTRAINT_GEOMETRIC), // ×
        "Midpoint" => ("M".to_string(), palette::CONSTRAINT_GEOMETRIC),
        "Symmetric" => ("\u{21C6}".to_string(), palette::CONSTRAINT_GEOMETRIC), // ⇆
        "Fixed" => ("F".to_string(), palette::CONSTRAINT_FIXED),

        // Dimensional constraints with values
        "Distance" => {
            let text = if let Some(v) = value {
                format!("{:.2}", v)
            } else {
                "?".to_string()
            };
            (text, palette::CONSTRAINT_DIMENSION)
        }
        "Horizontal Distance" => {
            let text = if let Some(v) = value {
                format!("\u{2194}{:.2}", v) // ↔
            } else {
                "\u{2194}?".to_string()
            };
            (text, palette::CONSTRAINT_DIMENSION)
        }
        "Vertical Distance" => {
            let text = if let Some(v) = value {
                format!("\u{2195}{:.2}", v) // ↕
            } else {
                "\u{2195}?".to_string()
            };
            (text, palette::CONSTRAINT_DIMENSION)
        }
        "Angle" => {
            let text = if let Some(v) = value {
                format!("\u{2220}{:.1}\u{00B0}", v.to_degrees()) // ∠ and °
            } else {
                "\u{2220}?".to_string()
            };
            (text, palette::CONSTRAINT_DIMENSION)
        }
        "Radius" => {
            let text = if let Some(v) = value {
                format!("R{:.2}", v)
            } else {
                "R?".to_string()
            };
            (text, palette::CONSTRAINT_DIMENSION)
        }
        "Diameter" => {
            let text = if let Some(v) = value {
                format!("\u{2300}{:.2}", v) // ⌀
            } else {
                "\u{2300}?".to_string()
            };
            (text, palette::CONSTRAINT_DIMENSION)
        }
        "Length" => {
            let text = if let Some(v) = value {
                format!("L{:.2}", v)
            } else {
                "L?".to_string()
            };
            (text, palette::CONSTRAINT_DIMENSION)
        }

        // Unknown constraint type
        _ => ("?".to_string(), palette::TEXT_SECONDARY),
    }
}

/// Draw a constraint icon with background at the given position
fn draw_constraint_icon(ui: &mut egui::Ui, pos: egui::Pos2, text: &str, color: egui::Color32) {
    let font_id = egui::FontId::proportional(11.0);
    let painter = ui.painter();

    // Estimate text size (approximation)
    let char_width = 7.0;
    let text_width = text.len() as f32 * char_width;
    let text_height = 14.0;
    let padding = egui::vec2(4.0, 2.0);

    // Background rect
    let bg_rect = egui::Rect::from_min_size(
        pos - egui::vec2(padding.x, padding.y + text_height * 0.5),
        egui::vec2(text_width + padding.x * 2.0, text_height + padding.y * 2.0),
    );

    // Draw background
    painter.rect_filled(bg_rect, 3.0, palette::overlay_bg(220));
    painter.rect_stroke(
        bg_rect,
        3.0,
        egui::Stroke::new(1.0, color.linear_multiply(0.5)),
        egui::StrokeKind::Outside,
    );

    // Draw text
    painter.text(
        bg_rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        font_id,
        color,
    );
}
