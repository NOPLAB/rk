//! Sketch action handling
//!
//! Handles actions related to sketch editing and CAD operations.

mod constraints;
mod extrude;

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

        // Extrude actions
        SketchAction::ShowExtrudeDialog => {
            extrude::handle_show_extrude_dialog(ctx);
        }

        SketchAction::UpdateExtrudeDistance { distance } => {
            extrude::handle_update_extrude_distance(ctx, distance);
        }

        SketchAction::UpdateExtrudeDirection { direction } => {
            extrude::handle_update_extrude_direction(ctx, direction);
        }

        SketchAction::UpdateExtrudeBooleanOp { boolean_op } => {
            extrude::handle_update_extrude_boolean_op(ctx, boolean_op);
        }

        SketchAction::UpdateExtrudeTargetBody { target_body } => {
            extrude::handle_update_extrude_target_body(ctx, target_body);
        }

        SketchAction::ToggleExtrudeProfile { profile_index } => {
            extrude::handle_toggle_extrude_profile(ctx, profile_index);
        }

        SketchAction::CancelExtrudeDialog => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.close();
                info!("Cancelled extrude dialog");
            }
        }

        SketchAction::ExecuteExtrude => {
            extrude::handle_execute_extrude(ctx);
        }

        // Constraint actions
        SketchAction::SelectEntityForConstraint { entity_id } => {
            constraints::handle_select_entity_for_constraint(ctx, entity_id);
        }

        SketchAction::CancelConstraintSelection => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.constraint_tool_state = None;
                sketch_state.clear_selection();
            }
        }

        SketchAction::ShowDimensionDialog {
            tool,
            entities,
            initial_value,
        } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state
                    .dimension_dialog
                    .open_for_constraint(tool, entities, initial_value);
            }
        }

        SketchAction::UpdateDimensionValue { value } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.dimension_dialog.value = value;
                sketch_state.dimension_dialog.value_text = format!("{:.2}", value);
            }
        }

        SketchAction::ConfirmDimensionConstraint => {
            constraints::handle_confirm_dimension_constraint(ctx);
        }

        SketchAction::CancelDimensionDialog => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.dimension_dialog.close();
                sketch_state.clear_selection();
            }
        }

        SketchAction::DeleteSketch { sketch_id } => {
            let mut state = ctx.app_state.lock();

            // Exit sketch mode if we're editing this sketch
            if let Some(sketch_state) = state.cad.editor_mode.sketch()
                && sketch_state.active_sketch == sketch_id
            {
                state.cad.exit_sketch_mode();
                info!("Exited sketch mode before deleting sketch");
            }

            // Remove the sketch from history
            if state.cad.data.history.remove_sketch(sketch_id).is_some() {
                info!("Deleted sketch: {}", sketch_id);
                state.modified = true;
            } else {
                tracing::warn!("Sketch not found for deletion: {}", sketch_id);
            }
        }

        SketchAction::DeleteFeature { feature_id } => {
            handle_delete_feature(ctx, feature_id);
        }

        SketchAction::ToggleFeatureSuppression { feature_id } => {
            handle_toggle_feature_suppression(ctx, feature_id);
        }
    }
}

/// Handle feature deletion
fn handle_delete_feature(ctx: &ActionContext, feature_id: uuid::Uuid) {
    let mut state = ctx.app_state.lock();

    // Remove the feature from history
    if state.cad.data.history.remove_feature(feature_id).is_some() {
        info!("Deleted feature: {}", feature_id);
        state.modified = true;

        // Rebuild geometry after feature deletion
        if let Err(e) = state.cad.data.history.rebuild(ctx.kernel.as_ref()) {
            tracing::error!("Failed to rebuild after feature deletion: {}", e);
        } else {
            info!("Rebuild complete after feature deletion");

            // Sync CAD bodies to renderer
            extrude::sync_cad_bodies_to_renderer(ctx, &state);
        }
    } else {
        tracing::warn!("Feature not found for deletion: {}", feature_id);
    }
}

/// Handle feature suppression toggle
fn handle_toggle_feature_suppression(ctx: &ActionContext, feature_id: uuid::Uuid) {
    let mut state = ctx.app_state.lock();

    // Toggle suppression state
    if let Some(feature) = state.cad.data.history.get_by_id_mut(feature_id) {
        let new_state = !feature.is_suppressed();
        feature.set_suppressed(new_state);
        info!(
            "Toggled feature {} suppression to {}",
            feature_id, new_state
        );
        state.modified = true;

        // Rebuild geometry after suppression change
        if let Err(e) = state.cad.data.history.rebuild(ctx.kernel.as_ref()) {
            tracing::error!("Failed to rebuild after suppression toggle: {}", e);
        } else {
            info!("Rebuild complete after suppression toggle");

            // Sync CAD bodies to renderer
            extrude::sync_cad_bodies_to_renderer(ctx, &state);
        }
    } else {
        tracing::warn!("Feature not found for suppression toggle: {}", feature_id);
    }
}
