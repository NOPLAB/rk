//! Assembly-related action handlers

use uuid::Uuid;

use rk_core::{CollisionElement, GeometryType, Joint, Link, Pose};

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

/// Sync renderer transforms with assembly world transforms
fn sync_renderer_transforms(state: &AppState, ctx: &ActionContext) {
    use glam::{Mat4, Quat, Vec3};

    if let Some(viewport_state) = ctx.viewport_state {
        let mut vp = viewport_state.lock();

        // For each part, collect all ancestor joints and their properties
        // Then apply transforms while updating pivot positions and axes
        for (link_id, link) in &state.project.assembly.links {
            if let Some(part_id) = link.part_id
                && let Some(part) = state.get_part(part_id)
            {
                // Collect ancestor joints from this link to root
                // Store: (original_pivot, original_axis, joint_type, joint_value)
                let mut joint_chain: Vec<(Vec3, Vec3, rk_core::JointType, f32)> = Vec::new();
                let mut current_link_id = *link_id;

                while let Some((joint_id, parent_link_id)) =
                    state.project.assembly.parent.get(&current_link_id)
                {
                    if let Some(joint) = state.project.assembly.joints.get(joint_id) {
                        let joint_pos = state.project.assembly.get_joint_position(*joint_id);

                        // Get parent part's center as original joint pivot point
                        let original_pivot = get_part_center(state, *parent_link_id);

                        joint_chain.push((original_pivot, joint.axis, joint.joint_type, joint_pos));
                    }
                    current_link_id = *parent_link_id;
                }

                // Apply transforms from root to leaf (reverse the chain)
                joint_chain.reverse();

                // Track accumulated transform and rotation to update pivot positions and axes
                let mut accumulated_transform = Mat4::IDENTITY;
                let mut accumulated_rotation = Quat::IDENTITY;

                // Apply each joint's rotation around its transformed pivot with transformed axis
                for (original_pivot, original_axis, joint_type, joint_value) in &joint_chain {
                    // Transform the pivot and axis by all previous joint transforms
                    let current_pivot = accumulated_transform.transform_point3(*original_pivot);
                    let current_axis = accumulated_rotation * *original_axis;

                    // Compute joint rotation with transformed axis
                    let joint_rotation = rk_core::Assembly::compute_joint_transform(
                        joint_type,
                        current_axis,
                        *joint_value,
                    );

                    // Extract rotation part for axis transformation
                    let (_, rot, _) = joint_rotation.to_scale_rotation_translation();
                    accumulated_rotation = rot * accumulated_rotation;

                    // Create rotation around the current pivot
                    let to_pivot = Mat4::from_translation(-current_pivot);
                    let from_pivot = Mat4::from_translation(current_pivot);
                    let this_transform = from_pivot * joint_rotation * to_pivot;

                    // Accumulate the transform
                    accumulated_transform = this_transform * accumulated_transform;
                }

                // Apply accumulated transform to the part's original transform
                let result = accumulated_transform * part.origin_transform;
                vp.update_part_transform(part_id, result);
            }
        }
    }
}

/// Get the world-space center of a part associated with a link
fn get_part_center(state: &AppState, link_id: Uuid) -> glam::Vec3 {
    if let Some(link) = state.project.assembly.links.get(&link_id)
        && let Some(part_id) = link.part_id
        && let Some(part) = state.get_part(part_id)
    {
        let center = glam::Vec3::new(
            (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
            (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
            (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
        );
        return part.origin_transform.transform_point3(center);
    }
    glam::Vec3::ZERO
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
