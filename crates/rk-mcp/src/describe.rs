//! Structured scene description for agents.
//!
//! Builds a JSON snapshot of everything an agent needs to reason about
//! the document: parts, assembly graph, sketches, features, bodies and
//! history state. Meshes are intentionally excluded (pull model); use
//! the screenshot tool to *see* the scene.

use rk_engine::Engine;
use serde_json::{Value, json};

/// Full document snapshot as JSON
pub fn describe_scene(engine: &Engine) -> Value {
    let project = engine.project();
    let assembly = engine.assembly();
    let cad = &engine.document().cad;
    let transforms = engine.part_render_transforms();

    let parts: Vec<Value> = engine
        .parts()
        .map(|part| {
            let world = transforms
                .iter()
                .find(|(id, _)| *id == part.id)
                .map(|(_, t)| *t)
                .unwrap_or(part.origin_transform);
            let (_, rotation, translation) = world.to_scale_rotation_translation();
            json!({
                "id": part.id,
                "name": part.name,
                "has_mesh": !part.vertices.is_empty(),
                "triangle_count": part.indices.len() / 3,
                "world_position": translation.to_array(),
                "world_rotation_quat": rotation.to_array(),
                "bbox_min": part.bbox_min,
                "bbox_max": part.bbox_max,
                "color": part.color,
                "mass": part.mass,
                "material": part.material_name,
            })
        })
        .collect();

    let links: Vec<Value> = assembly
        .links
        .values()
        .map(|link| {
            json!({
                "id": link.id,
                "name": link.name,
                "part_id": link.part_id,
                "collision_count": link.collisions.len(),
            })
        })
        .collect();

    let joints: Vec<Value> = assembly
        .joints
        .values()
        .map(|joint| {
            json!({
                "id": joint.id,
                "name": joint.name,
                "type": joint.joint_type,
                "parent_link": joint.parent_link,
                "child_link": joint.child_link,
                "origin": joint.origin,
                "axis": joint.axis.to_array(),
                "limits": joint.limits,
                "position": assembly.joint_positions.get(&joint.id).copied().unwrap_or(0.0),
            })
        })
        .collect();

    let sketches: Vec<Value> = cad
        .history
        .sketches()
        .values()
        .map(|sketch| {
            json!({
                "id": sketch.id,
                "name": sketch.name,
                "plane": sketch.plane,
                "entities": sketch.entities().values().collect::<Vec<_>>(),
                "constraints": sketch.constraints().values().collect::<Vec<_>>(),
            })
        })
        .collect();

    let features: Vec<Value> = engine
        .features()
        .iter()
        .map(|entry| {
            json!({
                "id": entry.feature.id(),
                "name": entry.feature.name(),
                "type": entry.feature.type_name(),
                "suppressed": entry.feature.is_suppressed(),
                "created_bodies": entry.created_bodies,
            })
        })
        .collect();

    let bodies: Vec<Value> = cad
        .history
        .bodies()
        .values()
        .map(|body| {
            json!({
                "id": body.id,
                "name": body.name,
                "source_feature": body.source_feature,
            })
        })
        .collect();

    json!({
        "project_name": project.name,
        "doc_path": engine.doc_path(),
        "modified": engine.is_modified(),
        "revision": engine.revision(),
        "history": {
            "can_undo": engine.can_undo(),
            "can_redo": engine.can_redo(),
            "undo_description": engine.undo_description(),
        },
        "parts": parts,
        "links": links,
        "joints": joints,
        "sketches": sketches,
        "features": features,
        "bodies": bodies,
    })
}
