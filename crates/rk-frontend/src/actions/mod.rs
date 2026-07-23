//! Action dispatch
//!
//! Panels queue `AppAction`s; once per frame they are dispatched here.
//! UI actions mutate `AppState` directly, engine actions go through
//! `Engine::apply` and return events that `crate::sync` turns into
//! renderer updates.

mod composite;
mod constraints;
mod ui;

use rk_engine::{Event, SharedEngine};

use crate::state::{AppAction, SharedAppState, SharedViewportState};

/// Context passed to action handlers
pub struct ActionContext<'a> {
    pub app_state: &'a SharedAppState,
    pub viewport_state: &'a Option<SharedViewportState>,
}

impl<'a> ActionContext<'a> {
    pub fn new(
        app_state: &'a SharedAppState,
        viewport_state: &'a Option<SharedViewportState>,
    ) -> Self {
        Self {
            app_state,
            viewport_state,
        }
    }

    /// Clone the engine handle out of the app state (so no app_state
    /// guard is held while the engine is locked)
    pub fn engine(&self) -> SharedEngine {
        self.app_state.lock().engine.clone()
    }

    /// Apply a command, collecting events; returns false on error
    pub fn apply(&self, cmd: rk_engine::Command, events: &mut Vec<Event>) -> bool {
        match self.engine().lock().apply(cmd) {
            Ok(evts) => {
                events.extend(evts);
                true
            }
            Err(e) => {
                tracing::error!("engine command failed: {e}");
                false
            }
        }
    }
}

/// Dispatch an action, accumulating engine events for the caller to sync
pub fn dispatch_action(action: AppAction, ctx: &ActionContext, events: &mut Vec<Event>) {
    match action {
        AppAction::SelectPart(part_id) => ui::handle_select_part(part_id, ctx),
        AppAction::SelectCollision(selection) => {
            ctx.app_state.lock().selected_collision = selection;
        }
        AppAction::SetEditingJoint(joint_id) => {
            ctx.app_state.lock().editing_joint_id = joint_id;
        }
        AppAction::SketchUi(action) => ui::handle_sketch_ui(action, ctx, events),

        AppAction::Cmd(cmd) => {
            ctx.apply(cmd, events);
        }
        AppAction::Interactive { session, cmd } => {
            match ctx.engine().lock().apply_interactive(session, cmd) {
                Ok(evts) => events.extend(evts),
                Err(e) => tracing::error!("interactive command failed: {e}"),
            }
        }
        AppAction::EndInteraction { session, cancel } => {
            match ctx.engine().lock().end_interaction(session, cancel) {
                Ok(evts) => events.extend(evts),
                Err(e) => tracing::error!("end_interaction failed: {e}"),
            }
        }

        AppAction::Composite(action) => composite::handle_composite(action, ctx, events),
    }
}
