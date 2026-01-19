//! Extrude feature handling

use tracing::info;

use rk_cad::{BooleanOp, CadKernel, Sketch, TessellatedMesh, Wire2D};

use crate::state::{AppState, ExtrudeDirection};

use super::super::ActionContext;

/// Generate preview mesh for multiple profiles by extruding each and combining with union.
fn generate_multi_profile_preview(
    kernel: &dyn CadKernel,
    sketch: &Sketch,
    profiles: &[Wire2D],
    distance: f32,
    direction: ExtrudeDirection,
) -> Result<TessellatedMesh, String> {
    if profiles.is_empty() {
        return Err("No profiles selected".to_string());
    }

    // Generate extrusion for the first profile
    let first_profile = &profiles[0];
    let mut combined_solid =
        generate_extrude_solid(kernel, sketch, first_profile, distance, direction)?;

    // Union with remaining profiles
    for profile in profiles.iter().skip(1) {
        let solid = generate_extrude_solid(kernel, sketch, profile, distance, direction)?;
        combined_solid = kernel
            .boolean(&combined_solid, &solid, rk_cad::BooleanType::Union)
            .map_err(|e| format!("Boolean union failed: {}", e))?;
    }

    // Tessellate for preview
    let mesh = kernel
        .tessellate(&combined_solid, 0.1)
        .map_err(|e| format!("Tessellation failed: {}", e))?;

    Ok(mesh)
}

/// Generate extrude solid for a single profile (without tessellation).
fn generate_extrude_solid(
    kernel: &dyn CadKernel,
    sketch: &Sketch,
    profile: &Wire2D,
    distance: f32,
    direction: ExtrudeDirection,
) -> Result<rk_cad::Solid, String> {
    if !kernel.is_available() {
        return Err("CAD kernel not available".to_string());
    }

    let extrude_dir = match direction {
        ExtrudeDirection::Positive => sketch.plane.normal,
        ExtrudeDirection::Negative => -sketch.plane.normal,
        ExtrudeDirection::Symmetric => sketch.plane.normal,
    };

    let extrude_dist = match direction {
        ExtrudeDirection::Symmetric => distance / 2.0,
        _ => distance,
    };

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

    // For symmetric, extrude in opposite direction and union
    if matches!(direction, ExtrudeDirection::Symmetric) {
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
            .map_err(|e| format!("Boolean union failed: {}", e))
    } else {
        Ok(solid)
    }
}

/// Regenerate preview mesh for the current extrude dialog state.
/// This should be called after profile selection or parameter changes.
pub fn regenerate_preview(ctx: &ActionContext) {
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

    // Get selected profiles
    let selected_profiles: Vec<Wire2D> = sketch_state
        .extrude_dialog
        .selected_profile_indices
        .iter()
        .filter_map(|&i| sketch_state.extrude_dialog.profiles.get(i).cloned())
        .collect();

    // Get the sketch plane info (need to drop state to avoid borrow issues)
    let sketch_info = state.cad.get_sketch(sketch_id).cloned();

    if let Some(sketch) = sketch_info {
        if selected_profiles.is_empty() {
            // No profiles selected
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.preview_mesh = None;
                sketch_state.extrude_dialog.error_message =
                    Some("No profiles selected".to_string());
            }
            return;
        }

        // Generate preview for multiple profiles
        match generate_multi_profile_preview(
            ctx.kernel.as_ref(),
            &sketch,
            &selected_profiles,
            distance,
            direction,
        ) {
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
        // Sketch not found
        if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
            sketch_state.extrude_dialog.preview_mesh = None;
            if sketch_state.extrude_dialog.profiles.is_empty() {
                sketch_state.extrude_dialog.error_message =
                    Some("No closed profiles found in sketch".to_string());
            }
        }
    }
}

/// Handle showing the extrude dialog
pub fn handle_show_extrude_dialog(ctx: &ActionContext) {
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

/// Handle updating extrude distance
pub fn handle_update_extrude_distance(ctx: &ActionContext, distance: f32) {
    {
        let mut state = ctx.app_state.lock();
        if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
            sketch_state.extrude_dialog.distance = distance;
        }
    }
    // Regenerate preview with new distance
    regenerate_preview(ctx);
}

/// Handle updating extrude direction
pub fn handle_update_extrude_direction(ctx: &ActionContext, direction: ExtrudeDirection) {
    {
        let mut state = ctx.app_state.lock();
        if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
            sketch_state.extrude_dialog.direction = direction;
        }
    }
    // Regenerate preview with new direction
    regenerate_preview(ctx);
}

/// Handle updating extrude boolean operation
pub fn handle_update_extrude_boolean_op(ctx: &ActionContext, boolean_op: BooleanOp) {
    {
        let mut state = ctx.app_state.lock();
        if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
            sketch_state.extrude_dialog.boolean_op = boolean_op;
            // Clear target_body if switching to New
            if boolean_op == BooleanOp::New {
                sketch_state.extrude_dialog.target_body = None;
            }
        }
    }
    // Regenerate preview with new boolean op
    regenerate_preview(ctx);
}

/// Handle updating extrude target body
pub fn handle_update_extrude_target_body(ctx: &ActionContext, target_body: Option<uuid::Uuid>) {
    {
        let mut state = ctx.app_state.lock();
        if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
            sketch_state.extrude_dialog.target_body = target_body;
        }
    }
    // Regenerate preview with new target body
    regenerate_preview(ctx);
}

/// Handle toggling extrude profile selection
pub fn handle_toggle_extrude_profile(ctx: &ActionContext, profile_index: usize) {
    {
        let mut state = ctx.app_state.lock();
        if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
            sketch_state.extrude_dialog.toggle_profile(profile_index);
            let is_selected = sketch_state
                .extrude_dialog
                .is_profile_selected(profile_index);
            info!(
                "Toggled profile index: {} (now {})",
                profile_index,
                if is_selected {
                    "selected"
                } else {
                    "deselected"
                }
            );
        }
    }
    // Regenerate preview with new profile selection
    regenerate_preview(ctx);
}

/// Handle executing the extrude operation
pub fn handle_execute_extrude(ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    // Get extrude parameters
    let (sketch_id, distance, direction, has_selected_profiles, boolean_op, target_body) = {
        let Some(sketch_state) = state.cad.editor_mode.sketch() else {
            return;
        };
        (
            sketch_state.extrude_dialog.sketch_id,
            sketch_state.extrude_dialog.distance,
            sketch_state.extrude_dialog.direction,
            !sketch_state
                .extrude_dialog
                .selected_profile_indices
                .is_empty(),
            sketch_state.extrude_dialog.boolean_op,
            sketch_state.extrude_dialog.target_body,
        )
    };

    // Solve the sketch first
    if let Some(sketch) = state.cad.get_sketch_mut(sketch_id) {
        sketch.solve();
    }

    // Check if any profiles are selected
    if !has_selected_profiles {
        tracing::warn!("No profiles selected for extrusion");
        return;
    }

    // Create and execute the extrude feature
    if state.cad.get_sketch(sketch_id).is_some() {
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
        let feature_id = feature.id();

        // Count bodies before adding the feature
        let bodies_before = state.cad.data.history.bodies().len();

        // Add the feature to history
        state.cad.data.history.add_feature(feature.clone());

        info!(
            "Execute extrude: sketch={}, distance={}, direction={:?}, boolean_op={:?}, target_body={:?}",
            sketch_id, distance, direction, boolean_op, target_body
        );

        // Rebuild the history to execute the feature
        if let Err(e) = state.cad.data.history.rebuild(ctx.kernel.as_ref()) {
            tracing::error!("Failed to rebuild after extrude: {}", e);
            // Show error in dialog and don't close
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.error_message = Some(format!("Extrude failed: {}", e));
            }
            // Remove the failed feature
            state.cad.data.history.remove_feature(feature_id);
            return;
        }

        // Check if the feature actually created a body
        let bodies_after = state.cad.data.history.bodies().len();
        if bodies_after <= bodies_before {
            // No new body was created - the feature execution failed silently
            let error_msg = if boolean_op == BooleanOp::Cut {
                "Cut operation is not supported by Truck kernel. Only 'New Body', 'Join', and 'Intersect' are available.".to_string()
            } else if boolean_op != BooleanOp::New {
                format!(
                    "Boolean operation '{}' failed. Please check the target body and try again.",
                    match boolean_op {
                        BooleanOp::Cut => "Cut",
                        BooleanOp::Join => "Join",
                        BooleanOp::Intersect => "Intersect",
                        BooleanOp::New => "New",
                    }
                )
            } else {
                "Extrude failed: No body was created. Check if the sketch has valid closed profiles.".to_string()
            };
            tracing::error!("{}", error_msg);
            // Show error in dialog and don't close
            if let Some(sketch_state) = state.cad.editor_mode.sketch_mut() {
                sketch_state.extrude_dialog.error_message = Some(error_msg);
            }
            // Remove the failed feature
            state.cad.data.history.remove_feature(feature_id);
            return;
        }

        // Log success
        info!("Rebuild complete. Total bodies: {}", bodies_after);

        // Sync CAD bodies to renderer
        sync_cad_bodies_to_renderer(ctx, &state);
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

/// Sync CAD bodies to the renderer
pub fn sync_cad_bodies_to_renderer(ctx: &ActionContext, state: &AppState) {
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
                        tracing::warn!("Failed to tessellate body {}: {}", body_id, e);
                    }
                }
            }
        }
    }
}
