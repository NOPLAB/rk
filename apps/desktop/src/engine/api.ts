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
  /** Link world transform; `render = parent_transform × origin_transform` */
  parent_transform: Mat4;
}

/** Serde PascalCase variants of rk_core::JointType */
export type JointType =
  | "Fixed"
  | "Revolute"
  | "Continuous"
  | "Prismatic"
  | "Floating"
  | "Planar";

export interface JointLimits {
  lower: number;
  upper: number;
  effort: number;
  velocity: number;
}

/** rk_core::Pose: translation + roll/pitch/yaw (radians) */
export interface Pose {
  xyz: [number, number, number];
  rpy: [number, number, number];
}

export interface LinkInfo {
  id: string;
  name: string;
  part_id: string | null;
}

export interface JointInfo {
  id: string;
  name: string;
  joint_type: JointType;
  parent_link: string;
  child_link: string;
  parent_part: string | null;
  child_part: string | null;
  origin: Pose;
  axis: [number, number, number];
  limits: JointLimits | null;
  position: number;
}

export interface SceneSnapshot {
  project_name: string;
  doc_path: string | null;
  modified: boolean;
  revision: number;
  parts: PartInfo[];
  transforms: [string, Mat4][];
  body_ids: string[];
  links: LinkInfo[];
  joints: JointInfo[];
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

/**
 * Apply one command inside an interaction session (gizmo drag): the whole
 * session collapses into a single undo step.
 */
export function applyInteractive(
  session: string,
  command: Command,
): Promise<ApplyOutcome> {
  return invoke("engine_apply_interactive", { session, command });
}

/** Close a session; `cancel` reverts the document to its pre-drag state */
export function endInteraction(
  session: string,
  cancel: boolean,
): Promise<ApplyOutcome> {
  return invoke("engine_end_interaction", { session, cancel });
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
