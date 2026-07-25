// Typed wrappers around the Tauri IPC commands exposed by rk-desktop.
//
// The engine protocol mirrors the MCP server: commands go in as tagged
// JSON, events come back tagged the same way, and bulk data (meshes) is
// pulled separately by ID.

import { invoke } from "@tauri-apps/api/core";

/** Engine command: `{"type": "create_primitive", ...}` (see commands_reference.md) */
export type Command = { type: string } & Record<string, unknown>;

/** Engine event, tagged like commands: `{"type": "part_added", ...}` */
export type EngineEvent = { type: string } & Record<string, unknown>;

export interface ApplyOutcome {
  applied: number;
  events: EngineEvent[];
  error: { index: number; message: string } | null;
}

/** Column-major 4x4 matrix, 16 elements (glam serde layout) */
export type Mat4 = number[];

export type Rgba = [number, number, number, number];

export interface PartInfo {
  id: string;
  name: string;
  color: Rgba;
  has_mesh: boolean;
  origin_transform: Mat4;
}

export interface SceneSnapshot {
  project_name: string;
  doc_path: string | null;
  modified: boolean;
  revision: number;
  parts: PartInfo[];
  transforms: [string, Mat4][];
  body_ids: string[];
  history: {
    can_undo: boolean;
    can_redo: boolean;
    undo_description: string | null;
  };
}

export interface MeshPayload {
  vertices: [number, number, number][];
  normals: [number, number, number][];
  indices: number[];
  color: Rgba;
}

export function applyCommands(commands: Command[]): Promise<ApplyOutcome> {
  return invoke("engine_apply", { commands });
}

export function sceneSnapshot(): Promise<SceneSnapshot> {
  return invoke("scene_snapshot");
}

export function getPartMesh(partId: string): Promise<MeshPayload> {
  return invoke("get_part_mesh", { partId });
}

export function getBodyMesh(bodyId: string): Promise<MeshPayload> {
  return invoke("get_body_mesh", { bodyId });
}
