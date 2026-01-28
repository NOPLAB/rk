//! Undo/Redo action handling

use crate::state::AppAction;

use super::ActionContext;

impl AppAction {
    /// Check if this action should be included in undo history
    pub fn is_undoable(&self) -> bool {
        match self {
            // Selection operations are not undoable (UX preference)
            AppAction::SelectPart(_)
            | AppAction::SelectCollision(_)
            | AppAction::SetEditingJoint(_) => false,

            // File operations that reset state are not undoable
            AppAction::NewProject
            | AppAction::LoadProject(_)
            | AppAction::LoadProjectBytes { .. } => false,

            // Real-time operations that fire frequently are not undoable
            AppAction::UpdateJointPosition { .. } => false,

            // Undo/Redo themselves are not undoable (prevent infinite loop)
            AppAction::Undo | AppAction::Redo => false,

            // All other actions are undoable
            _ => true,
        }
    }

    /// Get a description of the action for display
    pub fn description(&self) -> &'static str {
        match self {
            // File actions
            AppAction::ImportMesh(_) | AppAction::ImportMeshBytes { .. } => "Import Mesh",
            AppAction::ImportUrdf(_) => "Import URDF",
            AppAction::SaveProject(_) => "Save Project",
            AppAction::LoadProject(_) | AppAction::LoadProjectBytes { .. } => "Load Project",
            AppAction::ExportUrdf { .. } => "Export URDF",
            AppAction::NewProject => "New Project",

            // Part actions
            AppAction::CreatePrimitive { .. } => "Create Primitive",
            AppAction::CreateEmpty { .. } => "Create Empty Part",
            AppAction::SelectPart(_) => "Select Part",
            AppAction::DeleteSelectedPart => "Delete Part",
            AppAction::UpdatePartTransform { .. } => "Move Part",

            // Assembly actions
            AppAction::ConnectParts { .. } => "Connect Parts",
            AppAction::DisconnectPart { .. } => "Disconnect Part",
            AppAction::UpdateJointPosition { .. } => "Update Joint Position",
            AppAction::ResetJointPosition { .. } => "Reset Joint Position",
            AppAction::ResetAllJointPositions => "Reset All Joint Positions",
            AppAction::UpdateJointType { .. } => "Change Joint Type",
            AppAction::UpdateJointOrigin { .. } => "Update Joint Origin",
            AppAction::UpdateJointAxis { .. } => "Update Joint Axis",
            AppAction::UpdateJointLimits { .. } => "Update Joint Limits",
            AppAction::SetEditingJoint(_) => "Edit Joint",

            // Collision actions
            AppAction::SelectCollision(_) => "Select Collision",
            AppAction::AddCollision { .. } => "Add Collision",
            AppAction::RemoveCollision { .. } => "Remove Collision",
            AppAction::UpdateCollisionOrigin { .. } => "Update Collision Origin",
            AppAction::UpdateCollisionGeometry { .. } => "Update Collision Geometry",

            // Sketch actions
            AppAction::SketchAction(_) => "Sketch Action",

            // History actions
            AppAction::Undo => "Undo",
            AppAction::Redo => "Redo",
        }
    }
}

/// Handle the Undo action
pub fn handle_undo(ctx: &ActionContext) {
    let snapshot = {
        let mut state = ctx.app_state.lock();
        // Clone current state to pass to undo
        let current_project = state.project.clone();
        let current_cad = state.cad.clone();
        state.history.undo(&current_project, &current_cad)
    };

    if let Some(snapshot) = snapshot {
        {
            let mut state = ctx.app_state.lock();

            // Restore project and CAD state
            state.project = snapshot.project;
            state.cad = snapshot.cad;
            state.modified = true;

            // Update world transforms
            state
                .project
                .assembly
                .update_world_transforms_with_current_positions();

            // Clear selections (they may reference deleted items)
            state.selected_part = None;
            state.selected_collision = None;
            state.editing_joint_id = None;

            tracing::debug!("Undo: {}", snapshot.description);
        }

        // Sync viewport with restored state
        sync_viewport_with_project(ctx);
    }
}

/// Handle the Redo action
pub fn handle_redo(ctx: &ActionContext) {
    let snapshot = {
        let mut state = ctx.app_state.lock();
        // Clone current state to pass to redo
        let current_project = state.project.clone();
        let current_cad = state.cad.clone();
        state.history.redo(&current_project, &current_cad)
    };

    if let Some(snapshot) = snapshot {
        {
            let mut state = ctx.app_state.lock();

            // Restore project and CAD state
            state.project = snapshot.project;
            state.cad = snapshot.cad;
            state.modified = true;

            // Update world transforms
            state
                .project
                .assembly
                .update_world_transforms_with_current_positions();

            // Clear selections (they may reference deleted items)
            state.selected_part = None;
            state.selected_collision = None;
            state.editing_joint_id = None;

            tracing::debug!("Redo: {}", snapshot.description);
        }

        // Sync viewport with restored state
        sync_viewport_with_project(ctx);
    }
}

/// Synchronize the viewport state with the restored project
fn sync_viewport_with_project(ctx: &ActionContext) {
    if let Some(viewport_state) = ctx.viewport_state {
        let state = ctx.app_state.lock();

        // Clear and rebuild viewport parts
        let mut vp = viewport_state.lock();
        vp.clear_parts();
        vp.clear_overlays();

        // Re-add all parts from the project
        for part in state.project.parts_iter() {
            vp.add_part(part);
        }

        // Update transforms from assembly links
        for (link_id, link) in state.project.assembly.links.iter() {
            vp.update_part_transform(*link_id, link.world_transform);
        }
    }
}
