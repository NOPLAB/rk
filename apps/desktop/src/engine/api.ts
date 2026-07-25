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

/**
 * Apply a batch and sync the scene, returning the events it produced (empty
 * on failure). Panels read the events to learn engine-minted IDs.
 */
export type RunCommands = (commands: Command[]) => Promise<EngineEvent[]>;

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

export type Vec3 = [number, number, number];
/** A point in sketch coordinates */
export type Vec2 = [number, number];

export interface SketchPlane {
  origin: Vec3;
  normal: Vec3;
  x_axis: Vec3;
  y_axis: Vec3;
}

export interface SketchInfo {
  id: string;
  name: string;
  plane: SketchPlane;
  /** Sketch space → world */
  transform: Mat4;
  entity_count: number;
  constraint_count: number;
  is_solved: boolean;
  dof: number;
  /** Closed profiles found; extrude needs at least one */
  profile_count: number;
}

/** `Feature::type_name()` of rk-cad */
export type FeatureKind =
  | "Extrude"
  | "Revolve"
  | "Boolean"
  | "Fillet"
  | "Chamfer"
  | "Shell"
  | "Sweep"
  | "Loft";

export interface FeatureInfo {
  id: string;
  name: string;
  kind: FeatureKind;
  suppressed: boolean;
  sketch_id: string | null;
  created_bodies: string[];
}

/** Sketch entities with point references resolved to coordinates */
export interface SketchGeometry {
  points: {
    id: string;
    position: Vec2;
    construction: boolean;
  }[];
  lines: {
    id: string;
    start: Vec2;
    end: Vec2;
    start_id: string;
    end_id: string;
    construction: boolean;
  }[];
  circles: {
    id: string;
    center: Vec2;
    radius: number;
    construction: boolean;
  }[];
  arcs: {
    id: string;
    center: Vec2;
    radius: number;
    start_angle: number;
    end_angle: number;
    construction: boolean;
  }[];
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
  sketches: SketchInfo[];
  features: FeatureInfo[];
  /** Features from this index on are rolled back; `null` = all active */
  rollback_position: number | null;
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

export function sketchGeometry(sketchId: string): Promise<SketchGeometry> {
  return invoke("sketch_geometry", { sketchId });
}

export function getPartMesh(partId: string): Promise<MeshPayload> {
  return invoke("get_part_mesh", { partId });
}

export function getBodyMesh(bodyId: string): Promise<MeshPayload> {
  return invoke("get_body_mesh", { bodyId });
}
