//! Overlay update logic

use crate::state::{SharedAppState, SharedViewportState};

/// Update overlays based on current selection
pub fn update_overlays(app_state: &SharedAppState, viewport_state: &Option<SharedViewportState>) {
    let Some(viewport_state) = viewport_state else {
        return;
    };

    let state = app_state.lock();

    // First check if a collision is selected (takes priority over part selection)
    if let Some((link_id, collision_index)) = state.selected_collision
        && let Some(link) = state.project.assembly.get_link(link_id)
        && let Some(collision) = link.collisions.get(collision_index)
    {
        let link_world_transform = link.world_transform;
        let collision_origin = collision.origin.to_mat4();
        drop(state);

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

    // Check for part selection
    if let Some(part_id) = state.selected_part
        && let Some(part) = state.get_part(part_id)
    {
        let part_clone = part.clone();
        drop(state);

        let mut vp = viewport_state.lock();
        vp.update_axes_for_part(&part_clone);

        // Show gizmo at part center
        vp.show_gizmo_for_part(&part_clone);
        return;
    }

    // No selection - clear overlays
    drop(state);
    viewport_state.lock().clear_overlays();
}
