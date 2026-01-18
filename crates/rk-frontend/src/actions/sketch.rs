//! Sketch action handling
//!
//! Handles actions related to sketch editing and CAD operations.

use tracing::info;

use crate::state::{AppAction, SketchAction};

use super::ActionContext;

/// Handle sketch-related actions
pub fn handle_sketch_action(action: AppAction, ctx: &ActionContext) {
    let sketch_action = match action {
        AppAction::SketchAction(sa) => sa,
        _ => return,
    };

    match sketch_action {
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
    }
}
