//! Sketch action handling
//!
//! Handles actions related to sketch editing and CAD operations.

use std::collections::HashMap;

use glam::Vec2;
use tracing::info;

use rk_cad::{CadKernel, Sketch, SketchConstraint, SketchEntity, TessellatedMesh, Wire2D};

use crate::state::{
    AppAction, ConstraintToolState, EditorMode, ExtrudeDirection, PlaneSelectionState,
    ReferencePlane, SketchAction, SketchTool,
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
            sketch.plane.x_axis,
            sketch.plane.y_axis,
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
                sketch.plane.x_axis,
                sketch.plane.y_axis,
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
            // First, extract profiles from the sketch and get available bodies
            let (profiles, available_bodies) = {
                let mut state = ctx.app_state.lock();
                let sketch_id = state
                    .cad
                    .editor_mode
                    .sketch()
                    .map(|s| s.active_sketch)
                    .unwrap_or_default();

                // Extract profiles
                let profiles = if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
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
                };

                // Get available bodies for boolean operations
                let available_bodies: Vec<(uuid::Uuid, String)> = state
                    .cad
                    .data
                    .history
                    .bodies()
                    .iter()
                    .map(|(id, body)| (*id, body.name.clone()))
                    .collect();

                (profiles, available_bodies)
            };

            // Now open the dialog and set profiles
            {
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    let sketch_id = sketch_state.active_sketch;
                    sketch_state.extrude_dialog.open_for_sketch(sketch_id);
                    sketch_state.extrude_dialog.set_profiles(profiles);
                    sketch_state.extrude_dialog.available_bodies = available_bodies;
                    info!(
                        "Opened extrude dialog for sketch: {} with {} profiles, {} available bodies",
                        sketch_id,
                        sketch_state.extrude_dialog.profiles.len(),
                        sketch_state.extrude_dialog.available_bodies.len()
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

        SketchAction::UpdateExtrudeBooleanOp { boolean_op } => {
            {
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.extrude_dialog.boolean_op = boolean_op;
                    // Clear target_body if switching to New
                    if boolean_op == rk_cad::BooleanOp::New {
                        sketch_state.extrude_dialog.target_body = None;
                    }
                }
            }
            // Regenerate preview with new boolean op
            regenerate_preview(ctx);
        }

        SketchAction::UpdateExtrudeTargetBody { target_body } => {
            {
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.extrude_dialog.target_body = target_body;
                }
            }
            // Regenerate preview with new target body
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
            let (sketch_id, distance, direction, profile_index, boolean_op, target_body) = {
                let Some(sketch_state) = state.cad.editor_mode.sketch() else {
                    return;
                };
                (
                    sketch_state.extrude_dialog.sketch_id,
                    sketch_state.extrude_dialog.distance,
                    sketch_state.extrude_dialog.direction,
                    sketch_state.extrude_dialog.selected_profile_index,
                    sketch_state.extrude_dialog.boolean_op,
                    sketch_state.extrude_dialog.target_body,
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

                // Create the extrude feature with boolean operation
                let feature = rk_cad::Feature::extrude_with_boolean(
                    "Extrude",
                    sketch_id,
                    distance,
                    feature_direction,
                    boolean_op,
                    target_body,
                );

                // Add the feature to history
                state.cad.data.history.add_feature(feature.clone());

                info!(
                    "Execute extrude: sketch={}, distance={}, direction={:?}, boolean_op={:?}, target_body={:?}",
                    sketch_id, distance, direction, boolean_op, target_body
                );

                // Rebuild the history to execute the feature
                if let Err(e) = state.cad.data.history.rebuild(ctx.kernel.as_ref()) {
                    tracing::error!("Failed to rebuild after extrude: {}", e);
                } else {
                    // Log success
                    let body_count = state.cad.data.history.bodies().len();
                    info!("Rebuild complete. Total bodies: {}", body_count);

                    // Sync CAD bodies to renderer
                    if let Some(viewport_state) = ctx.viewport_state.as_ref() {
                        let mut vp = viewport_state.lock();
                        let device = vp.device.clone();
                        vp.renderer.clear_cad_bodies();

                        for (body_id, body) in state.cad.data.history.bodies() {
                            if let Some(ref solid) = body.solid {
                                // Tessellate the body
                                match ctx.kernel.tessellate(solid, 0.1) {
                                    Ok(mesh) => {
                                        let transform = glam::Mat4::IDENTITY;
                                        let color = [0.7, 0.7, 0.8, 1.0]; // Default gray-blue color
                                        vp.renderer.add_cad_body(
                                            &device,
                                            *body_id,
                                            &mesh.vertices,
                                            &mesh.normals,
                                            &mesh.indices,
                                            transform,
                                            color,
                                        );
                                        info!("Added CAD body to renderer: {}", body_id);
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to tessellate body {}: {}",
                                            body_id,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
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

        SketchAction::SelectEntityForConstraint { entity_id } => {
            handle_select_entity_for_constraint(ctx, entity_id);
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
            handle_confirm_dimension_constraint(ctx);
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
                    if let Some(viewport_state) = ctx.viewport_state.as_ref() {
                        let mut vp = viewport_state.lock();
                        let device = vp.device.clone();
                        vp.renderer.clear_cad_bodies();

                        for (body_id, body) in state.cad.data.history.bodies() {
                            if let Some(ref solid) = body.solid {
                                match ctx.kernel.tessellate(solid, 0.1) {
                                    Ok(mesh) => {
                                        let transform = glam::Mat4::IDENTITY;
                                        let color = [0.7, 0.7, 0.8, 1.0];
                                        vp.renderer.add_cad_body(
                                            &device,
                                            *body_id,
                                            &mesh.vertices,
                                            &mesh.normals,
                                            &mesh.indices,
                                            transform,
                                            color,
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to tessellate body {}: {}",
                                            body_id,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                tracing::warn!("Feature not found for deletion: {}", feature_id);
            }
        }

        SketchAction::ToggleFeatureSuppression { feature_id } => {
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
                    if let Some(viewport_state) = ctx.viewport_state.as_ref() {
                        let mut vp = viewport_state.lock();
                        let device = vp.device.clone();
                        vp.renderer.clear_cad_bodies();

                        for (body_id, body) in state.cad.data.history.bodies() {
                            if let Some(ref solid) = body.solid {
                                match ctx.kernel.tessellate(solid, 0.1) {
                                    Ok(mesh) => {
                                        let transform = glam::Mat4::IDENTITY;
                                        let color = [0.7, 0.7, 0.8, 1.0];
                                        vp.renderer.add_cad_body(
                                            &device,
                                            *body_id,
                                            &mesh.vertices,
                                            &mesh.normals,
                                            &mesh.indices,
                                            transform,
                                            color,
                                        );
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to tessellate body {}: {}",
                                            body_id,
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                tracing::warn!("Feature not found for suppression toggle: {}", feature_id);
            }
        }
    }
}

/// Handle entity selection for constraint tools
fn handle_select_entity_for_constraint(ctx: &ActionContext, entity_id: uuid::Uuid) {
    // Extract data needed for constraint creation first (immutable borrow)
    let (tool, sketch_id, _constraint_state, is_waiting_for_second, first_entity_opt) = {
        let state = ctx.app_state.lock();
        let Some(sketch_state) = state.cad.editor_mode.sketch() else {
            return;
        };
        let first_entity = match &sketch_state.constraint_tool_state {
            Some(ConstraintToolState::WaitingForSecond { first_entity }) => Some(*first_entity),
            _ => None,
        };
        let is_waiting = matches!(
            &sketch_state.constraint_tool_state,
            Some(ConstraintToolState::WaitingForSecond { .. })
        );
        (
            sketch_state.current_tool,
            sketch_state.active_sketch,
            sketch_state.constraint_tool_state.clone(),
            is_waiting,
            first_entity,
        )
    };

    // Handle based on state
    if is_waiting_for_second {
        let first = first_entity_opt.unwrap();
        if first == entity_id {
            return; // Can't constrain to self
        }

        if is_dimensional_constraint(tool) {
            // Need value input - compute initial value first
            let initial_value = {
                let state = ctx.app_state.lock();
                compute_initial_value(tool, &[first, entity_id], &state.cad, sketch_id)
            };

            // Now open dialog
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.dimension_dialog.open_for_constraint(
                    tool,
                    vec![first, entity_id],
                    initial_value,
                );
                sketch_state.constraint_tool_state = None;
                sketch_state.clear_selection();
            }
        } else {
            // Create geometric constraint immediately
            if let Some(constraint) = create_two_entity_constraint(tool, first, entity_id) {
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.constraint_tool_state = None;
                    sketch_state.clear_selection();
                }

                // Add constraint and solve
                if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                    if let Err(e) = sketch.add_constraint(constraint) {
                        tracing::warn!("Failed to add constraint: {}", e);
                    } else {
                        sketch.solve();
                        info!("Added two-entity constraint and solved sketch");
                    }
                }
            }
        }
    } else {
        // First selection

        // Check if selected entity is a Line for dimensional constraint (line length)
        if is_dimensional_constraint(tool) {
            let line_endpoints = {
                let state = ctx.app_state.lock();
                if let Some(sketch) = state.cad.get_sketch(sketch_id) {
                    if let Some(SketchEntity::Line { start, end, .. }) =
                        sketch.get_entity(entity_id)
                    {
                        Some((*start, *end))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some((start, end)) = line_endpoints {
                // Line selected - use its endpoints for distance constraint
                let initial_value = {
                    let state = ctx.app_state.lock();
                    compute_initial_value(tool, &[start, end], &state.cad, sketch_id)
                };

                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.dimension_dialog.open_for_constraint(
                        tool,
                        vec![start, end],
                        initial_value,
                    );
                    sketch_state.constraint_tool_state = None;
                }
                return;
            }
        }

        if is_single_entity_constraint(tool) {
            // Try to create constraint (need immutable access to CadState)
            let constraint = {
                let state = ctx.app_state.lock();
                create_single_entity_constraint(tool, entity_id, &state.cad, sketch_id)
            };

            if let Some(constraint) = constraint {
                // Reset state and add constraint
                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.constraint_tool_state = None;
                    sketch_state.clear_selection();
                }

                if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
                    if let Err(e) = sketch.add_constraint(constraint) {
                        tracing::warn!("Failed to add constraint: {}", e);
                    } else {
                        sketch.solve();
                        info!("Added single-entity constraint and solved sketch");
                    }
                }
            } else if is_dimensional_single_entity(tool) {
                // Need value input for dimensional constraint
                let initial_value = {
                    let state = ctx.app_state.lock();
                    compute_initial_value(tool, &[entity_id], &state.cad, sketch_id)
                };

                let mut state = ctx.app_state.lock();
                if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                    sketch_state.dimension_dialog.open_for_constraint(
                        tool,
                        vec![entity_id],
                        initial_value,
                    );
                    sketch_state.constraint_tool_state = None;
                }
            }
        } else {
            // Wait for second selection
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.constraint_tool_state = Some(ConstraintToolState::WaitingForSecond {
                    first_entity: entity_id,
                });
                sketch_state.select_entity(entity_id);
            }
        }
    }
}

/// Handle confirmation of dimension constraint from dialog
fn handle_confirm_dimension_constraint(ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    let Some(sketch_state) = state.cad.editor_mode.sketch_mut() else {
        return;
    };

    let tool = sketch_state.dimension_dialog.tool;
    let entities = sketch_state.dimension_dialog.entities.clone();
    let value = sketch_state.dimension_dialog.value;
    let sketch_id = sketch_state.active_sketch;

    // Close dialog
    sketch_state.dimension_dialog.close();
    sketch_state.clear_selection();

    if let Some(tool) = tool
        && let Some(constraint) = create_dimensional_constraint(tool, &entities, value)
        && let Some(sketch) = state.cad.get_sketch_mut(sketch_id)
    {
        if let Err(e) = sketch.add_constraint(constraint) {
            tracing::warn!("Failed to add dimensional constraint: {}", e);
        } else {
            sketch.solve();
            info!("Added dimensional constraint and solved sketch");
        }
    }
}

/// Check if a tool creates a single-entity constraint
fn is_single_entity_constraint(tool: SketchTool) -> bool {
    matches!(
        tool,
        SketchTool::ConstrainHorizontal
            | SketchTool::ConstrainVertical
            | SketchTool::ConstrainFixed
    )
}

fn is_dimensional_constraint(tool: SketchTool) -> bool {
    tool.is_dimension()
}

fn is_dimensional_single_entity(tool: SketchTool) -> bool {
    matches!(tool, SketchTool::DimensionRadius)
}

/// Create a single-entity geometric constraint
fn create_single_entity_constraint(
    tool: SketchTool,
    entity_id: uuid::Uuid,
    cad: &crate::state::CadState,
    sketch_id: uuid::Uuid,
) -> Option<SketchConstraint> {
    match tool {
        SketchTool::ConstrainHorizontal => Some(SketchConstraint::horizontal(entity_id)),
        SketchTool::ConstrainVertical => Some(SketchConstraint::vertical(entity_id)),
        SketchTool::ConstrainFixed => {
            // Get current position
            let sketch = cad.get_sketch(sketch_id)?;
            let entity = sketch.get_entity(entity_id)?;
            if let SketchEntity::Point { position, .. } = entity {
                Some(SketchConstraint::fixed(entity_id, position.x, position.y))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Create a two-entity geometric constraint
fn create_two_entity_constraint(
    tool: SketchTool,
    entity1: uuid::Uuid,
    entity2: uuid::Uuid,
) -> Option<SketchConstraint> {
    match tool {
        SketchTool::ConstrainCoincident => Some(SketchConstraint::coincident(entity1, entity2)),
        SketchTool::ConstrainParallel => Some(SketchConstraint::parallel(entity1, entity2)),
        SketchTool::ConstrainPerpendicular => {
            Some(SketchConstraint::perpendicular(entity1, entity2))
        }
        SketchTool::ConstrainTangent => Some(SketchConstraint::tangent(entity1, entity2)),
        SketchTool::ConstrainEqual => {
            // Assume equal length for lines (could be enhanced to detect type)
            Some(SketchConstraint::equal_length(entity1, entity2))
        }
        _ => None,
    }
}

/// Create a dimensional constraint with a value
fn create_dimensional_constraint(
    tool: SketchTool,
    entities: &[uuid::Uuid],
    value: f32,
) -> Option<SketchConstraint> {
    match tool {
        SketchTool::DimensionDistance => {
            if entities.len() >= 2 {
                Some(SketchConstraint::distance(entities[0], entities[1], value))
            } else {
                None
            }
        }
        SketchTool::DimensionHorizontal => {
            if entities.len() >= 2 {
                Some(SketchConstraint::horizontal_distance(
                    entities[0],
                    entities[1],
                    value,
                ))
            } else {
                None
            }
        }
        SketchTool::DimensionVertical => {
            if entities.len() >= 2 {
                Some(SketchConstraint::vertical_distance(
                    entities[0],
                    entities[1],
                    value,
                ))
            } else {
                None
            }
        }
        SketchTool::DimensionAngle => {
            if entities.len() >= 2 {
                Some(SketchConstraint::angle(
                    entities[0],
                    entities[1],
                    value.to_radians(),
                ))
            } else {
                None
            }
        }
        SketchTool::DimensionRadius => {
            if !entities.is_empty() {
                Some(SketchConstraint::radius(entities[0], value))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Compute initial value for dimensional constraints based on current geometry
fn compute_initial_value(
    tool: SketchTool,
    entities: &[uuid::Uuid],
    cad: &crate::state::CadState,
    sketch_id: uuid::Uuid,
) -> f32 {
    let sketch = match cad.get_sketch(sketch_id) {
        Some(s) => s,
        None => return 10.0,
    };

    // Get point positions
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

    match tool {
        SketchTool::DimensionDistance => {
            if entities.len() >= 2
                && let (Some(&p1), Some(&p2)) = (
                    point_positions.get(&entities[0]),
                    point_positions.get(&entities[1]),
                )
            {
                return (p2 - p1).length();
            }
            10.0
        }
        SketchTool::DimensionHorizontal => {
            if entities.len() >= 2
                && let (Some(&p1), Some(&p2)) = (
                    point_positions.get(&entities[0]),
                    point_positions.get(&entities[1]),
                )
            {
                return (p2.x - p1.x).abs();
            }
            10.0
        }
        SketchTool::DimensionVertical => {
            if entities.len() >= 2
                && let (Some(&p1), Some(&p2)) = (
                    point_positions.get(&entities[0]),
                    point_positions.get(&entities[1]),
                )
            {
                return (p2.y - p1.y).abs();
            }
            10.0
        }
        SketchTool::DimensionRadius => {
            if !entities.is_empty()
                && let Some(SketchEntity::Circle { radius, .. } | SketchEntity::Arc { radius, .. }) =
                    sketch.get_entity(entities[0])
            {
                return *radius;
            }
            5.0
        }
        SketchTool::DimensionAngle => 90.0, // Default 90 degrees
        _ => 10.0,
    }
}
