//! Constraint handling for sketch actions

use std::collections::HashMap;

use glam::Vec2;
use tracing::info;

use rk_cad::{SketchConstraint, SketchEntity};

use crate::state::{ConstraintToolState, SketchTool};

use super::super::ActionContext;

/// Handle entity selection for constraint tools
pub fn handle_select_entity_for_constraint(ctx: &ActionContext, entity_id: uuid::Uuid) {
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
pub fn handle_confirm_dimension_constraint(ctx: &ActionContext) {
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
