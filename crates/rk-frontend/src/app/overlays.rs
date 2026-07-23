//! Overlay update logic

use crate::state::{SharedAppState, SharedViewportState};

/// Update overlays based on current selection
pub fn update_overlays(app_state: &SharedAppState, viewport_state: &Option<SharedViewportState>) {
    let Some(viewport_state) = viewport_state else {
        return;
    };

    // Snapshot the selection and engine handle, then release the UI lock
    let (engine, editing_joint_id, selected_collision, selected_part) = {
        let state = app_state.lock();
        (
            state.engine.clone(),
            state.editing_joint_id,
            state.selected_collision,
            state.selected_part,
        )
    };

    // First check if a joint is being edited (highest priority)
    if let Some(joint_id) = editing_joint_id {
        let joint_info = {
            let eng = engine.lock();
            eng.assembly().joints.get(&joint_id).and_then(|joint| {
                eng.assembly()
                    .get_link(joint.parent_link)
                    .map(|parent_link| (parent_link.world_transform, joint.origin.to_mat4()))
            })
        };
        if let Some((parent_link_world_transform, joint_origin)) = joint_info {
            let mut vp = viewport_state.lock();
            // Clear part-specific overlays but keep gizmo for joint
            let queue = vp.queue.clone();
            vp.renderer.update_axes(&queue, &[]);
            vp.show_gizmo_for_joint(joint_id, parent_link_world_transform, joint_origin);
            return;
        }
    }

    // Check if a collision is selected (takes priority over part selection)
    if let Some((link_id, collision_index)) = selected_collision {
        let collision_info = {
            let eng = engine.lock();
            eng.assembly().get_link(link_id).and_then(|link| {
                link.collisions
                    .get(collision_index)
                    .map(|collision| (link.world_transform, collision.origin.to_mat4()))
            })
        };
        if let Some((link_world_transform, collision_origin)) = collision_info {
            let mut vp = viewport_state.lock();
            // Clear part-specific overlays but keep gizmo for collision
            let queue = vp.queue.clone();
            vp.renderer.update_axes(&queue, &[]);
            vp.show_gizmo_for_collision(
                link_id,
                collision_index,
                link_world_transform,
                collision_origin,
            );
            return;
        }
    }

    // Check for part selection
    if let Some(part_id) = selected_part {
        let part = engine.lock().part(part_id).cloned();
        if let Some(part) = part {
            let mut vp = viewport_state.lock();
            vp.update_axes_for_part(&part);
            // Show gizmo at part center
            vp.show_gizmo_for_part(&part);
            return;
        }
    }

    // No selection - clear overlays
    viewport_state.lock().clear_overlays();
}
