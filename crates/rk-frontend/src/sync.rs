//! Engine event → renderer/UI synchronization.
//!
//! This is the single place where engine changes reach the viewport.
//! Fine-grained events map to incremental renderer updates;
//! `DocumentReset` rebuilds the whole scene from the engine.

use glam::Vec3;
use rk_engine::{Event, ResetReason, SharedEngine};

use crate::state::{EditorMode, SharedAppState, SharedViewportState};

/// Apply engine events to the viewport and UI state
pub fn apply_events(
    events: &[Event],
    app_state: &SharedAppState,
    viewport_state: &Option<SharedViewportState>,
) {
    if events.is_empty() {
        return;
    }
    let engine = app_state.lock().engine.clone();

    for event in events {
        match event {
            Event::DocumentReset { reason } => {
                rebuild_viewport_from_engine(*reason, &engine, app_state, viewport_state);
            }

            Event::PartAdded { part_id } => {
                let part = engine.lock().part(*part_id).cloned();
                if let (Some(part), Some(vp_state)) = (part, viewport_state)
                    && !part.vertices.is_empty()
                {
                    let mut vp = vp_state.lock();
                    vp.add_part(&part);
                    // Focus the camera on the new part
                    let center = part.center();
                    let radius = part.size().length() / 2.0;
                    vp.renderer.camera_mut().fit_all(center, radius.max(0.5));
                }
            }

            Event::PartRemoved { part_id } => {
                if let Some(vp_state) = viewport_state {
                    let mut vp = vp_state.lock();
                    vp.remove_part(*part_id);
                    vp.clear_overlays();
                }
                let mut state = app_state.lock();
                if state.selected_part == Some(*part_id) {
                    state.selected_part = None;
                }
            }

            Event::PartAppearanceChanged { part_id } => {
                let color = engine.lock().part(*part_id).map(|p| p.color);
                if let (Some(color), Some(vp_state)) = (color, viewport_state) {
                    vp_state.lock().update_part_color(*part_id, color);
                }
            }

            Event::WorldTransformsChanged { transforms } => {
                if let Some(vp_state) = viewport_state {
                    let mut vp = vp_state.lock();
                    for (part_id, transform) in transforms {
                        vp.update_part_transform(*part_id, *transform);
                    }
                }
            }

            Event::BodiesRebuilt { body_ids } => {
                sync_bodies(body_ids, &engine, viewport_state);
            }

            Event::JointRemoved { joint_id } => {
                let mut state = app_state.lock();
                if state.editing_joint_id == Some(*joint_id) {
                    state.editing_joint_id = None;
                }
            }

            // Panels read these from the engine each frame; collision
            // instances and sketch geometry are re-pulled per frame too
            Event::DocumentSaved { .. }
            | Event::ModifiedChanged { .. }
            | Event::PartRenamed { .. }
            | Event::PartPhysicsChanged { .. }
            | Event::LinkAdded { .. }
            | Event::JointAdded { .. }
            | Event::JointChanged { .. }
            | Event::JointPositionChanged { .. }
            | Event::CollisionAdded { .. }
            | Event::CollisionRemoved { .. }
            | Event::CollisionChanged { .. }
            | Event::SketchAdded { .. }
            | Event::SketchRemoved { .. }
            | Event::SketchRenamed { .. }
            | Event::SketchGeometryChanged { .. }
            | Event::SketchSolved { .. }
            | Event::FeatureAdded { .. }
            | Event::FeatureRemoved { .. }
            | Event::FeatureChanged { .. }
            // Grouping is browser presentation; the egui tree does not show it
            | Event::FeatureGroupsChanged
            | Event::HistoryChanged { .. } => {}
        }
    }
}

/// Full resync after the document was replaced (new/load/import/undo)
fn rebuild_viewport_from_engine(
    reason: ResetReason,
    engine: &SharedEngine,
    app_state: &SharedAppState,
    viewport_state: &Option<SharedViewportState>,
) {
    // Reset document-scoped UI state
    {
        let mut state = app_state.lock();
        state.clear_selections();
        state.editor_mode = EditorMode::Assembly;
    }

    let Some(vp_state) = viewport_state else {
        return;
    };

    // Pull everything the renderer needs while holding the engine briefly
    let (parts, transforms, body_ids) = {
        let eng = engine.lock();
        let parts: Vec<rk_core::Part> = eng
            .parts()
            .filter(|p| !p.vertices.is_empty())
            .cloned()
            .collect();
        (parts, eng.part_render_transforms(), eng.body_ids())
    };

    {
        let mut vp = vp_state.lock();
        vp.clear_parts();
        vp.clear_overlays();
        vp.renderer.clear_cad_bodies();

        let mut total_center = Vec3::ZERO;
        let mut max_radius: f32 = 1.0;
        for part in &parts {
            vp.add_part(part);
            total_center += part.center();
            max_radius = max_radius.max(part.size().length() / 2.0);
        }
        for (part_id, transform) in &transforms {
            vp.update_part_transform(*part_id, *transform);
        }

        // Fit the camera when a document was opened; keep it on undo/redo
        if matches!(reason, ResetReason::Loaded | ResetReason::UrdfImported) && !parts.is_empty() {
            total_center /= parts.len() as f32;
            vp.renderer
                .camera_mut()
                .fit_all(total_center, max_radius * 2.0);
        }
    }

    sync_bodies(&body_ids, engine, viewport_state);
}

/// Re-upload all CAD bodies (meshes are pulled and cached in the engine)
fn sync_bodies(
    body_ids: &[uuid::Uuid],
    engine: &SharedEngine,
    viewport_state: &Option<SharedViewportState>,
) {
    let Some(vp_state) = viewport_state else {
        return;
    };
    let mut eng = engine.lock();
    let mut vp = vp_state.lock();
    let device = vp.device.clone();
    vp.renderer.clear_cad_bodies();

    for &body_id in body_ids {
        match eng.body_mesh(body_id) {
            Ok(mesh) => {
                vp.renderer.add_cad_body(
                    &device,
                    body_id,
                    &mesh.vertices,
                    &mesh.normals,
                    &mesh.indices,
                    glam::Mat4::IDENTITY,
                    [0.7, 0.7, 0.8, 1.0],
                );
            }
            Err(e) => {
                tracing::warn!("Failed to get mesh for body {}: {}", body_id, e);
            }
        }
    }
}
