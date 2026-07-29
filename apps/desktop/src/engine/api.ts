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
 *
 * `atomic` collapses the batch into a single undo step, for command pairs
 * that are one action to the user (adding a constraint and re-solving).
 */
export type RunCommands = (
  commands: Command[],
  atomic?: boolean,
) => Promise<EngineEvent[]>;

/** Column-major 4x4 matrix, 16 elements (glam serde layout) */
export type Mat4 = number[];

export type Rgba = [number, number, number, number];

export interface InertiaMatrix {
  ixx: number;
  ixy: number;
  ixz: number;
  iyy: number;
  iyz: number;
  izz: number;
}

export interface PartInfo {
  id: string;
  name: string;
  color: Rgba;
  has_mesh: boolean;
  origin_transform: Mat4;
  /** Link world transform; `render = parent_transform × origin_transform` */
  parent_transform: Mat4;
  mass: number;
  inertia: InertiaMatrix;
  /** Mesh bounds in part space, for fitting a collision shape to the part */
  bbox_min: [number, number, number];
  bbox_max: [number, number, number];
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

/** Externally tagged rk_core::GeometryType: `{"Box": {"size": [...]}}` */
export type GeometryType =
  | { Mesh: { path: string | null; scale?: [number, number, number] | null } }
  | { Box: { size: [number, number, number] } }
  | { Cylinder: { radius: number; length: number } }
  | { Sphere: { radius: number } }
  | { Capsule: { radius: number; length: number } };

export interface CollisionInfo {
  /** Position in the link's collision list — commands address it by index */
  index: number;
  name: string | null;
  origin: Pose;
  geometry: GeometryType;
  /** link world × origin, ready to render */
  transform: Mat4;
}

export interface LinkInfo {
  id: string;
  name: string;
  part_id: string | null;
  world_transform: Mat4;
  collisions: CollisionInfo[];
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

/**
 * Externally tagged rk_cad::SketchConstraint: `{"Horizontal": {...}}`.
 * Constraints are keyed by ID, so re-adding one with the same ID and a new
 * value edits it in place.
 */
export type SketchConstraint =
  | { Coincident: { id: string; point1: string; point2: string } }
  | { Horizontal: { id: string; line: string } }
  | { Vertical: { id: string; line: string } }
  | { Parallel: { id: string; line1: string; line2: string } }
  | { Perpendicular: { id: string; line1: string; line2: string } }
  | { Tangent: { id: string; curve1: string; curve2: string } }
  | { EqualLength: { id: string; line1: string; line2: string } }
  | { EqualRadius: { id: string; circle1: string; circle2: string } }
  | { PointOnCurve: { id: string; point: string; curve: string } }
  | { Midpoint: { id: string; point: string; line: string } }
  | { Symmetric: { id: string; entity1: string; entity2: string; axis: string } }
  | { Fixed: { id: string; point: string; x: number; y: number } }
  | {
      Distance: { id: string; entity1: string; entity2: string; value: number };
    }
  | {
      HorizontalDistance: {
        id: string;
        point1: string;
        point2: string;
        value: number;
      };
    }
  | {
      VerticalDistance: {
        id: string;
        point1: string;
        point2: string;
        value: number;
      };
    }
  | { Angle: { id: string; line1: string; line2: string; value: number } }
  | { Radius: { id: string; circle: string; value: number } }
  | { Diameter: { id: string; circle: string; value: number } }
  | { Length: { id: string; line: string; value: number } };

type VariantOf<T> = T extends object ? keyof T : never;
/** Serde variant name of a constraint, e.g. `"EqualLength"` */
export type ConstraintKind = VariantOf<SketchConstraint>;

export interface SketchConstraintInfo {
  id: string;
  /** Send this back with a new value to edit the constraint in place */
  constraint: SketchConstraint;
  /** Display name from the engine, e.g. "Equal Length" */
  label: string;
  /** Entities the constraint references, for highlighting */
  entities: string[];
  /** `null` for geometric constraints */
  value: number | null;
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
  constraints: SketchConstraintInfo[];
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
