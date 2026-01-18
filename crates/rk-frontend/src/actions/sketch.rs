//! Sketch action handling
//!
//! Handles actions related to sketch editing and CAD operations.

use tracing::info;

use rk_cad::{CadKernel, Sketch, TessellatedMesh, Wire2D};

use crate::state::{
    AppAction, EditorMode, ExtrudeDirection, PlaneSelectionState, ReferencePlane, SketchAction,
};

use super::ActionContext;

/// Generate a preview mesh for extrusion.
///
/// Returns Some(TessellatedMesh) if successful, None if failed.
fn generate_extrude_preview(
    kernel: &dyn CadKernel,
    sketch: &Sketch,
    profile: &Wire2D,
    distance: f32,
    direction: ExtrudeDirection,
) -> Result<TessellatedMesh, String> {
    if !kernel.is_available() {
        return Err("CAD kernel not available".to_string());
    }

    // Calculate extrusion direction vector
    let extrude_dir = match direction {
        ExtrudeDirection::Positive => sketch.plane.normal,
        ExtrudeDirection::Negative => -sketch.plane.normal,
        ExtrudeDirection::Symmetric => sketch.plane.normal,
    };

    // For symmetric, we extrude half in each direction
    let extrude_dist = match direction {
        ExtrudeDirection::Symmetric => distance / 2.0,
        _ => distance,
    };

    // Perform extrusion
    let solid = kernel
        .extrude(
            profile,
            sketch.plane.origin,
            sketch.plane.normal,
            extrude_dir,
            extrude_dist,
        )
        .map_err(|e| format!("Extrude failed: {}", e))?;

    // For symmetric, extrude in the opposite direction and union
    let solid = if matches!(direction, ExtrudeDirection::Symmetric) {
        let solid2 = kernel
            .extrude(
                profile,
                sketch.plane.origin,
                sketch.plane.normal,
                -extrude_dir,
                extrude_dist,
            )
            .map_err(|e| format!("Symmetric extrude failed: {}", e))?;

        kernel
            .boolean(&solid, &solid2, rk_cad::BooleanType::Union)
            .map_err(|e| format!("Boolean union failed: {}", e))?
    } else {
        solid
    };

    // Tessellate for preview
    let mesh = kernel
        .tessellate(&solid, 0.1)
        .map_err(|e| format!("Tessellation failed: {}", e))?;

    Ok(mesh)
}

/// Regenerate preview mesh for the current extrude dialog state.
/// This should be called after profile selection or parameter changes.
fn regenerate_preview(ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    let Some(sketch_state) = state.cad.editor_mode.sketch_mut() else {
        return;
    };

    if !sketch_state.extrude_dialog.open {
        return;
    }

    let sketch_id = sketch_state.extrude_dialog.sketch_id;
    let distance = sketch_state.extrude_dialog.distance;
    let direction = sketch_state.extrude_dialog.direction;
    let profile_index = sketch_state.extrude_dialog.selected_profile_index;

    // Get the profile
    let profile = sketch_state
        .extrude_dialog
        .profiles
        .get(profile_index)
        .cloned();

    // Get the sketch plane info (need to drop state to avoid borrow issues)
    let sketch_info = state.cad.get_sketch(sketch_id).cloned();

    if let (Some(profile), Some(sketch)) = (profile, sketch_info) {
        // Generate preview
        match generate_extrude_preview(ctx.kernel.as_ref(), &sketch, &profile, distance, direction)
        {
            Ok(mesh) => {
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.extrude_dialog.preview_mesh = Some(mesh);
                    sketch_state.extrude_dialog.error_message = None;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to generate extrude preview: {}", e);
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.extrude_dialog.preview_mesh = None;
                    sketch_state.extrude_dialog.error_message = Some(e);
                }
            }
        }
    } else {
        // No profile selected or sketch not found
        if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
            sketch_state.extrude_dialog.preview_mesh = None;
            if sketch_state.extrude_dialog.profiles.is_empty() {
                sketch_state.extrude_dialog.error_message =
                    Some("No closed profiles found in sketch".to_string());
            }
        }
    }
}

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
            // First, extract profiles from the sketch
            let profiles = {
                let mut state = ctx.app_state.lock();
                let sketch_id = state
                    .cad
                    .editor_mode
                    .sketch()
                    .map(|s| s.active_sketch)
                    .unwrap_or_default();

                if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                    // Solve the sketch first
                    sketch.solve();
                    // Extract profiles
                    match sketch.extract_profiles() {
                        Ok(profiles) => profiles,
                        Err(e) => {
                            tracing::warn!("Failed to extract profiles: {}", e);
                            Vec::new()
                        }
                    }
                } else {
                    Vec::new()
                }
            };

            // Now open the dialog and set profiles
            {
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    let sketch_id = sketch_state.active_sketch;
                    sketch_state.extrude_dialog.open_for_sketch(sketch_id);
                    sketch_state.extrude_dialog.set_profiles(profiles);
                    info!(
                        "Opened extrude dialog for sketch: {} with {} profiles",
                        sketch_id,
                        sketch_state.extrude_dialog.profiles.len()
                    );
                }
            }

            // Generate initial preview
            regenerate_preview(ctx);
        }

        SketchAction::UpdateExtrudeDistance { distance } => {
            {
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.extrude_dialog.distance = distance;
                }
            }
            // Regenerate preview with new distance
            regenerate_preview(ctx);
        }

        SketchAction::UpdateExtrudeDirection { direction } => {
            {
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.extrude_dialog.direction = direction;
                }
            }
            // Regenerate preview with new direction
            regenerate_preview(ctx);
        }

        SketchAction::SelectExtrudeProfile { profile_index } => {
            {
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.extrude_dialog.select_profile(profile_index);
                    info!("Selected profile index: {}", profile_index);
                }
            }
            // Regenerate preview with new profile
            regenerate_preview(ctx);
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
            let (sketch_id, distance, direction, profile_index) = {
                let Some(sketch_state) = state.cad.editor_mode.sketch() else {
                    return;
                };
                (
                    sketch_state.extrude_dialog.sketch_id,
                    sketch_state.extrude_dialog.distance,
                    sketch_state.extrude_dialog.direction,
                    sketch_state.extrude_dialog.selected_profile_index,
                )
            };

            // Solve the sketch first
            if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                sketch.solve();
            }

            // Get the profile
            let profile = {
                let sketch_state = state.cad.editor_mode.sketch();
                sketch_state.and_then(|s| s.extrude_dialog.profiles.get(profile_index).cloned())
            };

            // Create and execute the extrude feature
            if let Some(_profile) = profile
                && let Some(_sketch) = state.cad.get_sketch(sketch_id).cloned()
            {
                // Convert ExtrudeDirection from state to feature direction
                let feature_direction = match direction {
                    ExtrudeDirection::Positive => rk_cad::ExtrudeDirection::Positive,
                    ExtrudeDirection::Negative => rk_cad::ExtrudeDirection::Negative,
                    ExtrudeDirection::Symmetric => rk_cad::ExtrudeDirection::Symmetric,
                };

                // Create the extrude feature
                let feature =
                    rk_cad::Feature::extrude("Extrude", sketch_id, distance, feature_direction);

                // Add the feature to history
                state.cad.data.history.add_feature(feature.clone());

                info!(
                    "Execute extrude: sketch={}, distance={}, direction={:?}, profile_index={}",
                    sketch_id, distance, direction, profile_index
                );

                // Rebuild the history to execute the feature
                if let Err(e) = state.cad.data.history.rebuild(ctx.kernel.as_ref()) {
                    tracing::error!("Failed to rebuild after extrude: {}", e);
                } else {
                    // Log success
                    let body_count = state.cad.data.history.bodies().len();
                    info!("Rebuild complete. Total bodies: {}", body_count);
                }
            } else {
                tracing::warn!("No profile selected for extrusion");
            }

            // Close the dialog and exit sketch mode
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.close();
            }

            state.cad.exit_sketch_mode();
            info!("Exited sketch mode after extrude");
        }
    }
}
