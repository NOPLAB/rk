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
        AppAction::UpdateJointPosition { joint_id, position } => {
            handle_update_joint_position(joint_id, position, ctx)
        }
        AppAction::ResetJointPosition { joint_id } => handle_reset_joint_position(joint_id, ctx),
        AppAction::ResetAllJointPositions => handle_reset_all_joint_positions(ctx),
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
    if let Some(part) = state.parts.get_mut(&part_id)
        && let Some(idx) = part.joint_points.iter().position(|jp| jp.id == point_id)
    {
        part.joint_points.remove(idx);
        state.modified = true;
        tracing::info!("Removed joint point {} from part {}", point_id, part_id);
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

    state.set_joint_position(joint_id, clamped_position);

    // Update world transforms with new joint positions
    let joint_positions = state.joint_positions.clone();
    state
        .project
        .assembly
        .update_world_transforms_with_positions(&joint_positions);

    // Update renderer transforms
    sync_renderer_transforms(&state, ctx);
}

fn handle_reset_joint_position(joint_id: Uuid, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    state.reset_joint_position(joint_id);

    // Update world transforms with new joint positions
    let joint_positions = state.joint_positions.clone();
    state
        .project
        .assembly
        .update_world_transforms_with_positions(&joint_positions);

    // Update renderer transforms
    sync_renderer_transforms(&state, ctx);
}

fn handle_reset_all_joint_positions(ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    state.reset_all_joint_positions();

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
                && let Some(part) = state.parts.get(&part_id)
            {
                // Collect ancestor joints from this link to root
                // Store: (original_pivot, original_axis, joint_type, joint_value)
                let mut joint_chain: Vec<(Vec3, Vec3, urdf_core::JointType, f32)> = Vec::new();
                let mut current_link_id = *link_id;

                while let Some((joint_id, parent_link_id)) =
                    state.project.assembly.parent.get(&current_link_id)
                {
                    if let Some(joint) = state.project.assembly.joints.get(joint_id) {
                        let joint_pos = state.joint_positions.get(joint_id).copied().unwrap_or(0.0);

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
                    let joint_rotation = urdf_core::Assembly::compute_joint_transform_static(
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
        && let Some(part) = state.parts.get(&part_id)
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
