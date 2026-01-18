//! Sketch action handling
//!
//! Handles actions related to sketch editing and CAD operations.

use tracing::info;

use crate::state::{AppAction, EditorMode, PlaneSelectionState, ReferencePlane, SketchAction};

use super::ActionContext;

/// Handle sketch-related actions
pub fn handle_sketch_action(action: AppAction, ctx: &ActionContext) {
    let sketch_action = match action {
        AppAction::SketchAction(sa) => sa,
        _ => return,
    };

    match sketch_action {
        SketchAction::BeginPlaneSelection => {
            let mut state = ctx.app_state.lock();
            state.cad.editor_mode = EditorMode::PlaneSelection(PlaneSelectionState::default());
            info!("Entered plane selection mode");
        }

        SketchAction::CancelPlaneSelection => {
            let mut state = ctx.app_state.lock();
            state.cad.editor_mode = EditorMode::Assembly;
            info!("Cancelled plane selection");
        }

        SketchAction::SetHoveredPlane { plane } => {
            let mut state = ctx.app_state.lock();
            if let Some(plane_state) = state.cad.editor_mode.plane_selection_mut() {
                plane_state.hovered_plane = plane;
            }
        }

        SketchAction::SelectPlaneAndCreateSketch { plane } => {
            // 1. Move camera to the appropriate view
            if let Some(viewport_state) = ctx.viewport_state.as_ref() {
                let mut vp = viewport_state.lock();
                match plane {
                    ReferencePlane::XY => vp.renderer.camera_mut().set_top_view(),
                    ReferencePlane::XZ => vp.renderer.camera_mut().set_front_view(),
                    ReferencePlane::YZ => vp.renderer.camera_mut().set_side_view(),
                }
            }

            // 2. Create the sketch on the selected plane
            let sketch_plane = plane.to_sketch_plane();
            let mut state = ctx.app_state.lock();
            let sketch_id = state.cad.create_sketch("Sketch", sketch_plane);
            info!("Created sketch on {} plane: {}", plane.name(), sketch_id);

            // 3. Enter sketch mode
            state.cad.enter_sketch_mode(sketch_id);
        }

        SketchAction::CreateSketch { plane } => {
            let mut state = ctx.app_state.lock();
            let sketch_id = state.cad.create_sketch("Sketch", plane);
            info!("Created sketch: {}", sketch_id);
            // Automatically enter sketch mode for the new sketch
            state.cad.enter_sketch_mode(sketch_id);
        }

        SketchAction::EditSketch { sketch_id } => {
            let mut state = ctx.app_state.lock();
            if state.cad.get_sketch(sketch_id).is_some() {
                state.cad.enter_sketch_mode(sketch_id);
                info!("Entered sketch mode for: {}", sketch_id);
            } else {
                tracing::warn!("Sketch not found: {}", sketch_id);
            }
        }

        SketchAction::ExitSketchMode => {
            let mut state = ctx.app_state.lock();
            state.cad.exit_sketch_mode();
            info!("Exited sketch mode");
        }

        SketchAction::SetTool { tool } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.current_tool = tool;
                sketch_state.cancel_drawing(); // Cancel any in-progress drawing
            }
        }

        SketchAction::AddEntity { entity } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch() {
                let sketch_id = sketch_state.active_sketch;
                if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                    let entity_id = entity.id();
                    sketch.add_entity(entity);
                    info!("Added entity: {}", entity_id);
                }
            }
        }

        SketchAction::DeleteSelected => {
            let mut state = ctx.app_state.lock();
            let (sketch_id, selected) = {
                if let Some(sketch_state) = state.cad.editor_mode.sketch() {
                    (
                        sketch_state.active_sketch,
                        sketch_state.selected_entities.clone(),
                    )
                } else {
                    return;
                }
            };

            if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                for entity_id in &selected {
                    sketch.remove_entity(*entity_id);
                }
                info!("Deleted {} entities", selected.len());
            }

            // Clear selection
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.clear_selection();
            }
        }

        SketchAction::AddConstraint { constraint } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch() {
                let sketch_id = sketch_state.active_sketch;
                if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                    let constraint_id = constraint.id();
                    if let Err(e) = sketch.add_constraint(constraint) {
                        tracing::warn!("Failed to add constraint: {}", e);
                    } else {
                        info!("Added constraint: {}", constraint_id);
                    }
                }
            }
        }

        SketchAction::DeleteConstraint { constraint_id } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch() {
                let sketch_id = sketch_state.active_sketch;
                if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                    sketch.remove_constraint(constraint_id);
                    info!("Deleted constraint: {}", constraint_id);
                }
            }
        }

        SketchAction::SolveSketch => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch() {
                let sketch_id = sketch_state.active_sketch;
                if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                    let result = sketch.solve();
                    info!("Sketch solve result: {:?}", result);
                }
            }
        }

        SketchAction::ToggleSnap => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.snap_to_grid = !sketch_state.snap_to_grid;
            }
        }

        SketchAction::SetGridSpacing { spacing } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.grid_spacing = spacing;
            }
        }

        SketchAction::ShowExtrudeDialog => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                let sketch_id = sketch_state.active_sketch;
                sketch_state.extrude_dialog.open_for_sketch(sketch_id);
                info!("Opened extrude dialog for sketch: {}", sketch_id);
            }
        }

        SketchAction::UpdateExtrudeDistance { distance } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.distance = distance;
            }
        }

        SketchAction::UpdateExtrudeDirection { direction } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.direction = direction;
            }
        }

        SketchAction::CancelExtrudeDialog => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.close();
                info!("Cancelled extrude dialog");
            }
        }

        SketchAction::ExecuteExtrude => {
            let mut state = ctx.app_state.lock();

            // Get extrude parameters
            let (sketch_id, distance, direction) = {
                let Some(sketch_state) = state.cad.editor_mode.sketch() else {
                    return;
                };
                (
                    sketch_state.extrude_dialog.sketch_id,
                    sketch_state.extrude_dialog.distance,
                    sketch_state.extrude_dialog.direction,
                )
            };

            // Solve the sketch first
            if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                sketch.solve();
            }

            // TODO: Extract profiles from sketch and create extrusion feature
            // This would require:
            // 1. sketch.extract_profiles() - to get closed loops
            // 2. Feature::extrude(profile, distance, direction)
            // 3. history.add_feature(feature)
            // 4. history.rebuild() - to generate geometry

            info!(
                "Execute extrude: sketch={}, distance={}, direction={:?}",
                sketch_id, distance, direction
            );

            // Close the dialog and exit sketch mode
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.close();
            }

            // For now, just exit sketch mode
            state.cad.exit_sketch_mode();
            info!("Exited sketch mode after extrude");
        }
    }
}
