//! Assembly-related action handlers

use uuid::Uuid;

use urdf_core::{Joint, JointPoint, Link, Pose};

use crate::state::{AppAction, AppState};

use super::ActionContext;

/// Handle assembly-related actions
pub fn handle_assembly_action(action: AppAction, ctx: &ActionContext) {
    match action {
        AppAction::ConnectParts { parent, child } => handle_connect_parts(parent, child, ctx),
        AppAction::DisconnectPart { child } => handle_disconnect_part(child, ctx),
        AppAction::AddJointPoint { part_id, position } => {
            handle_add_joint_point(part_id, position, ctx)
        }
        AppAction::RemoveJointPoint { part_id, point_id } => {
            handle_remove_joint_point(part_id, point_id, ctx)
        }
        _ => {}
    }
}

fn handle_connect_parts(parent: Uuid, child: Uuid, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    // Get or create links
    let parent_link_id = find_or_create_link(&mut state, parent);
    let child_link_id = find_or_create_link(&mut state, child);

    if let (Some(parent_link_id), Some(child_link_id)) = (parent_link_id, child_link_id) {
        // Disconnect child from existing parent first
        if state.project.assembly.parent.contains_key(&child_link_id) {
            if let Err(e) = state.project.assembly.disconnect(child_link_id) {
                tracing::warn!("Failed to disconnect existing parent: {}", e);
            }
        }

        // Get names first to avoid borrow issues
        let child_name_for_jp = state
            .parts
            .get(&child)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "child".to_string());
        let parent_name_for_jp = state
            .parts
            .get(&parent)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "parent".to_string());

        // Create joint point on parent part (at center)
        let parent_jp_id = if let Some(part) = state.parts.get_mut(&parent) {
            let center = glam::Vec3::new(
                (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
                (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
                (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
            );
            let jp = JointPoint::new(format!("joint_to_{}", child_name_for_jp), center);
            let jp_id = jp.id;
            part.joint_points.push(jp);
            Some(jp_id)
        } else {
            None
        };

        // Create joint point on child part (at center)
        let child_jp_id = if let Some(part) = state.parts.get_mut(&child) {
            let center = glam::Vec3::new(
                (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
                (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
                (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
            );
            let jp = JointPoint::new(format!("joint_from_{}", parent_name_for_jp), center);
            let jp_id = jp.id;
            part.joint_points.push(jp);
            Some(jp_id)
        } else {
            None
        };

        // Get names for joint
        let parent_name = state
            .project
            .assembly
            .links
            .get(&parent_link_id)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let child_name = state
            .project
            .assembly
            .links
            .get(&child_link_id)
            .map(|l| l.name.clone())
            .unwrap_or_default();

        // Create fixed joint
        let mut joint = Joint::fixed(
            format!("{}_to_{}", parent_name, child_name),
            parent_link_id,
            child_link_id,
            Pose::default(),
        );
        joint.parent_joint_point = parent_jp_id;
        joint.child_joint_point = child_jp_id;

        match state
            .project
            .assembly
            .connect(parent_link_id, child_link_id, joint)
        {
            Ok(joint_id) => {
                tracing::info!(
                    "Connected {} to {} via joint {}",
                    parent_name,
                    child_name,
                    joint_id
                );
                state.modified = true;
            }
            Err(e) => {
                tracing::error!("Failed to connect parts: {}", e);
            }
        }
    }
}

fn handle_disconnect_part(child: Uuid, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    // Find the link for this part
    let child_link_id = state
        .project
        .assembly
        .links
        .iter()
        .find(|(_, l)| l.part_id == Some(child))
        .map(|(id, _)| *id);

    if let Some(link_id) = child_link_id {
        match state.project.assembly.disconnect(link_id) {
            Ok(joint) => {
                tracing::info!("Disconnected part {}, removed joint {}", child, joint.name);
                state.modified = true;
            }
            Err(e) => {
                tracing::error!("Failed to disconnect part: {}", e);
            }
        }
    }
}

fn handle_add_joint_point(part_id: Uuid, position: glam::Vec3, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    if let Some(part) = state.parts.get_mut(&part_id) {
        let jp = JointPoint::new(format!("joint_point_{}", part.joint_points.len()), position);
        part.joint_points.push(jp);
        state.modified = true;
        tracing::info!("Added joint point to part {}", part_id);
    }
}

fn handle_remove_joint_point(part_id: Uuid, point_id: Uuid, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    if let Some(part) = state.parts.get_mut(&part_id) {
        if let Some(idx) = part.joint_points.iter().position(|jp| jp.id == point_id) {
            part.joint_points.remove(idx);
            state.modified = true;
            tracing::info!("Removed joint point {} from part {}", point_id, part_id);
        }
    }
}

/// Helper to find or create a link for a part
fn find_or_create_link(state: &mut AppState, part_id: Uuid) -> Option<Uuid> {
    // Check if link already exists for this part
    if let Some((link_id, _)) = state
        .project
        .assembly
        .links
        .iter()
        .find(|(_, l)| l.part_id == Some(part_id))
    {
        return Some(*link_id);
    }
    // Create new link
    if let Some(part) = state.parts.get(&part_id) {
        let link = Link::from_part(part);
        let link_id = state.project.assembly.add_link(link);
        Some(link_id)
    } else {
        None
    }
}
