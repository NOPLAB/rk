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
}

#[derive(serde::Serialize)]
struct HistoryInfo {
    can_undo: bool,
    can_redo: bool,
    undo_description: Option<String>,
}

#[derive(serde::Serialize)]
struct LinkInfo {
    id: Uuid,
    name: String,
    part_id: Option<Uuid>,
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
            })
            .collect(),
        transforms: engine.part_render_transforms(),
        body_ids: engine.body_ids(),
        links,
        joints,
        history: HistoryInfo {
            can_undo: engine.can_undo(),
            can_redo: engine.can_redo(),
            undo_description: engine.undo_description().map(str::to_owned),
        },
    }
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
            get_part_mesh,
            get_body_mesh
        ])
        .run(tauri::generate_context!())
        .expect("error while running rk-desktop");
}
