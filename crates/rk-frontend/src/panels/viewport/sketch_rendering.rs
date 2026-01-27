//! Sketch rendering logic for the viewport

use glam::Vec2;
use rk_cad::{Sketch, SketchConstraint, SketchEntity};
use rk_renderer::{ConstraintIconData, SketchRenderData};

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

    // Generate constraint icon data (only for active sketch)
    if is_active {
        render_data.constraint_icons =
            generate_constraint_icons(sketch, &point_positions, selected_entities);
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

/// Generate constraint icon data for overlay rendering
fn generate_constraint_icons(
    sketch: &Sketch,
    point_positions: &std::collections::HashMap<uuid::Uuid, Vec2>,
    _selected_entities: &[uuid::Uuid],
) -> Vec<ConstraintIconData> {
    let mut icons = Vec::new();

    for constraint in sketch.constraints().values() {
        if let Some(position) = calculate_constraint_position(constraint, sketch, point_positions) {
            icons.push(ConstraintIconData {
                id: constraint.id(),
                position,
                constraint_type: constraint.type_name().to_string(),
                value: constraint.value(),
            });
        }
    }

    icons
}

/// Calculate the position where a constraint icon should be displayed
fn calculate_constraint_position(
    constraint: &SketchConstraint,
    sketch: &Sketch,
    point_positions: &std::collections::HashMap<uuid::Uuid, Vec2>,
) -> Option<Vec2> {
    // Small offset to prevent overlapping with geometry
    let offset = Vec2::new(0.015, 0.015);

    match constraint {
        SketchConstraint::Coincident { point1, point2, .. } => {
            // Position at the midpoint between the two points
            let p1 = point_positions.get(point1)?;
            let p2 = point_positions.get(point2)?;
            Some((*p1 + *p2) * 0.5 + offset)
        }

        SketchConstraint::Horizontal { line, .. } | SketchConstraint::Vertical { line, .. } => {
            // Position at the midpoint of the line
            line_midpoint(sketch, *line, point_positions).map(|p| p + offset)
        }

        SketchConstraint::Parallel { line1, line2, .. }
        | SketchConstraint::Perpendicular { line1, line2, .. }
        | SketchConstraint::EqualLength { line1, line2, .. } => {
            // Position at the midpoint between the two line midpoints
            let mid1 = line_midpoint(sketch, *line1, point_positions)?;
            let mid2 = line_midpoint(sketch, *line2, point_positions)?;
            Some((mid1 + mid2) * 0.5 + offset)
        }

        SketchConstraint::Tangent { curve1, curve2, .. } => {
            // Position at the midpoint between the two curve centers
            let c1 = entity_center(sketch, *curve1, point_positions)?;
            let c2 = entity_center(sketch, *curve2, point_positions)?;
            Some((c1 + c2) * 0.5 + offset)
        }

        SketchConstraint::EqualRadius {
            circle1, circle2, ..
        } => {
            // Position between the two circle centers
            let c1 = entity_center(sketch, *circle1, point_positions)?;
            let c2 = entity_center(sketch, *circle2, point_positions)?;
            Some((c1 + c2) * 0.5 + offset)
        }

        SketchConstraint::PointOnCurve { point, curve, .. } => {
            // Position near the point
            let p = point_positions.get(point)?;
            let c = entity_center(sketch, *curve, point_positions)?;
            Some((*p + c) * 0.5 + offset)
        }

        SketchConstraint::Midpoint { point, .. } => {
            // Position near the midpoint
            let p = point_positions.get(point)?;
            Some(*p + offset)
        }

        SketchConstraint::Symmetric {
            entity1, entity2, ..
        } => {
            // Position at the midpoint between the two entities
            let c1 = entity_center(sketch, *entity1, point_positions)?;
            let c2 = entity_center(sketch, *entity2, point_positions)?;
            Some((c1 + c2) * 0.5 + offset)
        }

        SketchConstraint::Fixed { point, .. } => {
            // Position near the fixed point
            let p = point_positions.get(point)?;
            Some(*p + offset)
        }

        SketchConstraint::Distance {
            entity1, entity2, ..
        } => {
            // Position at the midpoint between the two entities
            let c1 = entity_center(sketch, *entity1, point_positions)?;
            let c2 = entity_center(sketch, *entity2, point_positions)?;
            // Offset perpendicular to the line between entities
            let dir = (c2 - c1).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x);
            Some((c1 + c2) * 0.5 + perp * 0.03)
        }

        SketchConstraint::HorizontalDistance { point1, point2, .. } => {
            // Position above the line between points
            let p1 = point_positions.get(point1)?;
            let p2 = point_positions.get(point2)?;
            Some(Vec2::new((p1.x + p2.x) * 0.5, p1.y.max(p2.y) + 0.03))
        }

        SketchConstraint::VerticalDistance { point1, point2, .. } => {
            // Position to the right of the line between points
            let p1 = point_positions.get(point1)?;
            let p2 = point_positions.get(point2)?;
            Some(Vec2::new(p1.x.max(p2.x) + 0.03, (p1.y + p2.y) * 0.5))
        }

        SketchConstraint::Angle { line1, line2, .. } => {
            // Position at the intersection area of the two lines
            let mid1 = line_midpoint(sketch, *line1, point_positions)?;
            let mid2 = line_midpoint(sketch, *line2, point_positions)?;
            Some((mid1 + mid2) * 0.5 + offset)
        }

        SketchConstraint::Radius { circle, .. } | SketchConstraint::Diameter { circle, .. } => {
            // Position at the circle center with offset
            let center = entity_center(sketch, *circle, point_positions)?;
            Some(center + offset * 2.0)
        }

        SketchConstraint::Length { line, .. } => {
            // Position at the line midpoint with perpendicular offset
            let mid = line_midpoint(sketch, *line, point_positions)?;
            if let Some(entity) = sketch.get_entity(*line)
                && let SketchEntity::Line { start, end, .. } = entity
                && let (Some(p1), Some(p2)) = (point_positions.get(start), point_positions.get(end))
            {
                let dir = (*p2 - *p1).normalize_or_zero();
                let perp = Vec2::new(-dir.y, dir.x);
                return Some(mid + perp * 0.03);
            }
            Some(mid + offset)
        }
    }
}

/// Get the midpoint of a line entity
fn line_midpoint(
    sketch: &Sketch,
    line_id: uuid::Uuid,
    point_positions: &std::collections::HashMap<uuid::Uuid, Vec2>,
) -> Option<Vec2> {
    let entity = sketch.get_entity(line_id)?;
    if let SketchEntity::Line { start, end, .. } = entity {
        let p1 = point_positions.get(start)?;
        let p2 = point_positions.get(end)?;
        Some((*p1 + *p2) * 0.5)
    } else {
        None
    }
}

/// Get the center of an entity (for points, circles, arcs, lines)
fn entity_center(
    sketch: &Sketch,
    entity_id: uuid::Uuid,
    point_positions: &std::collections::HashMap<uuid::Uuid, Vec2>,
) -> Option<Vec2> {
    let entity = sketch.get_entity(entity_id)?;
    match entity {
        SketchEntity::Point { position, .. } => Some(*position),
        SketchEntity::Line { start, end, .. } => {
            let p1 = point_positions.get(start)?;
            let p2 = point_positions.get(end)?;
            Some((*p1 + *p2) * 0.5)
        }
        SketchEntity::Circle { center, .. } | SketchEntity::Arc { center, .. } => {
            point_positions.get(center).copied()
        }
        _ => point_positions.get(&entity_id).copied(),
    }
}
