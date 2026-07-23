//! Constraint icon overlay for sketch mode
//!
//! Renders constraint icons as egui overlay on the viewport when in sketch mode.

use glam::{Mat4, Vec2, Vec4};
use rk_renderer::{ArcData, Camera, ConstraintIconData, DimensionLine};

use crate::theme::palette;

/// Arrow head size in pixels
const ARROW_SIZE: f32 = 8.0;
/// Extension line gap from geometry
const EXTENSION_GAP: f32 = 3.0;
/// Extension line overshoot past dimension line
const EXTENSION_OVERSHOOT: f32 = 4.0;

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

                // Draw dimension line if present (for dimensional constraints)
                if let Some(ref dim_line) = icon.dimension_line {
                    draw_dimension_line(
                        ui,
                        rect,
                        dim_line,
                        sketch_transform,
                        view_proj,
                        viewport_size,
                        color,
                    );
                }

                // Draw the text label with background
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

/// Draw a dimension line with arrows and extension lines
fn draw_dimension_line(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    dim_line: &DimensionLine,
    sketch_transform: Mat4,
    view_proj: Mat4,
    viewport_size: egui::Vec2,
    color: egui::Color32,
) {
    let painter = ui.painter();
    let stroke = egui::Stroke::new(1.0, color);

    // Check for arc data (angle constraint)
    if let Some(ref arc_data) = dim_line.arc_data {
        draw_angle_arc(
            painter,
            rect,
            arc_data,
            sketch_transform,
            view_proj,
            viewport_size,
            color,
        );
        return;
    }

    // Convert dimension line endpoints to screen coordinates
    let start_screen = sketch_to_screen(
        dim_line.start.x,
        dim_line.start.y,
        sketch_transform,
        view_proj,
        viewport_size,
    );
    let end_screen = sketch_to_screen(
        dim_line.end.x,
        dim_line.end.y,
        sketch_transform,
        view_proj,
        viewport_size,
    );

    if let (Some(start), Some(end)) = (start_screen, end_screen) {
        let start_pos = egui::pos2(rect.left() + start.x, rect.top() + start.y);
        let end_pos = egui::pos2(rect.left() + end.x, rect.top() + end.y);

        // Draw the main dimension line
        painter.line_segment([start_pos, end_pos], stroke);

        // Draw arrows at both ends
        let direction = (end_pos - start_pos).normalized();
        draw_arrow(painter, start_pos, direction, color);
        draw_arrow(painter, end_pos, -direction, color);

        // Draw extension lines if present
        if let Some(ext_start) = dim_line.extension_start
            && let Some(ext_start_screen) = sketch_to_screen(
                ext_start.x,
                ext_start.y,
                sketch_transform,
                view_proj,
                viewport_size,
            )
        {
            let ext_start_pos = egui::pos2(
                rect.left() + ext_start_screen.x,
                rect.top() + ext_start_screen.y,
            );
            draw_extension_line(painter, ext_start_pos, start_pos, color);
        }

        if let Some(ext_end) = dim_line.extension_end
            && let Some(ext_end_screen) = sketch_to_screen(
                ext_end.x,
                ext_end.y,
                sketch_transform,
                view_proj,
                viewport_size,
            )
        {
            let ext_end_pos = egui::pos2(
                rect.left() + ext_end_screen.x,
                rect.top() + ext_end_screen.y,
            );
            draw_extension_line(painter, ext_end_pos, end_pos, color);
        }
    }
}

/// Draw an arrow head at the given position pointing in the given direction
fn draw_arrow(
    painter: &egui::Painter,
    tip: egui::Pos2,
    direction: egui::Vec2,
    color: egui::Color32,
) {
    let perp = egui::vec2(-direction.y, direction.x);

    // Arrow head points (triangle)
    let p1 = tip;
    let p2 = tip - direction * ARROW_SIZE + perp * (ARROW_SIZE * 0.4);
    let p3 = tip - direction * ARROW_SIZE - perp * (ARROW_SIZE * 0.4);

    painter.add(egui::Shape::convex_polygon(
        vec![p1, p2, p3],
        color,
        egui::Stroke::NONE,
    ));
}

/// Draw an extension line from geometry point to dimension line
fn draw_extension_line(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    color: egui::Color32,
) {
    let direction = (to - from).normalized();

    // Start with a small gap from the geometry
    let start = from + direction * EXTENSION_GAP;
    // Extend past the dimension line
    let end = to + direction * EXTENSION_OVERSHOOT;

    let stroke = egui::Stroke::new(0.5, color.linear_multiply(0.7));
    painter.line_segment([start, end], stroke);
}

/// Draw an arc for angle constraints
fn draw_angle_arc(
    painter: &egui::Painter,
    rect: egui::Rect,
    arc_data: &ArcData,
    sketch_transform: Mat4,
    view_proj: Mat4,
    viewport_size: egui::Vec2,
    color: egui::Color32,
) {
    // Convert center to screen coordinates
    let center_screen = sketch_to_screen(
        arc_data.center.x,
        arc_data.center.y,
        sketch_transform,
        view_proj,
        viewport_size,
    );

    if let Some(center) = center_screen {
        let center_pos = egui::pos2(rect.left() + center.x, rect.top() + center.y);

        // Calculate screen-space radius by converting a point on the arc
        let edge_point = Vec2::new(
            arc_data.center.x + arc_data.radius * arc_data.start_angle.cos(),
            arc_data.center.y + arc_data.radius * arc_data.start_angle.sin(),
        );
        if let Some(edge_screen) = sketch_to_screen(
            edge_point.x,
            edge_point.y,
            sketch_transform,
            view_proj,
            viewport_size,
        ) {
            let edge_pos = egui::pos2(rect.left() + edge_screen.x, rect.top() + edge_screen.y);
            let screen_radius = (edge_pos - center_pos).length();

            // Draw arc as line segments
            let segments = 32;
            let angle1 = arc_data.start_angle;
            let mut angle2 = arc_data.end_angle;

            // Normalize angle range
            while angle2 < angle1 {
                angle2 += std::f32::consts::TAU;
            }

            let step = (angle2 - angle1) / segments as f32;
            let stroke = egui::Stroke::new(1.0, color);

            for i in 0..segments {
                let a1 = angle1 + i as f32 * step;
                let a2 = angle1 + (i + 1) as f32 * step;

                // Note: In screen coordinates, Y is inverted
                let p1 =
                    center_pos + egui::vec2(a1.cos() * screen_radius, -a1.sin() * screen_radius);
                let p2 =
                    center_pos + egui::vec2(a2.cos() * screen_radius, -a2.sin() * screen_radius);

                painter.line_segment([p1, p2], stroke);
            }

            // Draw arrows at arc ends
            let end_angle1 = angle1;
            let end_angle2 = angle2;

            // Arrow at start of arc (tangent direction)
            let start_pos = center_pos
                + egui::vec2(
                    end_angle1.cos() * screen_radius,
                    -end_angle1.sin() * screen_radius,
                );
            let start_tangent = egui::vec2(end_angle1.sin(), end_angle1.cos()).normalized();
            draw_arrow(painter, start_pos, start_tangent, color);

            // Arrow at end of arc (tangent direction, opposite)
            let end_pos = center_pos
                + egui::vec2(
                    end_angle2.cos() * screen_radius,
                    -end_angle2.sin() * screen_radius,
                );
            let end_tangent = egui::vec2(-end_angle2.sin(), -end_angle2.cos()).normalized();
            draw_arrow(painter, end_pos, end_tangent, color);
        }
    }
}
