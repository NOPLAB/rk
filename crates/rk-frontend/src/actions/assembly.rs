//! Assembly-related action handlers

use uuid::Uuid;

use glam::Vec3;
use rk_core::{CollisionElement, GeometryType, Joint, JointLimits, JointType, Link, Pose};

use crate::state::{AppAction, AppState};

use super::ActionContext;

/// Handle assembly-related actions
pub fn handle_assembly_action(action: AppAction, ctx: &ActionContext) {
    match action {
        AppAction::ConnectParts { parent, child } => handle_connect_parts(parent, child, ctx),
        AppAction::DisconnectPart { child } => handle_disconnect_part(child, ctx),
        AppAction::UpdateJointPosition { joint_id, position } => {
            handle_update_joint_position(joint_id, position, ctx)
        }
        AppAction::ResetJointPosition { joint_id } => handle_reset_joint_position(joint_id, ctx),
        AppAction::ResetAllJointPositions => handle_reset_all_joint_positions(ctx),
        AppAction::SetEditingJoint(joint_id) => handle_set_editing_joint(joint_id, ctx),
        AppAction::SelectCollision(selection) => handle_select_collision(selection, ctx),
        AppAction::AddCollision { link_id, geometry } => {
            handle_add_collision(link_id, geometry, ctx)
        }
        AppAction::RemoveCollision { link_id, index } => {
            handle_remove_collision(link_id, index, ctx)
        }
        AppAction::UpdateCollisionOrigin {
            link_id,
            index,
            origin,
        } => handle_update_collision_origin(link_id, index, origin, ctx),
        AppAction::UpdateCollisionGeometry {
            link_id,
            index,
            geometry,
        } => handle_update_collision_geometry(link_id, index, geometry, ctx),
        // Joint configuration actions
        AppAction::UpdateJointType {
            joint_id,
            joint_type,
        } => handle_update_joint_type(joint_id, joint_type, ctx),
        AppAction::UpdateJointOrigin { joint_id, origin } => {
            handle_update_joint_origin(joint_id, origin, ctx)
        }
        AppAction::UpdateJointAxis { joint_id, axis } => {
            handle_update_joint_axis(joint_id, axis, ctx)
        }
        AppAction::UpdateJointLimits { joint_id, limits } => {
            handle_update_joint_limits(joint_id, limits, ctx)
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
        if state.project.assembly.parent.contains_key(&child_link_id)
            && let Err(e) = state.project.assembly.disconnect(child_link_id)
        {
            tracing::warn!("Failed to disconnect existing parent: {}", e);
        }

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
        let joint = Joint::fixed(
            format!("{}_to_{}", parent_name, child_name),
            parent_link_id,
            child_link_id,
            Pose::default(),
        );

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

                // Update world transforms after connection
                state
                    .project
                    .assembly
                    .update_world_transforms_with_current_positions();

                // Update renderer transforms
                sync_renderer_transforms(&state, ctx);
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

                // Update world transforms after disconnection
                state
                    .project
                    .assembly
                    .update_world_transforms_with_current_positions();

                // Update renderer transforms
                sync_renderer_transforms(&state, ctx);
            }
            Err(e) => {
                tracing::error!("Failed to disconnect part: {}", e);
            }
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
    if let Some(part) = state.get_part(part_id) {
        let link = Link::from_part(part);
        let link_id = state.project.assembly.add_link(link);
        Some(link_id)
    } else {
        None
    }
}

fn handle_update_joint_position(joint_id: Uuid, position: f32, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    // Clamp to limits if applicable
    let clamped_position = if let Some(joint) = state.project.assembly.joints.get(&joint_id) {
        if let Some(limits) = &joint.limits {
            position.clamp(limits.lower, limits.upper)
        } else {
            position
        }
    } else {
        position
    };

    state
        .project
        .assembly
        .set_joint_position(joint_id, clamped_position);

    // Update world transforms with new joint positions
    state
        .project
        .assembly
        .update_world_transforms_with_current_positions();

    // Update renderer transforms
    sync_renderer_transforms(&state, ctx);
}

fn handle_reset_joint_position(joint_id: Uuid, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    state.project.assembly.reset_joint_position(joint_id);

    // Update world transforms with new joint positions
    state
        .project
        .assembly
        .update_world_transforms_with_current_positions();

    // Update renderer transforms
    sync_renderer_transforms(&state, ctx);
}

fn handle_reset_all_joint_positions(ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    state.project.assembly.reset_all_joint_positions();

    // Update world transforms (all joints at 0)
    state.project.assembly.update_world_transforms();

    // Update renderer transforms
    sync_renderer_transforms(&state, ctx);
}

fn handle_set_editing_joint(joint_id: Option<Uuid>, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    state.editing_joint_id = joint_id;
    tracing::debug!("Set editing joint: {:?}", joint_id);
}

/// Sync renderer transforms with assembly world transforms
fn sync_renderer_transforms(state: &AppState, ctx: &ActionContext) {
    if let Some(viewport_state) = ctx.viewport_state {
        let mut vp = viewport_state.lock();

        // Use the pre-calculated world transforms from Assembly
        // These are computed by update_world_transforms_with_current_positions()
        // which correctly applies joint.origin and joint motion transforms
        for link in state.project.assembly.links.values() {
            if let Some(part_id) = link.part_id
                && let Some(part) = state.get_part(part_id)
            {
                // Apply link's world transform and part's local origin transform
                let result = link.world_transform * part.origin_transform;
                vp.update_part_transform(part_id, result);
            }
        }
    }
}

// ========== Collision action handlers ==========

fn handle_select_collision(selection: Option<(Uuid, usize)>, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    state.selected_collision = selection;
    tracing::debug!("Selected collision: {:?}", selection);
}

fn handle_add_collision(link_id: Uuid, geometry: GeometryType, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    if let Some(link) = state.project.assembly.get_link_mut(link_id) {
        let collision = CollisionElement {
            name: None,
            origin: Pose::default(),
            geometry,
        };
        link.collisions.push(collision);
        state.modified = true;
        tracing::info!("Added collision to link {}", link_id);
    } else {
        tracing::warn!("Link {} not found for adding collision", link_id);
    }
}

fn handle_remove_collision(link_id: Uuid, index: usize, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    // Clear selection if removing the selected collision
    if state.selected_collision == Some((link_id, index)) {
        state.selected_collision = None;
    }

    if let Some(link) = state.project.assembly.get_link_mut(link_id) {
        if index < link.collisions.len() {
            link.collisions.remove(index);
            state.modified = true;
            tracing::info!("Removed collision {} from link {}", index, link_id);
        } else {
            tracing::warn!(
                "Collision index {} out of bounds for link {}",
                index,
                link_id
            );
        }
    } else {
        tracing::warn!("Link {} not found for removing collision", link_id);
    }
}

fn handle_update_collision_origin(link_id: Uuid, index: usize, origin: Pose, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    if let Some(link) = state.project.assembly.get_link_mut(link_id) {
        if let Some(collision) = link.collisions.get_mut(index) {
            collision.origin = origin;
            state.modified = true;
            tracing::debug!("Updated collision {} origin for link {}", index, link_id);
        } else {
            tracing::warn!(
                "Collision index {} out of bounds for link {}",
                index,
                link_id
            );
        }
    } else {
        tracing::warn!("Link {} not found for updating collision origin", link_id);
    }
}

fn handle_update_collision_geometry(
    link_id: Uuid,
    index: usize,
    geometry: GeometryType,
    ctx: &ActionContext,
) {
    let mut state = ctx.app_state.lock();

    if let Some(link) = state.project.assembly.get_link_mut(link_id) {
        if let Some(collision) = link.collisions.get_mut(index) {
            collision.geometry = geometry;
            state.modified = true;
            tracing::debug!("Updated collision {} geometry for link {}", index, link_id);
        } else {
            tracing::warn!(
                "Collision index {} out of bounds for link {}",
                index,
                link_id
            );
        }
    } else {
        tracing::warn!("Link {} not found for updating collision geometry", link_id);
    }
}

// ========== Joint configuration action handlers ==========

fn handle_update_joint_type(joint_id: Uuid, joint_type: JointType, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    if let Some(joint) = state.project.assembly.get_joint_mut(joint_id) {
        let old_type = joint.joint_type;
        joint.joint_type = joint_type;

        // Add default limits when switching to a type that needs them
        if joint_type.has_limits() && joint.limits.is_none() {
            joint.limits = Some(if joint_type == JointType::Prismatic {
                JointLimits::default_prismatic()
            } else {
                JointLimits::default_revolute()
            });
        }

        // Clear limits when switching to a type that doesn't need them
        if !joint_type.has_limits() {
            joint.limits = None;
        }

        state.modified = true;
        tracing::info!(
            "Updated joint {} type: {:?} -> {:?}",
            joint_id,
            old_type,
            joint_type
        );

        // Update world transforms
        state
            .project
            .assembly
            .update_world_transforms_with_current_positions();

        // Update renderer transforms
        sync_renderer_transforms(&state, ctx);
    } else {
        tracing::warn!("Joint {} not found for updating type", joint_id);
    }
}

fn handle_update_joint_origin(joint_id: Uuid, origin: Pose, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    // Get child link info before modifying joint
    let child_link_id = state
        .project
        .assembly
        .get_joint(joint_id)
        .map(|j| j.child_link);

    // Get child link's old world transform and part info
    let child_part_info = child_link_id.and_then(|child_id| {
        state.project.assembly.get_link(child_id).and_then(|link| {
            link.part_id.map(|part_id| {
                let old_world = link.world_transform;
                (child_id, part_id, old_world)
            })
        })
    });

    if let Some(joint) = state.project.assembly.get_joint_mut(joint_id) {
        joint.origin = origin;
        state.modified = true;
        tracing::debug!("Updated joint {} origin", joint_id);

        // Update world transforms
        state
            .project
            .assembly
            .update_world_transforms_with_current_positions();

        // Compensate child link's part origin_transform to maintain its world position
        // This makes only the joint move, not the mesh
        if let Some((child_id, part_id, old_child_world)) = child_part_info
            && let Some(new_child_link) = state.project.assembly.get_link(child_id)
        {
            let new_child_world = new_child_link.world_transform;
            // Calculate compensation: new_origin = new_world^-1 * old_world * old_origin
            // This keeps the part's world position unchanged
            let compensation = new_child_world.inverse() * old_child_world;
            if let Some(part) = state.project.get_part_mut(part_id) {
                part.origin_transform = compensation * part.origin_transform;
            }
        }

        // Update renderer transforms
        sync_renderer_transforms(&state, ctx);
    } else {
        tracing::warn!("Joint {} not found for updating origin", joint_id);
    }
}

fn handle_update_joint_axis(joint_id: Uuid, axis: Vec3, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    let found = if let Some(joint) = state.project.assembly.get_joint_mut(joint_id) {
        joint.axis = axis.normalize();
        state.modified = true;
        tracing::debug!("Updated joint {} axis", joint_id);
        true
    } else {
        false
    };

    if found {
        // Update world transforms
        state
            .project
            .assembly
            .update_world_transforms_with_current_positions();

        // Update renderer transforms
        sync_renderer_transforms(&state, ctx);
    } else {
        tracing::warn!("Joint {} not found for updating axis", joint_id);
    }
}

fn handle_update_joint_limits(joint_id: Uuid, limits: Option<JointLimits>, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();

    // Update the joint limits
    let updated = {
        if let Some(joint) = state.project.assembly.get_joint_mut(joint_id) {
            joint.limits = limits;
            tracing::debug!("Updated joint {} limits", joint_id);
            true
        } else {
            tracing::warn!("Joint {} not found for updating limits", joint_id);
            false
        }
    };

    if !updated {
        return;
    }

    state.modified = true;

    // Clamp current joint position to new limits if necessary
    if let Some(limits) = limits {
        let current_pos = state.project.assembly.get_joint_position(joint_id);
        let clamped = current_pos.clamp(limits.lower, limits.upper);
        if clamped != current_pos {
            state.project.assembly.set_joint_position(joint_id, clamped);

            // Update world transforms
            state
                .project
                .assembly
                .update_world_transforms_with_current_positions();

            // Update renderer transforms
            sync_renderer_transforms(&state, ctx);
        }
    }
}
