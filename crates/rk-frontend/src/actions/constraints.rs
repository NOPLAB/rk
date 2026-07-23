//! Constraint tool workflow: entity picks either create a constraint
//! command immediately or open the dimension dialog for a value.

use std::collections::HashMap;

use glam::Vec2;
use rk_cad::{Sketch, SketchConstraint, SketchEntity};
use rk_engine::{Command, Event};
use tracing::info;
use uuid::Uuid;

use crate::state::{ConstraintToolState, SketchTool};

use super::{ActionContext, ui};

/// Handle entity selection for constraint tools
pub fn handle_select_entity_for_constraint(
    ctx: &ActionContext,
    entity_id: Uuid,
    events: &mut Vec<Event>,
) {
    let Some((tool, sketch_id, first_entity)) = ({
        let state = ctx.app_state.lock();
        state.editor_mode.sketch().map(|s| {
            let first = match &s.constraint_tool_state {
                Some(ConstraintToolState::WaitingForSecond { first_entity }) => {
                    Some(*first_entity)
                }
                _ => None,
            };
            (s.current_tool, s.active_sketch, first)
        })
    }) else {
        return;
    };

    // Snapshot the sketch for geometry inspection (initial values, entity kinds)
    let sketch = ctx.engine().lock().sketch(sketch_id).cloned();
    let Some(sketch) = sketch else {
        return;
    };

    if let Some(first) = first_entity {
        // Second pick
        if first == entity_id {
            return; // Can't constrain to self
        }

        if tool.is_dimension() {
            let initial_value = compute_initial_value(tool, &[first, entity_id], &sketch);
            ui::with_sketch_state(ctx, |s| {
                s.dimension_dialog
                    .open_for_constraint(tool, vec![first, entity_id], initial_value);
                s.constraint_tool_state = None;
                s.clear_selection();
            });
        } else if let Some(constraint) = create_two_entity_constraint(tool, first, entity_id) {
            ui::with_sketch_state(ctx, |s| {
                s.constraint_tool_state = None;
                s.clear_selection();
            });
            add_constraint_and_solve(ctx, sketch_id, constraint, events);
        }
    } else {
        // First pick

        // DimensionDistance on a line = length constraint, single pick
        if tool == SketchTool::DimensionDistance
            && matches!(sketch.get_entity(entity_id), Some(SketchEntity::Line { .. }))
        {
            let initial_value = compute_initial_value(tool, &[entity_id], &sketch);
            ui::with_sketch_state(ctx, |s| {
                s.dimension_dialog
                    .open_for_constraint(tool, vec![entity_id], initial_value);
                s.constraint_tool_state = None;
            });
            return;
        }

        if is_single_entity_constraint(tool) {
            if let Some(constraint) = create_single_entity_constraint(tool, entity_id, &sketch) {
                ui::with_sketch_state(ctx, |s| {
                    s.constraint_tool_state = None;
                    s.clear_selection();
                });
                add_constraint_and_solve(ctx, sketch_id, constraint, events);
            }
        } else if matches!(tool, SketchTool::DimensionRadius) {
            let initial_value = compute_initial_value(tool, &[entity_id], &sketch);
            ui::with_sketch_state(ctx, |s| {
                s.dimension_dialog
                    .open_for_constraint(tool, vec![entity_id], initial_value);
                s.constraint_tool_state = None;
            });
        } else {
            // Wait for the second pick
            ui::with_sketch_state(ctx, |s| {
                s.constraint_tool_state = Some(ConstraintToolState::WaitingForSecond {
                    first_entity: entity_id,
                });
                s.select_entity(entity_id);
            });
        }
    }
}

/// Confirm the dimension dialog: build the constraint and apply it
pub fn handle_confirm_dimension_constraint(ctx: &ActionContext, events: &mut Vec<Event>) {
    let Some((tool, entities, value, sketch_id)) = ({
        let mut state = ctx.app_state.lock();
        state.editor_mode.sketch_mut().map(|s| {
            let data = (
                s.dimension_dialog.tool,
                s.dimension_dialog.entities.clone(),
                s.dimension_dialog.value,
                s.active_sketch,
            );
            s.dimension_dialog.close();
            s.clear_selection();
            data
        })
    }) else {
        return;
    };

    if let Some(tool) = tool
        && let Some(constraint) = create_dimensional_constraint(tool, &entities, value)
    {
        add_constraint_and_solve(ctx, sketch_id, constraint, events);
    }
}

fn add_constraint_and_solve(
    ctx: &ActionContext,
    sketch_id: Uuid,
    constraint: SketchConstraint,
    events: &mut Vec<Event>,
) {
    if ctx.apply(
        Command::AddSketchConstraint {
            sketch_id,
            constraint,
        },
        events,
    ) {
        ctx.apply(Command::SolveSketch { sketch_id }, events);
        info!("Added constraint and solved sketch");
    }
}

fn is_single_entity_constraint(tool: SketchTool) -> bool {
    matches!(
        tool,
        SketchTool::ConstrainHorizontal
            | SketchTool::ConstrainVertical
            | SketchTool::ConstrainFixed
    )
}

fn create_single_entity_constraint(
    tool: SketchTool,
    entity_id: Uuid,
    sketch: &Sketch,
) -> Option<SketchConstraint> {
    match tool {
        SketchTool::ConstrainHorizontal => Some(SketchConstraint::horizontal(entity_id)),
        SketchTool::ConstrainVertical => Some(SketchConstraint::vertical(entity_id)),
        SketchTool::ConstrainFixed => {
            if let Some(SketchEntity::Point { position, .. }) = sketch.get_entity(entity_id) {
                Some(SketchConstraint::fixed(entity_id, position.x, position.y))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn create_two_entity_constraint(
    tool: SketchTool,
    entity1: Uuid,
    entity2: Uuid,
) -> Option<SketchConstraint> {
    match tool {
        SketchTool::ConstrainCoincident => Some(SketchConstraint::coincident(entity1, entity2)),
        SketchTool::ConstrainParallel => Some(SketchConstraint::parallel(entity1, entity2)),
        SketchTool::ConstrainPerpendicular => {
            Some(SketchConstraint::perpendicular(entity1, entity2))
        }
        SketchTool::ConstrainTangent => Some(SketchConstraint::tangent(entity1, entity2)),
        SketchTool::ConstrainEqual => Some(SketchConstraint::equal_length(entity1, entity2)),
        _ => None,
    }
}

fn create_dimensional_constraint(
    tool: SketchTool,
    entities: &[Uuid],
    value: f32,
) -> Option<SketchConstraint> {
    match tool {
        SketchTool::DimensionDistance => {
            if entities.len() == 1 {
                Some(SketchConstraint::length(entities[0], value))
            } else if entities.len() >= 2 {
                Some(SketchConstraint::distance(entities[0], entities[1], value))
            } else {
                None
            }
        }
        SketchTool::DimensionHorizontal => (entities.len() >= 2).then(|| {
            SketchConstraint::horizontal_distance(entities[0], entities[1], value)
        }),
        SketchTool::DimensionVertical => (entities.len() >= 2)
            .then(|| SketchConstraint::vertical_distance(entities[0], entities[1], value)),
        SketchTool::DimensionAngle => (entities.len() >= 2)
            .then(|| SketchConstraint::angle(entities[0], entities[1], value.to_radians())),
        SketchTool::DimensionRadius => (!entities.is_empty())
            .then(|| SketchConstraint::radius(entities[0], value)),
        _ => None,
    }
}

/// Compute initial value for dimensional constraints from current geometry
fn compute_initial_value(tool: SketchTool, entities: &[Uuid], sketch: &Sketch) -> f32 {
    let point_positions: HashMap<Uuid, Vec2> = sketch
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
            if entities.len() == 1 {
                if let Some(SketchEntity::Line { start, end, .. }) = sketch.get_entity(entities[0])
                    && let (Some(&p1), Some(&p2)) =
                        (point_positions.get(start), point_positions.get(end))
                {
                    return (p2 - p1).length();
                }
            } else if entities.len() >= 2
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
        SketchTool::DimensionAngle => 90.0,
        _ => 10.0,
    }
}
