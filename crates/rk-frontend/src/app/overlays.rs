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
            let part_clone = part.clone();
            drop(state);

            let mut vp = viewport_state.lock();
            vp.update_axes_for_part(&part_clone);

            // Show gizmo at part center
            vp.show_gizmo_for_part(&part_clone);
        }
    } else {
        // No selection - clear overlays
        viewport_state.lock().clear_overlays();
    }
}
