//! Action handling module
//!
//! This module contains the action dispatch system for the URDF editor.
//! Actions are queued in AppState and processed each frame.

mod assembly;
#[cfg(not(target_arch = "wasm32"))]
mod file;
#[cfg(target_arch = "wasm32")]
mod file_wasm;
mod part;
mod sketch;

use std::sync::Arc;

use rk_cad::CadKernel;

use crate::state::{AppAction, SharedAppState, SharedViewportState};

pub use assembly::handle_assembly_action;
#[cfg(not(target_arch = "wasm32"))]
pub use file::handle_file_action;
#[cfg(target_arch = "wasm32")]
pub use file_wasm::handle_file_action_wasm;
pub use part::handle_part_action;
pub use sketch::handle_sketch_action;

/// Shared CAD kernel for geometry operations
pub type SharedKernel = Arc<dyn CadKernel>;

/// Context for action handlers
pub struct ActionContext<'a> {
    pub app_state: &'a SharedAppState,
    pub viewport_state: &'a Option<SharedViewportState>,
    pub kernel: &'a SharedKernel,
}

impl<'a> ActionContext<'a> {
    pub fn new(
        app_state: &'a SharedAppState,
        viewport_state: &'a Option<SharedViewportState>,
        kernel: &'a SharedKernel,
    ) -> Self {
        Self {
            app_state,
            viewport_state,
            kernel,
        }
    }
}

/// Dispatch an action to the appropriate handler
pub fn dispatch_action(action: AppAction, ctx: &ActionContext) {
    match action {
        // File actions (native only)
        #[cfg(not(target_arch = "wasm32"))]
        AppAction::ImportMesh(_)
        | AppAction::ImportUrdf(_)
        | AppAction::SaveProject(_)
        | AppAction::LoadProject(_)
        | AppAction::ExportUrdf { .. }
        | AppAction::NewProject => {
            handle_file_action(action, ctx);
        }

        // File actions (WASM - ignore)
        #[cfg(target_arch = "wasm32")]
        AppAction::ImportMesh(_)
        | AppAction::ImportUrdf(_)
        | AppAction::SaveProject(_)
        | AppAction::LoadProject(_)
        | AppAction::ExportUrdf { .. } => {
            tracing::warn!("File actions are not supported in WASM");
        }

        #[cfg(target_arch = "wasm32")]
        AppAction::NewProject => {
            ctx.app_state.lock().new_project();
            if let Some(viewport_state) = ctx.viewport_state {
                viewport_state.lock().clear_parts();
                viewport_state.lock().clear_overlays();
            }
        }

        // Bytes-based file actions (for WASM, but works on native too)
        #[cfg(target_arch = "wasm32")]
        AppAction::ImportMeshBytes { .. } | AppAction::LoadProjectBytes { .. } => {
            handle_file_action_wasm(action, ctx);
        }

        #[cfg(not(target_arch = "wasm32"))]
        AppAction::ImportMeshBytes { .. } | AppAction::LoadProjectBytes { .. } => {
            // On native, bytes-based actions can still be used (e.g., for testing)
            // but we don't have the handler compiled, so just log
            tracing::warn!("Bytes-based file actions are primarily for WASM");
        }

        // Part actions
        AppAction::CreatePrimitive { .. }
        | AppAction::CreateEmpty { .. }
        | AppAction::SelectPart(_)
        | AppAction::DeleteSelectedPart
        | AppAction::UpdatePartTransform { .. } => {
            handle_part_action(action, ctx);
        }

        // Assembly actions
        AppAction::ConnectParts { .. }
        | AppAction::DisconnectPart { .. }
        | AppAction::UpdateJointPosition { .. }
        | AppAction::ResetJointPosition { .. }
        | AppAction::ResetAllJointPositions
        | AppAction::UpdateJointType { .. }
        | AppAction::UpdateJointOrigin { .. }
        | AppAction::UpdateJointAxis { .. }
        | AppAction::UpdateJointLimits { .. } => {
            handle_assembly_action(action, ctx);
        }

        // Collision actions
        AppAction::SelectCollision(_)
        | AppAction::AddCollision { .. }
        | AppAction::RemoveCollision { .. }
        | AppAction::UpdateCollisionOrigin { .. }
        | AppAction::UpdateCollisionGeometry { .. } => {
            handle_assembly_action(action, ctx);
        }

        // Sketch/CAD actions
        AppAction::SketchAction(_) => {
            handle_sketch_action(action, ctx);
        }
    }
}
