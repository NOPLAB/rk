//! rk-desktop: Tauri 2 shell around the headless RK engine.
//!
//! The webview (React + Three.js) is a pure client of `rk-engine`: it sends
//! `Command` batches through `engine_apply` and pulls bulk data (meshes,
//! scene snapshots) by ID — the same protocol shape the MCP server uses.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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

#[derive(serde::Serialize)]
struct PartInfo {
    id: Uuid,
    name: String,
    color: [f32; 4],
    has_mesh: bool,
    origin_transform: glam::Mat4,
}

#[derive(serde::Serialize)]
struct HistoryInfo {
    can_undo: bool,
    can_redo: bool,
    undo_description: Option<String>,
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
    history: HistoryInfo,
}

/// Lightweight UI snapshot: part metadata, render transforms, body IDs and
/// undo state. Mesh data is pulled separately per ID.
#[tauri::command]
fn scene_snapshot(state: State<'_, EngineState>) -> SceneSnapshot {
    let engine = state.engine.lock();
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
            })
            .collect(),
        transforms: engine.part_render_transforms(),
        body_ids: engine.body_ids(),
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
            scene_snapshot,
            get_part_mesh,
            get_body_mesh
        ])
        .run(tauri::generate_context!())
        .expect("error while running rk-desktop");
}
