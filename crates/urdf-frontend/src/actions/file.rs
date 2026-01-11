//! File I/O action handlers

use std::collections::HashMap;

use urdf_core::{ImportOptions, Project, import_urdf, load_stl_with_unit};

use crate::state::AppAction;

use super::ActionContext;

/// Handle file-related actions
pub fn handle_file_action(action: AppAction, ctx: &ActionContext) {
    match action {
        AppAction::ImportStl(path) => handle_import_stl(path, ctx),
        AppAction::ImportUrdf(path) => handle_import_urdf(path, ctx),
        AppAction::SaveProject(path) => handle_save_project(path, ctx),
        AppAction::LoadProject(path) => handle_load_project(path, ctx),
        AppAction::ExportUrdf { path, robot_name } => handle_export_urdf(path, robot_name, ctx),
        AppAction::NewProject => handle_new_project(ctx),
        _ => {}
    }
}

fn handle_import_stl(path: std::path::PathBuf, ctx: &ActionContext) {
    let unit = ctx.app_state.lock().stl_import_unit;
    match load_stl_with_unit(&path, unit) {
        Ok(part) => {
            tracing::info!(
                "Loaded STL: {} ({} vertices, unit={:?})",
                part.name,
                part.vertices.len(),
                unit
            );

            // Calculate bounding sphere for camera fit
            let center = glam::Vec3::new(
                (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
                (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
                (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
            );
            let extent = glam::Vec3::new(
                part.bbox_max[0] - part.bbox_min[0],
                part.bbox_max[1] - part.bbox_min[1],
                part.bbox_max[2] - part.bbox_min[2],
            );
            let radius = extent.length() / 2.0;

            // Add to viewport
            if let Some(viewport_state) = ctx.viewport_state {
                tracing::info!("Adding part to viewport...");
                let mut vp = viewport_state.lock();
                vp.add_part(&part);
                // Auto-fit camera to new part
                vp.renderer.camera_mut().fit_all(center, radius);
                tracing::info!(
                    "Part added, camera fitted to center={:?}, radius={}",
                    center,
                    radius
                );
            } else {
                tracing::warn!("viewport_state is None - cannot add part to renderer");
            }

            // Add to app state
            ctx.app_state.lock().add_part(part);
        }
        Err(e) => {
            tracing::error!("Failed to load STL: {}", e);
        }
    }
}

fn handle_import_urdf(path: std::path::PathBuf, ctx: &ActionContext) {
    let stl_unit = ctx.app_state.lock().stl_import_unit;
    let options = ImportOptions {
        base_dir: path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from(".")),
        stl_unit,
        default_color: [0.7, 0.7, 0.7, 1.0],
        package_paths: HashMap::new(),
    };

    match import_urdf(&path, &options) {
        Ok(project) => {
            tracing::info!(
                "Imported URDF: {} ({} links, {} joints, {} parts)",
                project.name,
                project.assembly.links.len(),
                project.assembly.joints.len(),
                project.parts.len()
            );

            // Clear viewport
            if let Some(viewport_state) = ctx.viewport_state {
                viewport_state.lock().clear_parts();
                viewport_state.lock().clear_overlays();
            }

            // Add parts to viewport and fit camera
            if let Some(viewport_state) = ctx.viewport_state {
                let mut total_center = glam::Vec3::ZERO;
                let mut max_radius: f32 = 1.0;
                let part_count = project.parts.len() as f32;

                for part in &project.parts {
                    viewport_state.lock().add_part(part);

                    // Accumulate for camera fitting
                    let center = glam::Vec3::new(
                        (part.bbox_min[0] + part.bbox_max[0]) / 2.0,
                        (part.bbox_min[1] + part.bbox_max[1]) / 2.0,
                        (part.bbox_min[2] + part.bbox_max[2]) / 2.0,
                    );
                    total_center += center;

                    let extent = glam::Vec3::new(
                        part.bbox_max[0] - part.bbox_min[0],
                        part.bbox_max[1] - part.bbox_min[1],
                        part.bbox_max[2] - part.bbox_min[2],
                    );
                    max_radius = max_radius.max(extent.length() / 2.0);
                }

                if part_count > 0.0 {
                    total_center /= part_count;
                    viewport_state
                        .lock()
                        .renderer
                        .camera_mut()
                        .fit_all(total_center, max_radius * 2.0);
                }
            }

            // Load into app state
            ctx.app_state.lock().load_project(project, path);
        }
        Err(e) => {
            tracing::error!("Failed to import URDF: {}", e);
        }
    }
}

fn handle_save_project(path: Option<std::path::PathBuf>, ctx: &ActionContext) {
    let mut state = ctx.app_state.lock();
    let save_path = path.or(state.project_path.clone());

    if let Some(ref path) = save_path {
        // Sync parts to project
        state.project.parts = state.parts.values().cloned().collect();

        match state.project.save(path) {
            Ok(()) => {
                tracing::info!("Saved project to {:?}", path);
                state.project_path = Some(path.clone());
                state.modified = false;
            }
            Err(e) => {
                tracing::error!("Failed to save project: {}", e);
            }
        }
    }
}

fn handle_load_project(path: std::path::PathBuf, ctx: &ActionContext) {
    match Project::load(&path) {
        Ok(mut project) => {
            tracing::info!("Loaded project: {}", project.name);

            // Ensure joint_points are synced from assembly (for older project files)
            sync_joint_points_from_assembly(&mut project);

            // Clear viewport
            if let Some(viewport_state) = ctx.viewport_state {
                viewport_state.lock().clear_parts();
                viewport_state.lock().clear_overlays();
            }

            // Load parts into viewport
            if let Some(viewport_state) = ctx.viewport_state {
                for part in &project.parts {
                    viewport_state.lock().add_part(part);
                }
            }

            // Load into app state
            ctx.app_state.lock().load_project(project, path);
        }
        Err(e) => {
            tracing::error!("Failed to load project: {}", e);
        }
    }
}

/// Sync joint_points from assembly joints to parts
/// This handles older project files that may not have joint_points saved
fn sync_joint_points_from_assembly(project: &mut Project) {
    use glam::Vec3;
    use urdf_core::JointPoint;

    for joint in project.assembly.joints.values_mut() {
        // Get link names for naming joint points
        let parent_link_name = project
            .assembly
            .links
            .get(&joint.parent_link)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        let child_link_name = project
            .assembly
            .links
            .get(&joint.child_link)
            .map(|l| l.name.clone())
            .unwrap_or_default();

        // Get part IDs from links
        let parent_part_id = project
            .assembly
            .links
            .get(&joint.parent_link)
            .and_then(|l| l.part_id);
        let child_part_id = project
            .assembly
            .links
            .get(&joint.child_link)
            .and_then(|l| l.part_id);

        // Create joint point on parent part if not exists
        if joint.parent_joint_point.is_none()
            && let Some(part_id) = parent_part_id
            && let Some(part) = project.parts.iter_mut().find(|p| p.id == part_id)
        {
            let jp = JointPoint {
                id: uuid::Uuid::new_v4(),
                name: format!("joint_to_{}", child_link_name),
                position: Vec3::new(
                    joint.origin.xyz[0],
                    joint.origin.xyz[1],
                    joint.origin.xyz[2],
                ),
                orientation: glam::Quat::from_euler(
                    glam::EulerRot::XYZ,
                    joint.origin.rpy[0],
                    joint.origin.rpy[1],
                    joint.origin.rpy[2],
                ),
                joint_type: joint.joint_type,
                axis: joint.axis,
                limits: joint.limits,
            };
            let jp_id = jp.id;
            part.joint_points.push(jp);
            joint.parent_joint_point = Some(jp_id);
        }

        // Create joint point on child part if not exists
        if joint.child_joint_point.is_none()
            && let Some(part_id) = child_part_id
            && let Some(part) = project.parts.iter_mut().find(|p| p.id == part_id)
        {
            let jp = JointPoint {
                id: uuid::Uuid::new_v4(),
                name: format!("joint_from_{}", parent_link_name),
                position: Vec3::ZERO,
                orientation: glam::Quat::IDENTITY,
                joint_type: joint.joint_type,
                axis: joint.axis,
                limits: joint.limits,
            };
            let jp_id = jp.id;
            part.joint_points.push(jp);
            joint.child_joint_point = Some(jp_id);
        }
    }
}

fn handle_export_urdf(path: std::path::PathBuf, robot_name: String, ctx: &ActionContext) {
    let state = ctx.app_state.lock();
    let options = urdf_core::ExportOptions {
        output_dir: path,
        robot_name,
        mesh_prefix: "meshes".to_string(),
        use_package_uri: false,
    };

    match urdf_core::export_urdf(&state.project.assembly, &state.parts, &options) {
        Ok(_urdf) => {
            tracing::info!("Exported URDF to {:?}", options.output_dir);
        }
        Err(e) => {
            tracing::error!("Failed to export URDF: {}", e);
        }
    }
}

fn handle_new_project(ctx: &ActionContext) {
    ctx.app_state.lock().new_project();
    if let Some(viewport_state) = ctx.viewport_state {
        viewport_state.lock().clear_parts();
        viewport_state.lock().clear_overlays();
    }
}
