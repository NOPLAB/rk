//! 3D Viewport panel

mod camera_overlay;

use glam::{Mat4, Vec2, Vec3};
use rk_cad::{Sketch, SketchEntity, SketchPlane};
use rk_renderer::{Camera, GizmoAxis, GizmoMode, GizmoSpace, SketchRenderData, plane_ids};

use crate::config::SharedConfig;
use crate::panels::Panel;
use crate::state::{
    AppAction, GizmoTransform, InProgressEntity, PickablePartData, ReferencePlane, SharedAppState,
    SharedViewportState, SketchAction, SketchTool, pick_object,
};

use camera_overlay::{
    render_axes_indicator, render_camera_settings, render_dimension_dialog, render_extrude_dialog,
    render_gizmo_toggle, render_plane_selection_hint, render_sketch_toolbar,
};

/// Colors for sketch rendering
mod sketch_colors {
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
const SKETCH_AXIS_LENGTH: f32 = 100.0;

/// Convert a Sketch to SketchRenderData
fn sketch_to_render_data(
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

/// Size of the reference planes for picking
const PLANE_SIZE: f32 = 2.0;

/// Pick a reference plane from screen coordinates.
///
/// Returns the closest plane that the ray intersects within the plane bounds,
/// or None if no plane is hit.
pub fn pick_reference_plane(
    camera: &Camera,
    screen_x: f32,
    screen_y: f32,
    width: f32,
    height: f32,
) -> Option<ReferencePlane> {
    let (ray_origin, ray_dir) = camera.screen_to_ray(screen_x, screen_y, width, height);

    let mut closest: Option<(ReferencePlane, f32)> = None;

    for plane in ReferencePlane::all() {
        if let Some(t) = ray_plane_intersection(ray_origin, ray_dir, plane) {
            // Check if the intersection point is within the plane bounds
            let hit_point = ray_origin + ray_dir * t;

            if is_point_in_plane_bounds(hit_point, plane, PLANE_SIZE) {
                // Check if this is closer than any previous hit
                if closest.is_none() || t < closest.unwrap().1 {
                    closest = Some((plane, t));
                }
            }
        }
    }

    closest.map(|(p, _)| p)
}

/// Calculate ray-plane intersection.
///
/// Returns the parameter t such that ray_origin + ray_dir * t is on the plane,
/// or None if the ray is parallel to the plane.
fn ray_plane_intersection(ray_origin: Vec3, ray_dir: Vec3, plane: ReferencePlane) -> Option<f32> {
    let normal = plane.normal();
    let denom = ray_dir.dot(normal);

    // Check if ray is parallel to plane
    if denom.abs() < 1e-6 {
        return None;
    }

    // Plane passes through origin, so d = 0
    let t = -ray_origin.dot(normal) / denom;

    // Only return positive t (intersection in front of camera)
    if t > 0.0 { Some(t) } else { None }
}

/// Check if a point is within the bounds of a reference plane.
fn is_point_in_plane_bounds(point: Vec3, plane: ReferencePlane, size: f32) -> bool {
    match plane {
        ReferencePlane::XY => point.x.abs() <= size && point.y.abs() <= size,
        ReferencePlane::XZ => point.x.abs() <= size && point.z.abs() <= size,
        ReferencePlane::YZ => point.y.abs() <= size && point.z.abs() <= size,
    }
}

/// Convert screen coordinates to sketch 2D coordinates.
///
/// Returns None if the ray is parallel to the sketch plane or if the intersection
/// is behind the camera.
fn screen_to_sketch_coords(
    camera: &Camera,
    sketch_plane: &SketchPlane,
    screen_x: f32,
    screen_y: f32,
    width: f32,
    height: f32,
) -> Option<Vec2> {
    let (ray_origin, ray_dir) = camera.screen_to_ray(screen_x, screen_y, width, height);

    // Ray-plane intersection
    let denom = ray_dir.dot(sketch_plane.normal);
    if denom.abs() < 1e-6 {
        return None; // Ray is parallel to the plane
    }

    let t = (sketch_plane.origin - ray_origin).dot(sketch_plane.normal) / denom;
    if t < 0.0 {
        return None; // Intersection is behind the camera
    }

    let hit_3d = ray_origin + ray_dir * t;
    Some(sketch_plane.to_local(hit_3d))
}

/// Pick a sketch entity from sketch coordinates.
/// Returns the closest entity to the point within the pick radius.
fn pick_sketch_entity(sketch: &Sketch, sketch_pos: Vec2, pick_radius: f32) -> Option<uuid::Uuid> {
    use std::collections::HashMap;

    // Collect point positions for line/arc/circle distance calculations
    let point_positions: HashMap<uuid::Uuid, Vec2> = sketch
        .entities()
        .values()
        .filter_map(|e| {
            if let SketchEntity::Point { id, position } = e {
                Some((*id, *position))
            } else {
                None
            }
        })
        .collect();

    let mut closest: Option<(uuid::Uuid, f32)> = None;

    for entity in sketch.entities().values() {
        let dist = entity_distance(entity, sketch_pos, &point_positions);
        if dist < pick_radius && (closest.is_none() || dist < closest.unwrap().1) {
            closest = Some((entity.id(), dist));
        }
    }

    closest.map(|(id, _)| id)
}

/// Calculate distance from a point to an entity
fn entity_distance(
    entity: &SketchEntity,
    point: Vec2,
    point_positions: &std::collections::HashMap<uuid::Uuid, Vec2>,
) -> f32 {
    match entity {
        SketchEntity::Point { position, .. } => (*position - point).length(),
        SketchEntity::Line { start, end, .. } => {
            if let (Some(&start_pos), Some(&end_pos)) =
                (point_positions.get(start), point_positions.get(end))
            {
                point_to_line_distance(point, start_pos, end_pos)
            } else {
                f32::INFINITY
            }
        }
        SketchEntity::Circle { center, radius, .. } => {
            if let Some(&center_pos) = point_positions.get(center) {
                ((center_pos - point).length() - radius).abs()
            } else {
                f32::INFINITY
            }
        }
        SketchEntity::Arc { center, radius, .. } => {
            if let Some(&center_pos) = point_positions.get(center) {
                // Simplified: just use radial distance
                ((center_pos - point).length() - radius).abs()
            } else {
                f32::INFINITY
            }
        }
        // Ellipse and Spline not yet supported for picking
        SketchEntity::Ellipse { .. } | SketchEntity::Spline { .. } => f32::INFINITY,
    }
}

/// Calculate distance from a point to a line segment
fn point_to_line_distance(point: Vec2, line_start: Vec2, line_end: Vec2) -> f32 {
    let line = line_end - line_start;
    let len_sq = line.length_squared();

    if len_sq < 1e-10 {
        return (point - line_start).length();
    }

    let t = ((point - line_start).dot(line) / len_sq).clamp(0.0, 1.0);
    let projection = line_start + line * t;
    (point - projection).length()
}

use crate::state::ViewportState;

/// Handle sketch mode mouse input.
///
/// Returns true if the input was consumed by sketch mode.
fn handle_sketch_mode_input(
    response: &egui::Response,
    ui: &egui::Ui,
    local_mouse: Option<egui::Vec2>,
    available_size: egui::Vec2,
    app_state: &SharedAppState,
    vp_state: &parking_lot::MutexGuard<ViewportState>,
) -> bool {
    let Some(pos) = local_mouse else {
        return false;
    };

    // Get sketch info from app state
    let (sketch_plane, current_tool, snap_to_grid, grid_spacing, active_sketch_id) = {
        let app = app_state.lock();
        let Some(sketch_state) = app.cad.editor_mode.sketch() else {
            return false;
        };
        let sketch_id = sketch_state.active_sketch;
        let Some(sketch) = app.cad.get_sketch(sketch_id) else {
            return false;
        };
        (
            sketch.plane,
            sketch_state.current_tool,
            sketch_state.snap_to_grid,
            sketch_state.grid_spacing,
            sketch_id,
        )
    };

    // Convert screen position to sketch coordinates
    let camera = vp_state.renderer.camera();
    let Some(sketch_pos) = screen_to_sketch_coords(
        camera,
        &sketch_plane,
        pos.x,
        pos.y,
        available_size.x,
        available_size.y,
    ) else {
        return false;
    };

    // Apply grid snapping
    let snapped_pos = if snap_to_grid {
        Vec2::new(
            (sketch_pos.x / grid_spacing).round() * grid_spacing,
            (sketch_pos.y / grid_spacing).round() * grid_spacing,
        )
    } else {
        sketch_pos
    };

    // Handle mouse move (update preview position for in-progress entities)
    {
        let mut app = app_state.lock();

        // First, collect all the point positions from the sketch
        let point_positions: std::collections::HashMap<uuid::Uuid, Vec2> =
            if let Some(sketch) = app.cad.get_sketch(active_sketch_id) {
                sketch
                    .entities()
                    .values()
                    .filter_map(|entity| {
                        if let SketchEntity::Point { id, position } = entity {
                            Some((*id, *position))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                std::collections::HashMap::new()
            };

        // Now update the in_progress state
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut() {
            match &mut sketch_state.in_progress {
                Some(InProgressEntity::Line { preview_end, .. }) => {
                    *preview_end = snapped_pos;
                }
                Some(InProgressEntity::Circle {
                    center_point,
                    preview_radius,
                }) => {
                    if let Some(&center_pos) = point_positions.get(center_point) {
                        *preview_radius = (center_pos - snapped_pos).length();
                    }
                }
                Some(InProgressEntity::Arc { preview_end, .. }) => {
                    *preview_end = snapped_pos;
                }
                Some(InProgressEntity::Rectangle {
                    preview_corner2, ..
                }) => {
                    *preview_corner2 = snapped_pos;
                }
                None => {}
            }
        }
    }

    // Handle Escape key to cancel in-progress drawing
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        let mut app = app_state.lock();
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut()
            && sketch_state.in_progress.is_some()
        {
            sketch_state.cancel_drawing();
            return true;
        }
    }

    // Handle right-click to cancel in-progress drawing
    if response.clicked_by(egui::PointerButton::Secondary) {
        let mut app = app_state.lock();
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut()
            && sketch_state.in_progress.is_some()
        {
            sketch_state.cancel_drawing();
            return true;
        }
    }

    // Handle left click based on current tool
    if response.clicked_by(egui::PointerButton::Primary) {
        match current_tool {
            SketchTool::Point => {
                // Create a point at the clicked position
                let point = SketchEntity::point(snapped_pos);
                app_state
                    .lock()
                    .queue_action(AppAction::SketchAction(SketchAction::AddEntity {
                        entity: point,
                    }));
                return true;
            }
            SketchTool::Line => {
                return handle_line_tool_click(app_state, snapped_pos);
            }
            SketchTool::RectangleCorner => {
                return handle_rectangle_tool_click(app_state, snapped_pos);
            }
            SketchTool::CircleCenterRadius => {
                return handle_circle_tool_click(app_state, snapped_pos);
            }
            SketchTool::Select => {
                // TODO: Implement entity selection
                return false;
            }
            tool if tool.is_constraint() || tool.is_dimension() => {
                // Handle constraint tool clicks
                return handle_constraint_tool_click(
                    app_state,
                    snapped_pos,
                    active_sketch_id,
                    tool,
                );
            }
            _ => {
                // Other tools not yet implemented
                return false;
            }
        }
    }

    false
}

/// Handle line tool click.
/// First click creates start point and starts line preview.
/// Second click creates end point and the line.
fn handle_line_tool_click(app_state: &SharedAppState, snapped_pos: Vec2) -> bool {
    let mut app = app_state.lock();

    let Some(sketch_state) = app.cad.editor_mode.sketch_mut() else {
        return false;
    };

    let active_sketch_id = sketch_state.active_sketch;

    if sketch_state.in_progress.is_none() {
        // First click: create start point and start line preview
        let start_point = SketchEntity::point(snapped_pos);
        let start_id = start_point.id();

        // Add the start point to the sketch
        if let Some(sketch) = app.cad.get_sketch_mut(active_sketch_id) {
            sketch.add_entity(start_point);
        }

        // Start the line preview
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut() {
            sketch_state.in_progress = Some(InProgressEntity::Line {
                start_point: start_id,
                preview_end: snapped_pos,
            });
        }

        true
    } else if let Some(InProgressEntity::Line { start_point, .. }) =
        sketch_state.in_progress.clone()
    {
        // Second click: create end point and line
        let end_point = SketchEntity::point(snapped_pos);
        let end_id = end_point.id();

        let line = SketchEntity::line(start_point, end_id);

        // Add entities to sketch
        if let Some(sketch) = app.cad.get_sketch_mut(active_sketch_id) {
            sketch.add_entity(end_point);
            sketch.add_entity(line);
        }

        // Clear in-progress
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut() {
            sketch_state.in_progress = None;
        }

        true
    } else {
        false
    }
}

/// Handle rectangle tool click (corner mode).
/// First click sets the first corner.
/// Second click creates the rectangle.
fn handle_rectangle_tool_click(app_state: &SharedAppState, snapped_pos: Vec2) -> bool {
    let mut app = app_state.lock();

    let Some(sketch_state) = app.cad.editor_mode.sketch_mut() else {
        return false;
    };

    let active_sketch_id = sketch_state.active_sketch;

    if sketch_state.in_progress.is_none() {
        // First click: start rectangle preview
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut() {
            sketch_state.in_progress = Some(InProgressEntity::Rectangle {
                corner1: snapped_pos,
                preview_corner2: snapped_pos,
            });
        }
        true
    } else if let Some(InProgressEntity::Rectangle { corner1, .. }) =
        sketch_state.in_progress.clone()
    {
        // Second click: create rectangle
        let c1 = corner1;
        let c2 = snapped_pos;

        // Create four corner points
        let p1 = SketchEntity::point(c1); // bottom-left
        let p2 = SketchEntity::point(Vec2::new(c2.x, c1.y)); // bottom-right
        let p3 = SketchEntity::point(c2); // top-right
        let p4 = SketchEntity::point(Vec2::new(c1.x, c2.y)); // top-left

        let p1_id = p1.id();
        let p2_id = p2.id();
        let p3_id = p3.id();
        let p4_id = p4.id();

        // Create four lines
        let line1 = SketchEntity::line(p1_id, p2_id);
        let line2 = SketchEntity::line(p2_id, p3_id);
        let line3 = SketchEntity::line(p3_id, p4_id);
        let line4 = SketchEntity::line(p4_id, p1_id);

        // Add all entities
        if let Some(sketch) = app.cad.get_sketch_mut(active_sketch_id) {
            sketch.add_entity(p1);
            sketch.add_entity(p2);
            sketch.add_entity(p3);
            sketch.add_entity(p4);
            sketch.add_entity(line1);
            sketch.add_entity(line2);
            sketch.add_entity(line3);
            sketch.add_entity(line4);
        }

        // Clear in-progress
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut() {
            sketch_state.in_progress = None;
        }

        true
    } else {
        false
    }
}

/// Handle circle tool click (center-radius mode).
/// First click sets the center.
/// Second click sets the radius and creates the circle.
fn handle_circle_tool_click(app_state: &SharedAppState, snapped_pos: Vec2) -> bool {
    let mut app = app_state.lock();

    let Some(sketch_state) = app.cad.editor_mode.sketch_mut() else {
        return false;
    };

    let active_sketch_id = sketch_state.active_sketch;

    if sketch_state.in_progress.is_none() {
        // First click: create center point and start circle preview
        let center_point = SketchEntity::point(snapped_pos);
        let center_id = center_point.id();

        // Add center point to sketch
        if let Some(sketch) = app.cad.get_sketch_mut(active_sketch_id) {
            sketch.add_entity(center_point);
        }

        // Start circle preview
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut() {
            sketch_state.in_progress = Some(InProgressEntity::Circle {
                center_point: center_id,
                preview_radius: 0.0,
            });
        }

        true
    } else if let Some(InProgressEntity::Circle {
        center_point,
        preview_radius,
    }) = sketch_state.in_progress.clone()
    {
        // Second click: create circle
        if preview_radius > 0.001 {
            let circle = SketchEntity::circle(center_point, preview_radius);

            if let Some(sketch) = app.cad.get_sketch_mut(active_sketch_id) {
                sketch.add_entity(circle);
            }
        }

        // Clear in-progress
        if let Some(sketch_state) = app.cad.editor_mode.sketch_mut() {
            sketch_state.in_progress = None;
        }

        true
    } else {
        false
    }
}

/// Handle constraint tool click.
/// Picks entity at click position and queues SelectEntityForConstraint action.
fn handle_constraint_tool_click(
    app_state: &SharedAppState,
    sketch_pos: Vec2,
    sketch_id: uuid::Uuid,
    tool: SketchTool,
) -> bool {
    // Get sketch for entity picking
    let sketch = {
        let app = app_state.lock();
        app.cad.get_sketch(sketch_id).cloned()
    };

    let Some(sketch) = sketch else {
        return false;
    };

    // Pick entity at click position (use 0.5 units as pick radius)
    let picked_entity = pick_sketch_entity(&sketch, sketch_pos, 0.5);

    let Some(entity_id) = picked_entity else {
        return false;
    };

    // Check if entity type is valid for the constraint tool
    let entity = sketch.get_entity(entity_id);
    if !is_valid_entity_for_tool(entity, tool) {
        return false;
    }

    // Queue the action to process the entity selection
    app_state.lock().queue_action(AppAction::SketchAction(
        SketchAction::SelectEntityForConstraint { entity_id },
    ));

    true
}

/// Check if an entity type is valid for a constraint tool
fn is_valid_entity_for_tool(entity: Option<&SketchEntity>, tool: SketchTool) -> bool {
    let Some(entity) = entity else {
        return false;
    };

    match tool {
        SketchTool::ConstrainCoincident => matches!(entity, SketchEntity::Point { .. }),
        SketchTool::ConstrainHorizontal | SketchTool::ConstrainVertical => {
            matches!(entity, SketchEntity::Line { .. })
        }
        SketchTool::ConstrainParallel | SketchTool::ConstrainPerpendicular => {
            matches!(entity, SketchEntity::Line { .. })
        }
        SketchTool::ConstrainTangent => {
            matches!(
                entity,
                SketchEntity::Line { .. } | SketchEntity::Circle { .. } | SketchEntity::Arc { .. }
            )
        }
        SketchTool::ConstrainEqual => {
            matches!(
                entity,
                SketchEntity::Line { .. } | SketchEntity::Circle { .. }
            )
        }
        SketchTool::ConstrainFixed => matches!(entity, SketchEntity::Point { .. }),
        SketchTool::DimensionDistance
        | SketchTool::DimensionHorizontal
        | SketchTool::DimensionVertical => {
            matches!(
                entity,
                SketchEntity::Point { .. } | SketchEntity::Line { .. }
            )
        }
        SketchTool::DimensionAngle => matches!(entity, SketchEntity::Line { .. }),
        SketchTool::DimensionRadius => {
            matches!(
                entity,
                SketchEntity::Circle { .. } | SketchEntity::Arc { .. }
            )
        }
        _ => false,
    }
}

/// Convert ReferencePlane to plane_ids for the renderer.
fn reference_plane_to_id(plane: Option<ReferencePlane>) -> u32 {
    match plane {
        None => plane_ids::NONE,
        Some(ReferencePlane::XY) => plane_ids::XY,
        Some(ReferencePlane::XZ) => plane_ids::XZ,
        Some(ReferencePlane::YZ) => plane_ids::YZ,
    }
}

/// 3D viewport panel
pub struct ViewportPanel {
    last_size: egui::Vec2,
    hovered_axis: GizmoAxis,
    show_camera_settings: bool,
    /// Gizmo toolbar collapsed state
    gizmo_collapsed: bool,
    /// Camera toolbar collapsed state
    camera_collapsed: bool,
}

impl ViewportPanel {
    pub fn new() -> Self {
        Self {
            last_size: egui::Vec2::ZERO,
            hovered_axis: GizmoAxis::None,
            show_camera_settings: false,
            gizmo_collapsed: false,
            camera_collapsed: false,
        }
    }
}

impl Default for ViewportPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for ViewportPanel {
    fn name(&self) -> &str {
        "3D Viewport"
    }

    fn needs_render_context(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut egui::Ui, _app_state: &SharedAppState) {
        // Fallback when no render context
        let available_size = ui.available_size();
        let (response, painter) =
            ui.allocate_painter(available_size, egui::Sense::click_and_drag());

        painter.rect_filled(response.rect, 0.0, egui::Color32::from_rgb(30, 30, 30));
        painter.text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            "3D Viewport\n(WebGPU not available)",
            egui::FontId::proportional(16.0),
            egui::Color32::GRAY,
        );

        self.last_size = available_size;
    }

    fn ui_with_render_context(
        &mut self,
        ui: &mut egui::Ui,
        app_state: &SharedAppState,
        render_state: &egui_wgpu::RenderState,
        viewport_state: &SharedViewportState,
        config: &SharedConfig,
    ) {
        // Toolbar
        ui.horizontal(|ui| {
            ui.label("View:");
            if ui.button("Top").clicked() {
                viewport_state.lock().renderer.camera_mut().set_top_view();
            }
            if ui.button("Front").clicked() {
                viewport_state.lock().renderer.camera_mut().set_front_view();
            }
            if ui.button("Side").clicked() {
                viewport_state.lock().renderer.camera_mut().set_side_view();
            }
            if ui.button("Fit All").clicked() {
                viewport_state
                    .lock()
                    .renderer
                    .camera_mut()
                    .fit_all(Vec3::ZERO, 2.0);
            }

            ui.separator();

            let mut state = viewport_state.lock();
            let mut show_grid = state.renderer.show_grid();
            let mut show_axes = state.renderer.show_axes();
            let mut show_markers = state.renderer.show_markers();
            if ui.checkbox(&mut show_grid, "Grid").changed() {
                state.renderer.set_show_grid(show_grid);
            }
            if ui.checkbox(&mut show_axes, "Axes").changed() {
                state.renderer.set_show_axes(show_axes);
            }
            if ui.checkbox(&mut show_markers, "Markers").changed() {
                state.renderer.set_show_markers(show_markers);
            }
        });

        // Main viewport area
        let available_size = ui.available_size();
        let width = available_size.x as u32;
        let height = available_size.y as u32;

        if width == 0 || height == 0 {
            return;
        }

        // Ensure texture and render
        let texture_id = {
            let mut vp_state = viewport_state.lock();
            let mut egui_renderer = render_state.renderer.write();
            let tex_id = vp_state.ensure_texture(width, height, &mut egui_renderer);

            // Check if we're in plane selection mode
            let is_plane_selection_mode = {
                let app = app_state.lock();
                app.cad.editor_mode.is_plane_selection()
            };

            // Enable/disable plane selector based on editor mode
            vp_state
                .renderer
                .set_plane_selector_visible(is_plane_selection_mode);

            // Update sketch data for rendering
            let sketch_render_data: Vec<SketchRenderData> = {
                let app = app_state.lock();
                let cad = &app.cad;

                // Get selected entities and in-progress state if in sketch mode
                let (selected_entities, in_progress, active_sketch_id) =
                    if let Some(sketch_state) = cad.editor_mode.sketch() {
                        (
                            sketch_state.selected_entities.clone(),
                            sketch_state.in_progress.clone(),
                            Some(sketch_state.active_sketch),
                        )
                    } else {
                        (Vec::new(), None, None)
                    };

                // Convert all sketches to render data
                cad.data
                    .history
                    .sketches()
                    .values()
                    .map(|sketch| {
                        let is_active = active_sketch_id == Some(sketch.id);
                        let in_prog_ref = if is_active {
                            in_progress.as_ref()
                        } else {
                            None
                        };
                        sketch_to_render_data(sketch, &selected_entities, is_active, in_prog_ref)
                    })
                    .collect()
            };

            // Set sketch data and prepare GPU resources
            let device = vp_state.device.clone();
            vp_state.renderer.set_sketch_data(sketch_render_data);
            vp_state.renderer.prepare_sketches(&device);

            // Update preview mesh for extrude preview
            {
                let app = app_state.lock();
                if let Some(sketch_state) = app.cad.editor_mode.sketch() {
                    if sketch_state.extrude_dialog.open {
                        if let Some(ref preview_mesh) = sketch_state.extrude_dialog.preview_mesh {
                            // The preview mesh is already in world coordinates
                            // (kernel.extrude transforms profile using plane origin/axes)
                            // so we use identity transform here
                            vp_state.renderer.set_preview_mesh(
                                &device,
                                &preview_mesh.vertices,
                                &preview_mesh.normals,
                                &preview_mesh.indices,
                                Mat4::IDENTITY,
                            );
                        } else {
                            vp_state.renderer.clear_preview_mesh();
                        }
                    } else {
                        vp_state.renderer.clear_preview_mesh();
                    }
                } else {
                    vp_state.renderer.clear_preview_mesh();
                }
            }

            vp_state.render();
            tex_id
        };

        // Display the rendered texture
        let response = ui.add(
            egui::Image::new(egui::load::SizedTexture::new(
                texture_id,
                [available_size.x, available_size.y],
            ))
            .sense(egui::Sense::click_and_drag()),
        );

        // Get mouse position relative to viewport
        let mouse_pos = response.hover_pos().or(response.interact_pointer_pos());
        let local_mouse = mouse_pos.map(|p| p - response.rect.min);

        // Handle camera input
        let mut vp_state = viewport_state.lock();

        // Check if in plane selection mode
        let is_plane_selection = {
            let app = app_state.lock();
            app.cad.editor_mode.is_plane_selection()
        };

        // Handle plane selection mode
        if is_plane_selection && let Some(pos) = local_mouse {
            // Pick plane under cursor
            let camera = vp_state.renderer.camera();
            let hovered_plane =
                pick_reference_plane(camera, pos.x, pos.y, available_size.x, available_size.y);

            // Update highlight
            let plane_id = reference_plane_to_id(hovered_plane);
            let queue = vp_state.queue.clone();
            vp_state
                .renderer
                .set_plane_selector_highlighted(&queue, plane_id);

            // Handle click to select plane
            if response.clicked_by(egui::PointerButton::Primary)
                && let Some(plane) = hovered_plane
            {
                drop(vp_state);
                app_state.lock().queue_action(AppAction::SketchAction(
                    SketchAction::SelectPlaneAndCreateSketch { plane },
                ));
                vp_state = viewport_state.lock();
            }
        }

        // Check if in sketch mode
        let is_sketch_mode = {
            let app = app_state.lock();
            app.cad.editor_mode.is_sketch()
        };

        // Handle sketch mode input
        if is_sketch_mode {
            let sketch_input_result = handle_sketch_mode_input(
                &response,
                ui,
                local_mouse,
                available_size,
                app_state,
                &vp_state,
            );

            // If sketch mode consumed the click, skip normal interaction
            if sketch_input_result {
                // Continue to camera controls at the end
            }
        }

        // Gizmo interaction (left mouse button) - skip if in sketch mode
        let mut gizmo_delta: Option<GizmoTransform> = None;

        if !is_sketch_mode && let Some(pos) = local_mouse {
            // Check for gizmo hover
            if !vp_state.is_dragging_gizmo() {
                let hit_axis =
                    vp_state.gizmo_hit_test(pos.x, pos.y, available_size.x, available_size.y);
                if hit_axis != self.hovered_axis {
                    self.hovered_axis = hit_axis;
                    let queue = vp_state.queue.clone();
                    vp_state.renderer.set_gizmo_highlight(&queue, hit_axis);
                }
            }

            // Start drag on left click
            if response.drag_started_by(egui::PointerButton::Primary)
                && self.hovered_axis != GizmoAxis::None
            {
                vp_state.start_gizmo_drag(
                    self.hovered_axis,
                    pos.x,
                    pos.y,
                    available_size.x,
                    available_size.y,
                );
            }

            // Update drag
            if vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Primary) {
                gizmo_delta =
                    vp_state.update_gizmo_drag(pos.x, pos.y, available_size.x, available_size.y);
            }

            // End drag
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                vp_state.end_gizmo_drag();
            }

            // Object picking on click (only if not interacting with gizmo)
            if response.clicked_by(egui::PointerButton::Primary)
                && self.hovered_axis == GizmoAxis::None
            {
                // Gather pickable part data from app_state
                let pickable_parts: Vec<PickablePartData> = {
                    let app = app_state.lock();
                    app.project
                        .parts()
                        .values()
                        .map(|part| PickablePartData {
                            id: part.id,
                            vertices: part.vertices.clone(),
                            indices: part.indices.clone(),
                            transform: part.origin_transform,
                            bbox_min: part.bbox_min,
                            bbox_max: part.bbox_max,
                        })
                        .collect()
                };

                // Perform picking
                let camera = vp_state.renderer.camera();
                let hit = pick_object(
                    camera,
                    pos.x,
                    pos.y,
                    available_size.x,
                    available_size.y,
                    &pickable_parts,
                );

                // Queue selection action
                let selected_id = hit.map(|(id, _)| id);
                app_state
                    .lock()
                    .queue_action(AppAction::SelectPart(selected_id));
            }
        }

        // Apply gizmo transform to collision element
        if let Some(transform) = gizmo_delta
            && let Some((link_id, collision_index)) = vp_state.gizmo.editing_collision
        {
            let link_world_transform = vp_state.gizmo.link_world_transform;
            drop(vp_state);

            // Calculate the delta in link-local space
            let link_world_inv = link_world_transform.inverse();

            let mut app = app_state.lock();

            match transform {
                GizmoTransform::Translation(delta) => {
                    // Transform world delta to link-local delta
                    let local_delta = link_world_inv.transform_vector3(delta);

                    if let Some(link) = app.project.assembly.get_link_mut(link_id)
                        && let Some(collision) = link.collisions.get_mut(collision_index)
                    {
                        collision.origin.xyz[0] += local_delta.x;
                        collision.origin.xyz[1] += local_delta.y;
                        collision.origin.xyz[2] += local_delta.z;
                    }
                }
                GizmoTransform::Rotation(rotation) => {
                    // For rotation, we need to update the RPY angles
                    if let Some(link) = app.project.assembly.get_link_mut(link_id)
                        && let Some(collision) = link.collisions.get_mut(collision_index)
                    {
                        // Current rotation as quaternion
                        let current_quat = collision.origin.to_quat();
                        // Apply the rotation delta
                        let new_quat = rotation * current_quat;
                        // Convert back to euler angles (XYZ order)
                        let (x, y, z) = new_quat.to_euler(glam::EulerRot::XYZ);
                        collision.origin.rpy = [x, y, z];
                    }
                }
                GizmoTransform::Scale(_) => {
                    // Collision origins don't support scaling - ignore
                }
            }

            app.modified = true;
            drop(app);

            // Re-lock viewport state for rest of handling
            vp_state = viewport_state.lock();
        }
        // Apply gizmo transform to part
        else if let Some(transform) = gizmo_delta
            && let Some(part_id) = vp_state.gizmo.part_id
        {
            let queue = vp_state.queue.clone();
            drop(vp_state);

            let mut app = app_state.lock();

            match transform {
                GizmoTransform::Translation(delta) => {
                    // Moving the whole part - update part transform
                    let new_transform = if let Some(part) = app.get_part_mut(part_id) {
                        let (scale, rotation, translation) =
                            part.origin_transform.to_scale_rotation_translation();
                        let new_translation = translation + delta;
                        part.origin_transform = glam::Mat4::from_scale_rotation_translation(
                            scale,
                            rotation,
                            new_translation,
                        );
                        Some(part.origin_transform)
                    } else {
                        None
                    };
                    drop(app);

                    // Update mesh renderer transform
                    if let Some(transform) = new_transform {
                        let mut vp = viewport_state.lock();
                        vp.renderer
                            .update_part_transform(&queue, part_id, transform);
                        drop(vp);
                    }
                }
                GizmoTransform::Rotation(rotation) => {
                    // Rotating the whole part
                    let new_transform = if let Some(part) = app.get_part_mut(part_id) {
                        let (scale, old_rotation, translation) =
                            part.origin_transform.to_scale_rotation_translation();
                        let new_rotation = rotation * old_rotation;
                        part.origin_transform = glam::Mat4::from_scale_rotation_translation(
                            scale,
                            new_rotation,
                            translation,
                        );
                        Some(part.origin_transform)
                    } else {
                        None
                    };
                    drop(app);

                    // Update mesh renderer transform
                    if let Some(transform) = new_transform {
                        let mut vp = viewport_state.lock();
                        vp.renderer
                            .update_part_transform(&queue, part_id, transform);
                        drop(vp);
                    }
                }
                GizmoTransform::Scale(scale_delta) => {
                    // Scaling the whole part
                    let new_transform = if let Some(part) = app.get_part_mut(part_id) {
                        let (old_scale, rotation, translation) =
                            part.origin_transform.to_scale_rotation_translation();
                        let new_scale = old_scale * scale_delta;
                        part.origin_transform = glam::Mat4::from_scale_rotation_translation(
                            new_scale,
                            rotation,
                            translation,
                        );
                        Some(part.origin_transform)
                    } else {
                        None
                    };
                    drop(app);

                    // Update mesh renderer transform
                    if let Some(transform) = new_transform {
                        let mut vp = viewport_state.lock();
                        vp.renderer
                            .update_part_transform(&queue, part_id, transform);
                        drop(vp);
                    }
                }
            }

            // Re-lock viewport state for rest of handling
            vp_state = viewport_state.lock();
        }

        // Get camera sensitivity from config
        let (orbit_sens, pan_sens, zoom_sens) = {
            let cfg = config.read();
            let cam = &cfg.config().renderer.camera;
            (
                cam.orbit_sensitivity,
                cam.pan_sensitivity,
                cam.zoom_sensitivity,
            )
        };

        // Middle mouse button for orbit/pan (only if not dragging gizmo)
        if !vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            if ui.input(|i| i.modifiers.shift) {
                // Pan
                vp_state
                    .renderer
                    .camera_mut()
                    .pan_with_sensitivity(delta.x, delta.y, pan_sens);
            } else {
                // Orbit
                vp_state
                    .renderer
                    .camera_mut()
                    .orbit(-delta.x * orbit_sens, delta.y * orbit_sens);
            }
        }

        // Right mouse button for orbit as well
        if !vp_state.is_dragging_gizmo() && response.dragged_by(egui::PointerButton::Secondary) {
            let delta = response.drag_delta();
            vp_state
                .renderer
                .camera_mut()
                .orbit(-delta.x * orbit_sens, delta.y * orbit_sens);
        }

        // Zoom with scroll
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_delta != 0.0 {
                vp_state
                    .renderer
                    .camera_mut()
                    .zoom(scroll_delta * zoom_sens);
            }
        }

        // Gizmo mode keyboard shortcuts
        if response.hovered() {
            ui.input(|i| {
                if i.key_pressed(egui::Key::T) {
                    vp_state.renderer.set_gizmo_mode(GizmoMode::Translate);
                }
                if i.key_pressed(egui::Key::R) {
                    vp_state.renderer.set_gizmo_mode(GizmoMode::Rotate);
                }
                if i.key_pressed(egui::Key::S) {
                    vp_state.renderer.set_gizmo_mode(GizmoMode::Scale);
                }
                // Toggle coordinate space (G key)
                if i.key_pressed(egui::Key::G) {
                    let current_space = vp_state.renderer.gizmo_space();
                    let next_space = match current_space {
                        GizmoSpace::Global => GizmoSpace::Local,
                        GizmoSpace::Local => GizmoSpace::Global,
                    };
                    let queue = vp_state.queue.clone();
                    vp_state.renderer.set_gizmo_space(&queue, next_space);
                }
            });
        }

        // Context menu
        response.context_menu(|ui| {
            if ui.button("Reset View").clicked() {
                vp_state.renderer.camera_mut().fit_all(Vec3::ZERO, 2.0);
                ui.close();
            }
            ui.separator();
            if ui.button("Top View").clicked() {
                vp_state.renderer.camera_mut().set_top_view();
                ui.close();
            }
            if ui.button("Front View").clicked() {
                vp_state.renderer.camera_mut().set_front_view();
                ui.close();
            }
            if ui.button("Side View").clicked() {
                vp_state.renderer.camera_mut().set_side_view();
                ui.close();
            }
        });

        // Get camera state for axes indicator
        let yaw = vp_state.renderer.camera().yaw;
        let pitch = vp_state.renderer.camera().pitch;
        drop(vp_state);

        // Draw axes indicator overlay
        render_axes_indicator(ui, response.rect, yaw, pitch);

        // Draw gizmo mode toggle overlay (top-left) with slide animation
        render_gizmo_toggle(ui, response.rect, viewport_state, &mut self.gizmo_collapsed);

        // Draw camera settings overlay (top-right, Unity-style)
        render_camera_settings(
            ui,
            response.rect,
            viewport_state,
            &mut self.show_camera_settings,
            &mut self.camera_collapsed,
        );

        // Draw sketch toolbar (bottom-left) when in sketch mode
        {
            let app = app_state.lock();
            if let Some(sketch_state) = app.cad.editor_mode.sketch() {
                let current_tool = sketch_state.current_tool;
                let extrude_dialog_open = sketch_state.extrude_dialog.open;
                let dimension_dialog_open = sketch_state.dimension_dialog.open;
                drop(app); // Release lock before calling render
                render_sketch_toolbar(ui, response.rect, app_state, current_tool);

                // Draw extrude dialog if open
                if extrude_dialog_open {
                    render_extrude_dialog(ui, response.rect, app_state);
                }

                // Draw dimension dialog if open
                if dimension_dialog_open {
                    render_dimension_dialog(ui, response.rect, app_state);
                }
            }
        }

        // Draw plane selection hint when in plane selection mode
        {
            let app = app_state.lock();
            if app.cad.editor_mode.is_plane_selection() {
                drop(app);
                render_plane_selection_hint(ui, response.rect);
            }
        }

        self.last_size = available_size;
    }
}
