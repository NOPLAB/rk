//! rk-desktop: Tauri 2 shell around the headless RK engine.
//!
//! The webview (React + Three.js) is a pure client of `rk-engine`: it sends
//! `Command` batches through `engine_apply` and pulls bulk data (meshes,
//! scene snapshots) by ID — the same protocol shape the MCP server uses.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::Arc;

use rk_engine::{Command, Engine, Event, SharedEngine};
use tauri::State;
use uuid::Uuid;

/// Default CAD body display color (matches the egui viewport)
const BODY_COLOR: [f32; 4] = [0.7, 0.7, 0.8, 1.0];

struct EngineState {
    engine: SharedEngine,
}

#[derive(serde::Serialize)]
struct ApplyOutcome {
    applied: usize,
    events: Vec<Event>,
    error: Option<ApplyFailure>,
}

#[derive(serde::Serialize)]
struct ApplyFailure {
    index: usize,
    message: String,
}

/// Apply a batch of engine commands.
///
/// All commands are parsed up front — a malformed batch is rejected without
/// applying anything. On an engine error the batch stops there; earlier
/// commands stay applied (undo can revert them) and the outcome reports the
/// failing index.
#[tauri::command]
fn engine_apply(
    state: State<'_, EngineState>,
    commands: Vec<serde_json::Value>,
) -> Result<ApplyOutcome, String> {
    let parsed: Vec<Command> = commands
        .into_iter()
        .enumerate()
        .map(|(i, raw)| {
            serde_json::from_value(raw)
                .map_err(|e| format!("commands[{i}] is not a valid command: {e}"))
        })
        .collect::<Result<_, _>>()?;

    let total = parsed.len();
    let mut engine = state.engine.lock();
    let mut events = Vec::new();
    for (i, cmd) in parsed.into_iter().enumerate() {
        match engine.apply(cmd) {
            Ok(evs) => events.extend(evs),
            Err(e) => {
                return Ok(ApplyOutcome {
                    applied: i,
                    events,
                    error: Some(ApplyFailure {
                        index: i,
                        message: e.to_string(),
                    }),
                });
            }
        }
    }
    Ok(ApplyOutcome {
        applied: total,
        events,
        error: None,
    })
}

/// Apply one command inside an interaction session (gizmo drag). The whole
/// session collapses into a single undo step.
#[tauri::command]
fn engine_apply_interactive(
    state: State<'_, EngineState>,
    session: Uuid,
    command: serde_json::Value,
) -> Result<ApplyOutcome, String> {
    let cmd: Command =
        serde_json::from_value(command).map_err(|e| format!("not a valid command: {e}"))?;
    let mut engine = state.engine.lock();
    match engine.apply_interactive(session, cmd) {
        Ok(events) => Ok(ApplyOutcome {
            applied: 1,
            events,
            error: None,
        }),
        Err(e) => Ok(ApplyOutcome {
            applied: 0,
            events: Vec::new(),
            error: Some(ApplyFailure {
                index: 0,
                message: e.to_string(),
            }),
        }),
    }
}

/// Close an interaction session. `cancel` rolls the document back to the
/// state from before the drag started.
#[tauri::command]
fn engine_end_interaction(
    state: State<'_, EngineState>,
    session: Uuid,
    cancel: bool,
) -> Result<ApplyOutcome, String> {
    let mut engine = state.engine.lock();
    let events = engine
        .end_interaction(session, cancel)
        .map_err(|e| e.to_string())?;
    Ok(ApplyOutcome {
        applied: 1,
        events,
        error: None,
    })
}

#[derive(serde::Serialize)]
struct PartInfo {
    id: Uuid,
    name: String,
    color: [f32; 4],
    has_mesh: bool,
    origin_transform: glam::Mat4,
    /// World transform of the link owning this part (identity when the part
    /// is not in the assembly). `render = parent_transform × origin_transform`
    parent_transform: glam::Mat4,
    mass: f32,
    inertia: rk_core::InertiaMatrix,
    /// Mesh bounds in part space, for fitting a collision shape to the part
    bbox_min: [f32; 3],
    bbox_max: [f32; 3],
}

#[derive(serde::Serialize)]
struct HistoryInfo {
    can_undo: bool,
    can_redo: bool,
    undo_description: Option<String>,
}

#[derive(serde::Serialize)]
struct CollisionInfo {
    /// Position in the link's collision list — commands address it by index
    index: usize,
    name: Option<String>,
    origin: rk_core::Pose,
    geometry: rk_core::GeometryType,
    /// `link world × origin`, so the viewport never re-derives the euler
    /// convention behind `Pose::rpy`
    transform: glam::Mat4,
}

#[derive(serde::Serialize)]
struct LinkInfo {
    id: Uuid,
    name: String,
    part_id: Option<Uuid>,
    world_transform: glam::Mat4,
    collisions: Vec<CollisionInfo>,
}

#[derive(serde::Serialize)]
struct JointInfo {
    id: Uuid,
    name: String,
    joint_type: rk_core::JointType,
    parent_link: Uuid,
    child_link: Uuid,
    /// Parts behind the links, pre-resolved so the UI doesn't have to
    parent_part: Option<Uuid>,
    child_part: Option<Uuid>,
    origin: rk_core::Pose,
    axis: [f32; 3],
    limits: Option<rk_core::JointLimits>,
    position: f32,
}

#[derive(serde::Serialize)]
struct SketchInfo {
    id: Uuid,
    name: String,
    plane: rk_cad::SketchPlane,
    /// Sketch space → world, so the UI can place 2D geometry without
    /// rebuilding the basis itself
    transform: glam::Mat4,
    entity_count: usize,
    constraint_count: usize,
    is_solved: bool,
    dof: u32,
    /// Closed profiles the sketch yields — extrude needs at least one
    profile_count: usize,
}

#[derive(serde::Serialize)]
struct FeatureInfo {
    id: Uuid,
    name: String,
    /// `Feature::type_name()`: "Extrude", "Revolve", ...
    kind: &'static str,
    suppressed: bool,
    /// Sketch the feature is built from (extrude/revolve only)
    sketch_id: Option<Uuid>,
    created_bodies: Vec<Uuid>,
}

#[derive(serde::Serialize)]
struct SceneSnapshot {
    project_name: String,
    doc_path: Option<String>,
    modified: bool,
    revision: u64,
    parts: Vec<PartInfo>,
    /// Final render transforms (link world × part origin), column-major
    transforms: Vec<(Uuid, glam::Mat4)>,
    body_ids: Vec<Uuid>,
    links: Vec<LinkInfo>,
    joints: Vec<JointInfo>,
    sketches: Vec<SketchInfo>,
    features: Vec<FeatureInfo>,
    /// Features from this index on are rolled back (inactive); `None` = all active
    rollback_position: Option<usize>,
    history: HistoryInfo,
}

/// Lightweight UI snapshot: part metadata, render transforms, body IDs and
/// undo state. Mesh data is pulled separately per ID.
#[tauri::command]
fn scene_snapshot(state: State<'_, EngineState>) -> SceneSnapshot {
    let engine = state.engine.lock();
    let assembly = engine.assembly();
    let part_of = |link_id: Uuid| assembly.links.get(&link_id).and_then(|l| l.part_id);
    let link_world: HashMap<Uuid, glam::Mat4> = assembly
        .links
        .values()
        .filter_map(|l| l.part_id.map(|p| (p, l.world_transform)))
        .collect();
    let mut links: Vec<LinkInfo> = assembly
        .links
        .values()
        .map(|l| LinkInfo {
            id: l.id,
            name: l.name.clone(),
            part_id: l.part_id,
            world_transform: l.world_transform,
            collisions: l
                .collisions
                .iter()
                .enumerate()
                .map(|(index, c)| CollisionInfo {
                    index,
                    name: c.name.clone(),
                    origin: c.origin,
                    geometry: c.geometry.clone(),
                    transform: l.world_transform * c.origin.to_mat4(),
                })
                .collect(),
        })
        .collect();
    links.sort_by(|a, b| a.name.cmp(&b.name));
    let mut joints: Vec<JointInfo> = assembly
        .joints
        .values()
        .map(|j| JointInfo {
            id: j.id,
            name: j.name.clone(),
            joint_type: j.joint_type,
            parent_link: j.parent_link,
            child_link: j.child_link,
            parent_part: part_of(j.parent_link),
            child_part: part_of(j.child_link),
            origin: j.origin,
            axis: j.axis.to_array(),
            limits: j.limits,
            position: assembly.get_joint_position(j.id),
        })
        .collect();
    joints.sort_by(|a, b| a.name.cmp(&b.name));

    let history = &engine.document().cad.history;
    let mut sketches: Vec<SketchInfo> = history
        .sketches()
        .values()
        .map(|s| SketchInfo {
            id: s.id,
            name: s.name.clone(),
            plane: s.plane,
            transform: s.plane.transform(),
            entity_count: s.entities().len(),
            constraint_count: s.constraints().len(),
            is_solved: s.is_solved(),
            dof: s.degrees_of_freedom(),
            profile_count: s.extract_profiles().map(|p| p.len()).unwrap_or(0),
        })
        .collect();
    sketches.sort_by(|a, b| a.name.cmp(&b.name));
    // Features keep history order — the list is the model's build sequence
    let features: Vec<FeatureInfo> = history
        .entries()
        .iter()
        .map(|e| FeatureInfo {
            id: e.feature.id(),
            name: e.feature.name().to_owned(),
            kind: e.feature.type_name(),
            suppressed: e.feature.is_suppressed(),
            sketch_id: match &e.feature {
                rk_cad::Feature::Extrude { sketch_id, .. }
                | rk_cad::Feature::Revolve { sketch_id, .. } => Some(*sketch_id),
                _ => None,
            },
            created_bodies: e.created_bodies.clone(),
        })
        .collect();
    let rollback_position = history.rollback_position();

    SceneSnapshot {
        project_name: engine.project().name.clone(),
        doc_path: engine.doc_path().map(|p| p.display().to_string()),
        modified: engine.is_modified(),
        revision: engine.revision(),
        parts: engine
            .parts()
            .map(|p| PartInfo {
                id: p.id,
                name: p.name.clone(),
                color: p.color,
                has_mesh: !p.vertices.is_empty(),
                origin_transform: p.origin_transform,
                parent_transform: link_world
                    .get(&p.id)
                    .copied()
                    .unwrap_or(glam::Mat4::IDENTITY),
                mass: p.mass,
                inertia: p.inertia,
                bbox_min: p.bbox_min,
                bbox_max: p.bbox_max,
            })
            .collect(),
        transforms: engine.part_render_transforms(),
        body_ids: engine.body_ids(),
        links,
        joints,
        sketches,
        features,
        rollback_position,
        history: HistoryInfo {
            can_undo: engine.can_undo(),
            can_redo: engine.can_redo(),
            undo_description: engine.undo_description().map(str::to_owned),
        },
    }
}

#[derive(serde::Serialize)]
struct SketchPointGeom {
    id: Uuid,
    position: [f32; 2],
    construction: bool,
}

#[derive(serde::Serialize)]
struct SketchLineGeom {
    id: Uuid,
    start: [f32; 2],
    end: [f32; 2],
    /// Endpoint IDs, so clicks can snap onto and reuse existing points
    start_id: Uuid,
    end_id: Uuid,
    construction: bool,
}

#[derive(serde::Serialize)]
struct SketchCircleGeom {
    id: Uuid,
    center: [f32; 2],
    radius: f32,
    construction: bool,
}

#[derive(serde::Serialize)]
struct SketchArcGeom {
    id: Uuid,
    center: [f32; 2],
    radius: f32,
    /// Sweep from `start_angle` counter-clockwise to `end_angle` (radians)
    start_angle: f32,
    end_angle: f32,
    construction: bool,
}

#[derive(serde::Serialize)]
struct SketchConstraintInfo {
    id: Uuid,
    /// The constraint exactly as `add_sketch_constraint` takes it. Sending it
    /// back with a different value replaces it — constraints are keyed by ID
    constraint: rk_cad::SketchConstraint,
    /// `SketchConstraint::type_name()`, e.g. "Equal Length"
    label: &'static str,
    /// Referenced entities, for highlighting the constraint in the viewport
    entities: Vec<Uuid>,
    /// The driven value, or `None` for a purely geometric constraint
    value: Option<f32>,
}

/// Sketch entities with point references already resolved to coordinates —
/// the viewport draws straight from this without chasing IDs.
#[derive(serde::Serialize)]
struct SketchGeometry {
    points: Vec<SketchPointGeom>,
    lines: Vec<SketchLineGeom>,
    circles: Vec<SketchCircleGeom>,
    arcs: Vec<SketchArcGeom>,
    constraints: Vec<SketchConstraintInfo>,
}

#[tauri::command]
fn sketch_geometry(
    state: State<'_, EngineState>,
    sketch_id: Uuid,
) -> Result<SketchGeometry, String> {
    let engine = state.engine.lock();
    let sketch = engine
        .sketch(sketch_id)
        .ok_or_else(|| format!("unknown sketch: {sketch_id}"))?;
    let pos = |id: Uuid| match sketch.get_entity(id) {
        Some(rk_cad::SketchEntity::Point { position, .. }) => Some(position.to_array()),
        _ => None,
    };
    let angle = |center: [f32; 2], p: [f32; 2]| (p[1] - center[1]).atan2(p[0] - center[0]);

    let mut geom = SketchGeometry {
        points: Vec::new(),
        lines: Vec::new(),
        circles: Vec::new(),
        arcs: Vec::new(),
        constraints: Vec::new(),
    };
    for entity in sketch.entities_iter() {
        let construction = sketch.is_construction(entity.id());
        match entity {
            rk_cad::SketchEntity::Point { id, position } => geom.points.push(SketchPointGeom {
                id: *id,
                position: position.to_array(),
                construction,
            }),
            rk_cad::SketchEntity::Line { id, start, end } => {
                if let (Some(a), Some(b)) = (pos(*start), pos(*end)) {
                    geom.lines.push(SketchLineGeom {
                        id: *id,
                        start: a,
                        end: b,
                        start_id: *start,
                        end_id: *end,
                        construction,
                    });
                }
            }
            rk_cad::SketchEntity::Circle { id, center, radius } => {
                if let Some(c) = pos(*center) {
                    geom.circles.push(SketchCircleGeom {
                        id: *id,
                        center: c,
                        radius: *radius,
                        construction,
                    });
                }
            }
            rk_cad::SketchEntity::Arc {
                id,
                center,
                start,
                end,
                radius,
            } => {
                if let (Some(c), Some(a), Some(b)) = (pos(*center), pos(*start), pos(*end)) {
                    geom.arcs.push(SketchArcGeom {
                        id: *id,
                        center: c,
                        radius: *radius,
                        start_angle: angle(c, a),
                        end_angle: angle(c, b),
                        construction,
                    });
                }
            }
            // Ellipses and splines have no UI yet
            _ => {}
        }
    }

    geom.constraints = sketch
        .constraints_iter()
        .map(|c| SketchConstraintInfo {
            id: c.id(),
            constraint: c.clone(),
            label: c.type_name(),
            entities: c.referenced_entities(),
            value: c.value(),
        })
        .collect();
    // Constraints live in a hash map; the list would otherwise reshuffle on
    // every refresh
    geom.constraints
        .sort_by(|a, b| a.label.cmp(b.label).then(a.id.cmp(&b.id)));

    Ok(geom)
}

#[derive(serde::Serialize)]
struct MeshPayload {
    vertices: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    color: [f32; 4],
}

#[tauri::command]
fn get_part_mesh(state: State<'_, EngineState>, part_id: Uuid) -> Result<MeshPayload, String> {
    let engine = state.engine.lock();
    let part = engine
        .part(part_id)
        .ok_or_else(|| format!("unknown part: {part_id}"))?;
    Ok(MeshPayload {
        vertices: part.vertices.clone(),
        normals: part.normals.clone(),
        indices: part.indices.clone(),
        color: part.color,
    })
}

#[tauri::command]
fn get_body_mesh(state: State<'_, EngineState>, body_id: Uuid) -> Result<MeshPayload, String> {
    let mut engine = state.engine.lock();
    let mesh = engine.body_mesh(body_id).map_err(|e| e.to_string())?;
    Ok(MeshPayload {
        vertices: mesh.vertices.clone(),
        normals: mesh.normals.clone(),
        indices: mesh.indices.clone(),
        color: BODY_COLOR,
    })
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let kernel: Arc<dyn rk_cad::CadKernel> = Arc::from(rk_cad::default_kernel());
    tracing::info!("rk-desktop starting (kernel: {})", kernel.name());
    let engine: SharedEngine = Arc::new(parking_lot::Mutex::new(Engine::new(kernel)));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(EngineState { engine })
        .invoke_handler(tauri::generate_handler![
            engine_apply,
            engine_apply_interactive,
            engine_end_interaction,
            scene_snapshot,
            sketch_geometry,
            get_part_mesh,
            get_body_mesh
        ])
        .run(tauri::generate_context!())
        .expect("error while running rk-desktop");
}
