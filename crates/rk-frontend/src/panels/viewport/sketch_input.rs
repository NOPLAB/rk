//! Sketch input handling for the viewport
//!
//! Drawing keeps its preview state (`InProgressEntity`) purely in UI
//! state; the finished shape is committed to the engine as a single
//! `AddSketchEntities` command (one undo step, no orphan points on
//! cancel).

use glam::Vec2;
use rk_cad::{Sketch, SketchEntity, SketchPlane};
use rk_engine::Command;
use rk_renderer::Camera;

use crate::state::{
    AppAction, InProgressEntity, SharedAppState, SketchTool, SketchUiAction, ViewportState,
};

/// Convert screen coordinates to sketch 2D coordinates.
///
/// Returns None if the ray is parallel to the sketch plane or if the intersection
/// is behind the camera.
pub fn screen_to_sketch_coords(
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
pub fn pick_sketch_entity(
    sketch: &Sketch,
    sketch_pos: Vec2,
    pick_radius: f32,
) -> Option<uuid::Uuid> {
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

/// Handle sketch mode mouse input.
///
/// Returns true if the input was consumed by sketch mode.
pub fn handle_sketch_mode_input(
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

    // Get tool state from the UI and the plane from the engine
    let (engine, current_tool, snap_to_grid, grid_spacing, active_sketch_id) = {
        let app = app_state.lock();
        let Some(sketch_state) = app.editor_mode.sketch() else {
            return false;
        };
        (
            app.engine.clone(),
            sketch_state.current_tool,
            sketch_state.snap_to_grid,
            sketch_state.grid_spacing,
            sketch_state.active_sketch,
        )
    };
    let Some(sketch_plane) = engine.lock().sketch(active_sketch_id).map(|s| s.plane) else {
        return false;
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
        if let Some(sketch_state) = app.editor_mode.sketch_mut() {
            match &mut sketch_state.in_progress {
                Some(InProgressEntity::Line { preview_end, .. }) => {
                    *preview_end = snapped_pos;
                }
                Some(InProgressEntity::Circle {
                    center,
                    preview_radius,
                }) => {
                    *preview_radius = (*center - snapped_pos).length();
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

    // Escape or right-click cancels the in-progress drawing (nothing was
    // committed to the engine, so nothing is left behind)
    let cancel_requested = ui.input(|i| i.key_pressed(egui::Key::Escape))
        || response.clicked_by(egui::PointerButton::Secondary);
    if cancel_requested {
        let mut app = app_state.lock();
        if let Some(sketch_state) = app.editor_mode.sketch_mut()
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
                commit_entities(
                    app_state,
                    active_sketch_id,
                    vec![SketchEntity::point(snapped_pos)],
                );
                return true;
            }
            SketchTool::Line => {
                return handle_line_tool_click(app_state, active_sketch_id, snapped_pos);
            }
            SketchTool::RectangleCorner => {
                return handle_rectangle_tool_click(app_state, active_sketch_id, snapped_pos);
            }
            SketchTool::CircleCenterRadius => {
                return handle_circle_tool_click(app_state, active_sketch_id, snapped_pos);
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

/// Queue an atomic add of the given entities to the sketch
fn commit_entities(app_state: &SharedAppState, sketch_id: uuid::Uuid, entities: Vec<SketchEntity>) {
    app_state
        .lock()
        .queue_action(AppAction::Cmd(Command::AddSketchEntities {
            sketch_id,
            entities,
        }));
}

/// Run a closure over the in-progress state (if in sketch mode)
fn with_in_progress(app_state: &SharedAppState, f: impl FnOnce(&mut Option<InProgressEntity>)) {
    let mut app = app_state.lock();
    if let Some(sketch_state) = app.editor_mode.sketch_mut() {
        f(&mut sketch_state.in_progress);
    }
}

/// Line tool: first click starts the preview, second click commits
/// start point + end point + line as one command.
fn handle_line_tool_click(
    app_state: &SharedAppState,
    sketch_id: uuid::Uuid,
    snapped_pos: Vec2,
) -> bool {
    let in_progress = {
        let app = app_state.lock();
        app.editor_mode.sketch().and_then(|s| s.in_progress.clone())
    };

    match in_progress {
        None => {
            with_in_progress(app_state, |in_progress| {
                *in_progress = Some(InProgressEntity::Line {
                    start: snapped_pos,
                    preview_end: snapped_pos,
                });
            });
            true
        }
        Some(InProgressEntity::Line { start, .. }) => {
            let start_point = SketchEntity::point(start);
            let end_point = SketchEntity::point(snapped_pos);
            let line = SketchEntity::line(start_point.id(), end_point.id());
            commit_entities(app_state, sketch_id, vec![start_point, end_point, line]);
            with_in_progress(app_state, |in_progress| *in_progress = None);
            true
        }
        Some(_) => false,
    }
}

/// Rectangle tool: first click sets the corner, second click commits
/// 4 points + 4 lines as one command.
fn handle_rectangle_tool_click(
    app_state: &SharedAppState,
    sketch_id: uuid::Uuid,
    snapped_pos: Vec2,
) -> bool {
    let in_progress = {
        let app = app_state.lock();
        app.editor_mode.sketch().and_then(|s| s.in_progress.clone())
    };

    match in_progress {
        None => {
            with_in_progress(app_state, |in_progress| {
                *in_progress = Some(InProgressEntity::Rectangle {
                    corner1: snapped_pos,
                    preview_corner2: snapped_pos,
                });
            });
            true
        }
        Some(InProgressEntity::Rectangle { corner1, .. }) => {
            let c1 = corner1;
            let c2 = snapped_pos;

            let p1 = SketchEntity::point(c1); // bottom-left
            let p2 = SketchEntity::point(Vec2::new(c2.x, c1.y)); // bottom-right
            let p3 = SketchEntity::point(c2); // top-right
            let p4 = SketchEntity::point(Vec2::new(c1.x, c2.y)); // top-left

            let (p1_id, p2_id, p3_id, p4_id) = (p1.id(), p2.id(), p3.id(), p4.id());

            let entities = vec![
                p1,
                p2,
                p3,
                p4,
                SketchEntity::line(p1_id, p2_id),
                SketchEntity::line(p2_id, p3_id),
                SketchEntity::line(p3_id, p4_id),
                SketchEntity::line(p4_id, p1_id),
            ];
            commit_entities(app_state, sketch_id, entities);
            with_in_progress(app_state, |in_progress| *in_progress = None);
            true
        }
        Some(_) => false,
    }
}

/// Circle tool: first click sets the center, second click commits
/// center point + circle as one command.
fn handle_circle_tool_click(
    app_state: &SharedAppState,
    sketch_id: uuid::Uuid,
    snapped_pos: Vec2,
) -> bool {
    let in_progress = {
        let app = app_state.lock();
        app.editor_mode.sketch().and_then(|s| s.in_progress.clone())
    };

    match in_progress {
        None => {
            with_in_progress(app_state, |in_progress| {
                *in_progress = Some(InProgressEntity::Circle {
                    center: snapped_pos,
                    preview_radius: 0.0,
                });
            });
            true
        }
        Some(InProgressEntity::Circle {
            center,
            preview_radius,
        }) => {
            if preview_radius > 0.001 {
                let center_point = SketchEntity::point(center);
                let circle = SketchEntity::circle(center_point.id(), preview_radius);
                commit_entities(app_state, sketch_id, vec![center_point, circle]);
            }
            with_in_progress(app_state, |in_progress| *in_progress = None);
            true
        }
        Some(_) => false,
    }
}

/// Handle constraint tool click.
/// Picks entity at click position and queues SelectEntityForConstraint.
fn handle_constraint_tool_click(
    app_state: &SharedAppState,
    sketch_pos: Vec2,
    sketch_id: uuid::Uuid,
    tool: SketchTool,
) -> bool {
    // Snapshot the sketch for entity picking
    let engine = app_state.lock().engine.clone();
    let sketch = engine.lock().sketch(sketch_id).cloned();

    let Some(sketch) = sketch else {
        return false;
    };

    // Pick entity at click position (use 0.8 units as pick radius for easier selection)
    let picked_entity = pick_sketch_entity(&sketch, sketch_pos, 0.8);

    let Some(entity_id) = picked_entity else {
        return false;
    };

    // Check if entity type is valid for the constraint tool
    let entity = sketch.get_entity(entity_id);
    if !is_valid_entity_for_tool(entity, tool) {
        return false;
    }

    // Queue the action to process the entity selection
    app_state.lock().queue_action(AppAction::SketchUi(
        SketchUiAction::SelectEntityForConstraint { entity_id },
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
