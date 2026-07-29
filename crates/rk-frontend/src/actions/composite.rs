//! Composite actions: engine commands combined with UI state changes

use rk_engine::{Command, Event};
use tracing::info;
use uuid::Uuid;

use crate::state::{CompositeAction, EditorMode, ReferencePlane, SketchModeState};

use super::{ActionContext, constraints, ui};

pub fn handle_composite(action: CompositeAction, ctx: &ActionContext, events: &mut Vec<Event>) {
    match action {
        CompositeAction::SelectPlaneAndCreateSketch { plane } => {
            select_plane_and_create_sketch(plane, ctx, events)
        }
        CompositeAction::ExitSketchMode => exit_sketch_mode(ctx, events),
        CompositeAction::DeleteSelectedSketchEntities => {
            delete_selected_sketch_entities(ctx, events)
        }
        CompositeAction::ExecuteExtrude => execute_extrude(ctx, events),
        CompositeAction::ConfirmDimensionConstraint => {
            constraints::handle_confirm_dimension_constraint(ctx, events)
        }
        CompositeAction::DeleteSketch { sketch_id } => delete_sketch(sketch_id, ctx, events),
    }
}

fn select_plane_and_create_sketch(
    plane: ReferencePlane,
    ctx: &ActionContext,
    events: &mut Vec<Event>,
) {
    // 1. Move the camera to face the plane
    if let Some(viewport_state) = ctx.viewport_state {
        let mut vp = viewport_state.lock();
        match plane {
            ReferencePlane::XY => vp.renderer.camera_mut().set_top_view(),
            ReferencePlane::XZ => vp.renderer.camera_mut().set_front_view(),
            ReferencePlane::YZ => vp.renderer.camera_mut().set_side_view(),
        }
    }

    // 2. Create the sketch (ID minted here so we can enter its mode)
    let sketch_id = Uuid::new_v4();
    if !ctx.apply(
        Command::CreateSketch {
            id: Some(sketch_id),
            name: None,
            plane: plane.to_sketch_plane(),
        },
        events,
    ) {
        return;
    }
    info!("Created sketch on {} plane: {}", plane.name(), sketch_id);

    // 3. Enter sketch mode
    ctx.app_state.lock().editor_mode = EditorMode::Sketch(SketchModeState::new(sketch_id));
}

fn exit_sketch_mode(ctx: &ActionContext, events: &mut Vec<Event>) {
    let sketch_id = ctx
        .app_state
        .lock()
        .editor_mode
        .sketch()
        .map(|s| s.active_sketch);
    if let Some(sketch_id) = sketch_id {
        ctx.apply(Command::SolveSketch { sketch_id }, events);
    }
    ctx.app_state.lock().editor_mode = EditorMode::Assembly;
    info!("Exited sketch mode");
}

fn delete_selected_sketch_entities(ctx: &ActionContext, events: &mut Vec<Event>) {
    let Some((sketch_id, selected)) = ({
        let state = ctx.app_state.lock();
        state
            .editor_mode
            .sketch()
            .map(|s| (s.active_sketch, s.selected_entities.clone()))
    }) else {
        return;
    };
    if selected.is_empty() {
        return;
    }

    if ctx.apply(
        Command::DeleteSketchEntities {
            sketch_id,
            entity_ids: selected,
        },
        events,
    ) {
        ui::with_sketch_state(ctx, |s| s.clear_selection());
    }
}

fn execute_extrude(ctx: &ActionContext, events: &mut Vec<Event>) {
    let Some((sketch_id, distance, direction, picked, boolean_op, target_body)) = ({
        let state = ctx.app_state.lock();
        state.editor_mode.sketch().map(|s| {
            (
                s.extrude_dialog.sketch_id,
                s.extrude_dialog.distance,
                s.extrude_dialog.direction,
                s.extrude_dialog.selected_profile_indices.clone(),
                s.extrude_dialog.boolean_op,
                s.extrude_dialog.target_body,
            )
        })
    }) else {
        return;
    };

    if picked.is_empty() {
        tracing::warn!("No profiles selected for extrusion");
        return;
    }

    ctx.apply(Command::SolveSketch { sketch_id }, events);

    // The dialog picks regions by their position in the list; the command
    // names them by ID, which is what survives a rebuild
    let engine = ctx.engine();
    let profiles: Vec<uuid::Uuid> = {
        let engine = engine.lock();
        let Some(sketch) = engine.sketch(sketch_id) else {
            return;
        };
        let regions = sketch.profiles();
        picked
            .iter()
            .filter_map(|i| regions.get(*i).map(|p| p.id))
            .collect()
    };

    let result = ctx.engine().lock().apply(Command::AddExtrude {
        id: None,
        name: None,
        sketch_id,
        profiles,
        distance,
        direction: direction.to_cad(),
        boolean_op,
        target_body,
    });

    match result {
        Ok(evts) => {
            events.extend(evts);
            // Close the dialog and return to assembly mode
            ui::with_sketch_state(ctx, |s| s.extrude_dialog.close());
            ctx.app_state.lock().editor_mode = EditorMode::Assembly;
            info!("Extrude complete, exited sketch mode");
        }
        Err(e) => {
            tracing::error!("Extrude failed: {e}");
            ui::with_sketch_state(ctx, |s| {
                s.extrude_dialog.error_message = Some(e.to_string());
            });
        }
    }
}

fn delete_sketch(sketch_id: Uuid, ctx: &ActionContext, events: &mut Vec<Event>) {
    // Leave sketch mode first if this sketch is being edited
    {
        let mut state = ctx.app_state.lock();
        if state
            .editor_mode
            .sketch()
            .is_some_and(|s| s.active_sketch == sketch_id)
        {
            state.editor_mode = EditorMode::Assembly;
            info!("Exited sketch mode before deleting sketch");
        }
    }
    ctx.apply(Command::DeleteSketch { sketch_id }, events);
}
