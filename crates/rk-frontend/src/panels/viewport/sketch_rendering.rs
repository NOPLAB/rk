//! Sketch rendering logic for the viewport

use glam::Vec2;
use rk_cad::{Sketch, SketchEntity};
use rk_renderer::SketchRenderData;

use crate::state::InProgressEntity;

/// Colors for sketch rendering
pub mod sketch_colors {
    use glam::Vec4;

    pub const POINT: Vec4 = Vec4::new(0.0, 0.8, 0.0, 1.0); // Green
    pub const LINE: Vec4 = Vec4::new(1.0, 1.0, 1.0, 1.0); // White
    pub const CIRCLE: Vec4 = Vec4::new(0.0, 0.7, 1.0, 1.0); // Cyan
    pub const ARC: Vec4 = Vec4::new(0.0, 0.7, 1.0, 1.0); // Cyan
    pub const SELECTED: Vec4 = Vec4::new(1.0, 0.5, 0.0, 1.0); // Orange
    pub const PREVIEW: Vec4 = Vec4::new(0.5, 0.5, 1.0, 0.7); // Semi-transparent blue for preview

    // Origin and axis colors
    pub const ORIGIN: Vec4 = Vec4::new(1.0, 1.0, 0.0, 1.0); // Yellow
    pub const AXIS_X: Vec4 = Vec4::new(1.0, 0.2, 0.2, 1.0); // Red
    pub const AXIS_Y: Vec4 = Vec4::new(0.2, 1.0, 0.2, 1.0); // Green
}

/// Length of the axis lines in sketch coordinate units
pub const SKETCH_AXIS_LENGTH: f32 = 100.0;

/// Convert a Sketch to SketchRenderData
pub fn sketch_to_render_data(
    sketch: &Sketch,
    selected_entities: &[uuid::Uuid],
    is_active: bool,
    in_progress: Option<&InProgressEntity>,
) -> SketchRenderData {
    let mut render_data = SketchRenderData::new(sketch.id, sketch.plane.transform());
    render_data.is_active = is_active;

    // Draw origin point and axis lines (always visible as reference)
    // Origin point
    render_data.add_point(Vec2::ZERO, sketch_colors::ORIGIN, 0);
    // X axis (positive direction)
    render_data.add_line(
        Vec2::ZERO,
        Vec2::new(SKETCH_AXIS_LENGTH, 0.0),
        sketch_colors::AXIS_X,
        0,
    );
    // Y axis (positive direction)
    render_data.add_line(
        Vec2::ZERO,
        Vec2::new(0.0, SKETCH_AXIS_LENGTH),
        sketch_colors::AXIS_Y,
        0,
    );

    // First pass: collect all point positions
    let mut point_positions: std::collections::HashMap<uuid::Uuid, Vec2> =
        std::collections::HashMap::new();

    for entity in sketch.entities().values() {
        if let SketchEntity::Point { id, position } = entity {
            point_positions.insert(*id, *position);
        }
    }

    // Second pass: render entities
    for entity in sketch.entities().values() {
        let entity_id = entity.id();
        let is_selected = selected_entities.contains(&entity_id);
        let flags = if is_selected { 1 } else { 0 };

        match entity {
            SketchEntity::Point { position, .. } => {
                let color = if is_selected {
                    sketch_colors::SELECTED
                } else {
                    sketch_colors::POINT
                };
                render_data.add_point(*position, color, flags);
            }
            SketchEntity::Line { start, end, .. } => {
                if let (Some(&start_pos), Some(&end_pos)) =
                    (point_positions.get(start), point_positions.get(end))
                {
                    let color = if is_selected {
                        sketch_colors::SELECTED
                    } else {
                        sketch_colors::LINE
                    };
                    render_data.add_line(start_pos, end_pos, color, flags);
                }
            }
            SketchEntity::Circle { center, radius, .. } => {
                if let Some(&center_pos) = point_positions.get(center) {
                    let color = if is_selected {
                        sketch_colors::SELECTED
                    } else {
                        sketch_colors::CIRCLE
                    };
                    render_data.add_circle(center_pos, *radius, color, flags, 64);
                }
            }
            SketchEntity::Arc {
                center,
                start,
                end,
                radius,
                ..
            } => {
                if let (Some(&center_pos), Some(&start_pos), Some(&end_pos)) = (
                    point_positions.get(center),
                    point_positions.get(start),
                    point_positions.get(end),
                ) {
                    let color = if is_selected {
                        sketch_colors::SELECTED
                    } else {
                        sketch_colors::ARC
                    };
                    // Calculate start and end angles
                    let start_offset = start_pos - center_pos;
                    let end_offset = end_pos - center_pos;
                    let start_angle = start_offset.y.atan2(start_offset.x);
                    let end_angle = end_offset.y.atan2(end_offset.x);
                    render_data.add_arc(
                        center_pos,
                        *radius,
                        start_angle,
                        end_angle,
                        color,
                        flags,
                        32,
                    );
                }
            }
            _ => {} // Other entity types not yet rendered
        }
    }

    // Render in-progress entity preview
    if let Some(in_progress) = in_progress {
        render_in_progress_preview(&mut render_data, in_progress, &point_positions);
    }

    render_data
}

/// Render preview for in-progress entities
fn render_in_progress_preview(
    render_data: &mut SketchRenderData,
    in_progress: &InProgressEntity,
    point_positions: &std::collections::HashMap<uuid::Uuid, Vec2>,
) {
    let preview_color = sketch_colors::PREVIEW;

    match in_progress {
        InProgressEntity::Line {
            start_point,
            preview_end,
        } => {
            if let Some(&start_pos) = point_positions.get(start_point) {
                render_data.add_line(start_pos, *preview_end, preview_color, 0);
                // Also draw preview point at the end
                render_data.add_point(*preview_end, preview_color, 0);
            }
        }
        InProgressEntity::Circle {
            center_point,
            preview_radius,
        } => {
            if let Some(&center_pos) = point_positions.get(center_point) {
                render_data.add_circle(center_pos, *preview_radius, preview_color, 0, 64);
            }
        }
        InProgressEntity::Arc {
            center_point,
            start_point,
            preview_end,
        } => {
            if let Some(&center_pos) = point_positions.get(center_point) {
                if let Some(start_id) = start_point {
                    if let Some(&start_pos) = point_positions.get(start_id) {
                        let radius = (start_pos - center_pos).length();
                        let start_offset = start_pos - center_pos;
                        let end_offset = *preview_end - center_pos;
                        let start_angle = start_offset.y.atan2(start_offset.x);
                        let end_angle = end_offset.y.atan2(end_offset.x);
                        render_data.add_arc(
                            center_pos,
                            radius,
                            start_angle,
                            end_angle,
                            preview_color,
                            0,
                            32,
                        );
                    }
                } else {
                    // Just show a line from center to preview
                    render_data.add_line(center_pos, *preview_end, preview_color, 0);
                }
                render_data.add_point(*preview_end, preview_color, 0);
            }
        }
        InProgressEntity::Rectangle {
            corner1,
            preview_corner2,
        } => {
            // Draw rectangle as 4 lines
            let c1 = *corner1;
            let c2 = *preview_corner2;
            let tl = Vec2::new(c1.x, c2.y);
            let br = Vec2::new(c2.x, c1.y);
            render_data.add_line(c1, tl, preview_color, 0);
            render_data.add_line(tl, c2, preview_color, 0);
            render_data.add_line(c2, br, preview_color, 0);
            render_data.add_line(br, c1, preview_color, 0);
            // Draw corner points
            render_data.add_point(c1, preview_color, 0);
            render_data.add_point(c2, preview_color, 0);
        }
    }
}
