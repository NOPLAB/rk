//! UI-only action handlers: selection, sketch mode/tool state, and the
//! extrude dialog (whose preview is an engine query, not a command).

use rk_engine::{Command, Event, ExtrudePreviewRequest};
use tracing::info;
use uuid::Uuid;

use crate::state::{EditorMode, PlaneSelectionState, SketchModeState, SketchUiAction};

use super::{ActionContext, constraints};

pub fn handle_select_part(part_id: Option<Uuid>, ctx: &ActionContext) {
    ctx.app_state.lock().select_part(part_id);
    // Update mesh highlighting; overlays follow in update_overlays()
    if let Some(viewport_state) = ctx.viewport_state {
        viewport_state.lock().set_selected_part(part_id);
    }
}

pub fn handle_sketch_ui(action: SketchUiAction, ctx: &ActionContext, events: &mut Vec<Event>) {
    match action {
        SketchUiAction::BeginPlaneSelection => {
            ctx.app_state.lock().editor_mode =
                EditorMode::PlaneSelection(PlaneSelectionState::default());
            info!("Entered plane selection mode");
        }

        SketchUiAction::CancelPlaneSelection => {
            ctx.app_state.lock().editor_mode = EditorMode::Assembly;
            info!("Cancelled plane selection");
        }

        SketchUiAction::SetHoveredPlane { plane } => {
            let mut state = ctx.app_state.lock();
            if let Some(plane_state) = state.editor_mode.plane_selection_mut() {
                plane_state.hovered_plane = plane;
            }
        }

        SketchUiAction::EditSketch { sketch_id } => {
            let exists = ctx.engine().lock().sketch(sketch_id).is_some();
            if exists {
                ctx.app_state.lock().editor_mode =
                    EditorMode::Sketch(SketchModeState::new(sketch_id));
                info!("Entered sketch mode for: {}", sketch_id);
            } else {
                tracing::warn!("Sketch not found: {}", sketch_id);
            }
        }

        SketchUiAction::SetTool { tool } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.editor_mode.sketch_mut() {
                sketch_state.current_tool = tool;
                sketch_state.cancel_drawing();
            }
        }

        SketchUiAction::ToggleSnap => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.editor_mode.sketch_mut() {
                sketch_state.snap_to_grid = !sketch_state.snap_to_grid;
            }
        }

        SketchUiAction::SetGridSpacing { spacing } => {
            let mut state = ctx.app_state.lock();
            if let Some(sketch_state) = state.editor_mode.sketch_mut() {
                sketch_state.grid_spacing = spacing;
            }
        }

        // ---- Extrude dialog ----
        SketchUiAction::ShowExtrudeDialog => handle_show_extrude_dialog(ctx, events),

        SketchUiAction::UpdateExtrudeDistance { distance } => {
            with_sketch_state(ctx, |s| s.extrude_dialog.distance = distance);
            regenerate_preview(ctx);
        }

        SketchUiAction::UpdateExtrudeDirection { direction } => {
            with_sketch_state(ctx, |s| s.extrude_dialog.direction = direction);
            regenerate_preview(ctx);
        }

        SketchUiAction::UpdateExtrudeBooleanOp { boolean_op } => {
            with_sketch_state(ctx, |s| {
                s.extrude_dialog.boolean_op = boolean_op;
                if boolean_op == rk_cad::BooleanOp::New {
                    s.extrude_dialog.target_body = None;
                }
            });
            regenerate_preview(ctx);
        }

        SketchUiAction::UpdateExtrudeTargetBody { target_body } => {
            with_sketch_state(ctx, |s| s.extrude_dialog.target_body = target_body);
            regenerate_preview(ctx);
        }

        SketchUiAction::ToggleExtrudeProfile { profile_index } => {
            with_sketch_state(ctx, |s| s.extrude_dialog.toggle_profile(profile_index));
            regenerate_preview(ctx);
        }

        SketchUiAction::CancelExtrudeDialog => {
            with_sketch_state(ctx, |s| s.extrude_dialog.close());
            info!("Cancelled extrude dialog");
        }

        // ---- Constraint workflow ----
        SketchUiAction::SelectEntityForConstraint { entity_id } => {
            constraints::handle_select_entity_for_constraint(ctx, entity_id, events);
        }

        SketchUiAction::CancelConstraintSelection => {
            with_sketch_state(ctx, |s| {
                s.constraint_tool_state = None;
                s.clear_selection();
            });
        }

        SketchUiAction::ShowDimensionDialog {
            tool,
            entities,
            initial_value,
        } => {
            with_sketch_state(ctx, |s| {
                s.dimension_dialog
                    .open_for_constraint(tool, entities, initial_value)
            });
        }

        SketchUiAction::UpdateDimensionValue { value } => {
            with_sketch_state(ctx, |s| {
                s.dimension_dialog.value = value;
                s.dimension_dialog.value_text = format!("{:.2}", value);
            });
        }

        SketchUiAction::CancelDimensionDialog => {
            with_sketch_state(ctx, |s| {
                s.dimension_dialog.close();
                s.clear_selection();
            });
        }
    }
}

/// Run a closure on the sketch mode state, if in sketch mode
pub(super) fn with_sketch_state(ctx: &ActionContext, f: impl FnOnce(&mut SketchModeState)) {
    let mut state = ctx.app_state.lock();
    if let Some(sketch_state) = state.editor_mode.sketch_mut() {
        f(sketch_state);
    }
}

fn handle_show_extrude_dialog(ctx: &ActionContext, events: &mut Vec<Event>) {
    let Some(sketch_id) = ctx
        .app_state
        .lock()
        .editor_mode
        .sketch()
        .map(|s| s.active_sketch)
    else {
        return;
    };

    // Solve so profile extraction sees up-to-date geometry
    ctx.apply(Command::SolveSketch { sketch_id }, events);

    let (profiles, available_bodies) = {
        let engine = ctx.engine();
        let eng = engine.lock();
        let profiles = match eng.sketch(sketch_id).map(|s| s.extract_profiles()) {
            Some(Ok(profiles)) => profiles,
            Some(Err(e)) => {
                tracing::warn!("Failed to extract profiles: {}", e);
                Vec::new()
            }
            None => Vec::new(),
        };
        let bodies: Vec<(Uuid, String)> = eng
            .document()
            .cad
            .history
            .bodies()
            .iter()
            .map(|(id, body)| (*id, body.name.clone()))
            .collect();
        (profiles, bodies)
    };

    with_sketch_state(ctx, |s| {
        s.extrude_dialog.open_for_sketch(sketch_id);
        s.extrude_dialog.set_profiles(profiles);
        s.extrude_dialog.available_bodies = available_bodies;
        info!(
            "Opened extrude dialog for sketch: {} with {} profiles, {} available bodies",
            sketch_id,
            s.extrude_dialog.profiles.len(),
            s.extrude_dialog.available_bodies.len()
        );
    });

    regenerate_preview(ctx);
}

/// Regenerate the extrude preview mesh via the engine's preview query
pub(super) fn regenerate_preview(ctx: &ActionContext) {
    let request = {
        let state = ctx.app_state.lock();
        let Some(sketch_state) = state.editor_mode.sketch() else {
            return;
        };
        if !sketch_state.extrude_dialog.open {
            return;
        }
        let dialog = &sketch_state.extrude_dialog;
        let profiles: Vec<_> = dialog
            .selected_profile_indices
            .iter()
            .filter_map(|&i| dialog.profiles.get(i).cloned())
            .collect();
        ExtrudePreviewRequest {
            sketch_id: dialog.sketch_id,
            profiles,
            distance: dialog.distance,
            direction: dialog.direction.to_cad(),
        }
    };

    let result = ctx.engine().lock().preview_extrude(&request);
    with_sketch_state(ctx, |s| match &result {
        Ok(mesh) => {
            s.extrude_dialog.preview_mesh = Some(mesh.clone());
            s.extrude_dialog.error_message = None;
        }
        Err(e) => {
            s.extrude_dialog.preview_mesh = None;
            s.extrude_dialog.error_message = Some(e.to_string());
        }
    });
}
