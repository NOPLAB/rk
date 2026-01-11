//! Part-related action handlers

use glam::Mat4;
use uuid::Uuid;

use urdf_core::{Part, generate_box_mesh, generate_cylinder_mesh, generate_sphere_mesh};

use crate::state::{AppAction, PrimitiveType};

use super::ActionContext;

/// Handle part-related actions
pub fn handle_part_action(action: AppAction, ctx: &ActionContext) {
    match action {
        AppAction::CreatePrimitive {
            primitive_type,
            name,
        } => handle_create_primitive(primitive_type, name, ctx),
        AppAction::CreateEmpty { name } => handle_create_empty(name, ctx),
        AppAction::SelectPart(part_id) => handle_select_part(part_id, ctx),
        AppAction::DeleteSelectedPart => handle_delete_selected_part(ctx),
        AppAction::UpdatePartTransform { part_id, transform } => {
            handle_update_part_transform(part_id, transform, ctx)
        }
        _ => {}
    }
}

fn handle_create_primitive(
    primitive_type: PrimitiveType,
    name: Option<String>,
    ctx: &ActionContext,
) {
    // Generate unique name
    let existing_count = ctx.app_state.lock().parts.len();
    let part_name =
        name.unwrap_or_else(|| format!("{}_{}", primitive_type.name(), existing_count + 1));

    // Generate mesh based on primitive type (default size: 0.1m)
    let (vertices, normals, indices) = match primitive_type {
        PrimitiveType::Box => generate_box_mesh([0.1, 0.1, 0.1]),
        PrimitiveType::Cylinder => generate_cylinder_mesh(0.05, 0.1),
        PrimitiveType::Sphere => generate_sphere_mesh(0.05),
    };

    // Create part
    let mut part = Part::new(&part_name);
    part.vertices = vertices;
    part.normals = normals;
    part.indices = indices;
    part.calculate_bounding_box();

    // Set material name based on primitive type
    part.material_name = Some(format!("{}_material", primitive_type.name().to_lowercase()));

    tracing::info!(
        "Created primitive: {} ({} vertices)",
        part.name,
        part.vertices.len()
    );

    // Add to viewport
    if let Some(viewport_state) = ctx.viewport_state {
        let mut vp = viewport_state.lock();
        vp.add_part(&part);

        // Fit camera if this is the first part
        let center = part.center();
        let radius = part.size().length() / 2.0;
        vp.renderer.camera_mut().fit_all(center, radius.max(0.5));
    }

    // Add to app state
    ctx.app_state.lock().add_part(part);
}

fn handle_create_empty(name: Option<String>, ctx: &ActionContext) {
    // Generate unique name
    let existing_count = ctx.app_state.lock().parts.len();
    let part_name = name.unwrap_or_else(|| format!("Empty_{}", existing_count + 1));

    // Create empty part (no geometry)
    let part = Part::new(&part_name);

    tracing::info!("Created empty part: {}", part.name);

    // Add to app state (no viewport mesh for empty parts)
    ctx.app_state.lock().add_part(part);
}

fn handle_select_part(part_id: Option<Uuid>, ctx: &ActionContext) {
    ctx.app_state.lock().select_part(part_id);

    // Update viewport selection (mesh highlighting)
    if let Some(viewport_state) = ctx.viewport_state {
        viewport_state.lock().set_selected_part(part_id);
    }
    // Overlays are updated in update_overlays() called after process_actions
}

fn handle_delete_selected_part(ctx: &ActionContext) {
    let selected = ctx.app_state.lock().selected_part;
    if let Some(id) = selected {
        ctx.app_state.lock().remove_part(id);

        if let Some(viewport_state) = ctx.viewport_state {
            viewport_state.lock().remove_part(id);
            viewport_state.lock().clear_overlays();
        }
    }
}

fn handle_update_part_transform(part_id: Uuid, transform: Mat4, ctx: &ActionContext) {
    if let Some(part) = ctx.app_state.lock().get_part_mut(part_id) {
        part.origin_transform = transform;
    }
    if let Some(viewport_state) = ctx.viewport_state {
        viewport_state
            .lock()
            .update_part_transform(part_id, transform);
    }
}
