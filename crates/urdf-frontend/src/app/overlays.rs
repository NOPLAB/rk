//! Overlay update logic

use crate::state::{SharedAppState, SharedViewportState};

/// Update overlays based on current selection
pub fn update_overlays(app_state: &SharedAppState, viewport_state: &Option<SharedViewportState>) {
    let Some(viewport_state) = viewport_state else {
        return;
    };

    let state = app_state.lock();
    if let Some(part_id) = state.selected_part {
        if let Some(part) = state.get_part(part_id) {
            let selected_point = state.selected_joint_point.map(|(_, idx)| idx);
            let part_clone = part.clone();
            let joint_points: Vec<_> = state
                .project
                .assembly
                .get_joint_points_for_part(part_id)
                .into_iter()
                .cloned()
                .collect();
            drop(state);

            let mut vp = viewport_state.lock();
            vp.update_axes_for_part(&part_clone);
            vp.update_markers_for_part(&part_clone, &joint_points, selected_point);

            // Show gizmo based on selection
            if let Some(point_idx) = selected_point {
                // Show gizmo at joint point position
                vp.show_gizmo_for_joint_point(&part_clone, &joint_points, point_idx);
            } else {
                // Show gizmo at part center
                vp.show_gizmo_for_part(&part_clone);
            }
        }
    } else {
        // No selection - clear overlays
        viewport_state.lock().clear_overlays();
    }
}
