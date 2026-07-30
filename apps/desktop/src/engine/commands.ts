// Builders for the engine commands the desktop UI uses today.
// The full catalog lives in crates/rk-mcp/src/commands_reference.md.

import type {
  Command,
  GeometryType,
  InertiaMatrix,
  JointLimits,
  JointType,
  Pose,
  Rgba,
  SketchConstraint,
  SketchPlane,
  Vec2,
  Vec3,
} from "./api";

/** Serde PascalCase variants of rk_core::StlUnit */
export type StlUnit = "Meters" | "Millimeters" | "Centimeters" | "Inches";

export const newDocument = (): Command => ({ type: "new_document" });

export const loadDocument = (path: string): Command => ({
  type: "load_document",
  path,
});

/** `path: null` saves to the current file */
export const saveDocument = (path: string | null): Command => ({
  type: "save_document",
  path,
});

export const createBox = (
  size: [number, number, number],
  name: string | null = null,
): Command => ({
  type: "create_primitive",
  id: null,
  primitive: { shape: "box", size },
  name,
});

export const createCylinder = (
  radius: number,
  height: number,
  name: string | null = null,
): Command => ({
  type: "create_primitive",
  id: null,
  primitive: { shape: "cylinder", radius, height },
  name,
});

export const createSphere = (
  radius: number,
  name: string | null = null,
): Command => ({
  type: "create_primitive",
  id: null,
  primitive: { shape: "sphere", radius },
  name,
});

export const deletePart = (partId: string): Command => ({
  type: "delete_part",
  part_id: partId,
});

export const renamePart = (partId: string, name: string): Command => ({
  type: "rename_part",
  part_id: partId,
  name,
});

export const setPartColor = (partId: string, color: Rgba): Command => ({
  type: "set_part_color",
  part_id: partId,
  color,
});

/** `transform` is a column-major 16-element Mat4 */
export const setPartTransform = (
  partId: string,
  transform: number[],
): Command => ({
  type: "set_part_transform",
  part_id: partId,
  transform,
});

export const setPartMass = (partId: string, mass: number): Command => ({
  type: "set_part_mass",
  part_id: partId,
  mass,
});

export const setPartInertia = (
  partId: string,
  inertia: InertiaMatrix,
): Command => ({
  type: "set_part_inertia",
  part_id: partId,
  inertia,
});

export const importMesh = (path: string, unit: StlUnit): Command => ({
  type: "import_mesh",
  path,
  unit,
});

/** Replaces the current document with the imported robot */
export const importUrdf = (path: string, stlUnit: StlUnit): Command => ({
  type: "import_urdf",
  path,
  stl_unit: stlUnit,
});

export const exportUrdf = (path: string, robotName: string): Command => ({
  type: "export_urdf",
  path,
  robot_name: robotName,
});

export const connectParts = (
  parentPart: string,
  childPart: string,
): Command => ({
  type: "connect_parts",
  parent_part: parentPart,
  child_part: childPart,
});

export const disconnectPart = (childPart: string): Command => ({
  type: "disconnect_part",
  child_part: childPart,
});

export const setJointPosition = (
  jointId: string,
  position: number,
): Command => ({
  type: "set_joint_position",
  joint_id: jointId,
  position,
});

export const resetJointPosition = (jointId: string): Command => ({
  type: "reset_joint_position",
  joint_id: jointId,
});

export const resetAllJointPositions = (): Command => ({
  type: "reset_all_joint_positions",
});

export const setJointType = (
  jointId: string,
  jointType: JointType,
): Command => ({
  type: "set_joint_type",
  joint_id: jointId,
  joint_type: jointType,
});

export const setJointAxis = (
  jointId: string,
  axis: [number, number, number],
): Command => ({
  type: "set_joint_axis",
  joint_id: jointId,
  axis,
});

export const setJointLimits = (
  jointId: string,
  limits: JointLimits | null,
): Command => ({
  type: "set_joint_limits",
  joint_id: jointId,
  limits,
});

// ---- collisions ---------------------------------------------------------

export const identityPose = (): Pose => ({ xyz: [0, 0, 0], rpy: [0, 0, 0] });

export const addCollision = (
  linkId: string,
  geometry: GeometryType,
  origin: Pose = identityPose(),
): Command => ({
  type: "add_collision",
  link_id: linkId,
  geometry,
  origin,
});

export const removeCollision = (linkId: string, index: number): Command => ({
  type: "remove_collision",
  link_id: linkId,
  index,
});

export const setCollisionOrigin = (
  linkId: string,
  index: number,
  origin: Pose,
): Command => ({
  type: "set_collision_origin",
  link_id: linkId,
  index,
  origin,
});

export const setCollisionGeometry = (
  linkId: string,
  index: number,
  geometry: GeometryType,
): Command => ({
  type: "set_collision_geometry",
  link_id: linkId,
  index,
  geometry,
});

// ---- sketches -----------------------------------------------------------

/** Externally tagged rk_cad::SketchEntity: `{"Line": {...}}` */
export type SketchEntity =
  | { Point: { id: string; position: Vec2 } }
  | { Line: { id: string; start: string; end: string } }
  | { Circle: { id: string; center: string; radius: number } }
  | {
      Arc: {
        id: string;
        center: string;
        start: string;
        end: string;
        radius: number;
      };
    }
  | {
      Ellipse: {
        id: string;
        center: string;
        major_radius: number;
        minor_radius: number;
        rotation: number;
      };
    }
  | { Spline: { id: string; control_points: string[]; closed: boolean } };

export const sketchPoint = (id: string, position: Vec2): SketchEntity => ({
  Point: { id, position },
});

/** `start`/`end` are point entity IDs */
export const sketchLine = (
  id: string,
  start: string,
  end: string,
): SketchEntity => ({ Line: { id, start, end } });

export const sketchCircle = (
  id: string,
  center: string,
  radius: number,
): SketchEntity => ({ Circle: { id, center, radius } });

/** Sweeps counter-clockwise from `start` to `end`; swap them to go the other way */
export const sketchArc = (
  id: string,
  center: string,
  start: string,
  end: string,
  radius: number,
): SketchEntity => ({ Arc: { id, center, start, end, radius } });

export const sketchEllipse = (
  id: string,
  center: string,
  majorRadius: number,
  minorRadius: number,
  rotation: number,
): SketchEntity => ({
  Ellipse: {
    id,
    center,
    major_radius: majorRadius,
    minor_radius: minorRadius,
    rotation,
  },
});

export const sketchSpline = (
  id: string,
  controlPoints: string[],
  closed: boolean,
): SketchEntity => ({
  Spline: { id, control_points: controlPoints, closed },
});

/** The three standard planes, optionally offset along their normal */
export const standardPlane = (
  which: "XY" | "XZ" | "YZ",
  offset = 0,
): SketchPlane => {
  const axes: Record<string, [Vec3, Vec3, Vec3]> = {
    XY: [
      [0, 0, 1],
      [1, 0, 0],
      [0, 1, 0],
    ],
    XZ: [
      [0, 1, 0],
      [1, 0, 0],
      [0, 0, 1],
    ],
    YZ: [
      [1, 0, 0],
      [0, 1, 0],
      [0, 0, 1],
    ],
  };
  const [normal, x_axis, y_axis] = axes[which];
  return {
    origin: [normal[0] * offset, normal[1] * offset, normal[2] * offset],
    normal,
    x_axis,
    y_axis,
  };
};

export const createSketch = (
  plane: SketchPlane,
  name: string | null = null,
  id: string | null = null,
): Command => ({ type: "create_sketch", id, name, plane });

export const deleteSketch = (sketchId: string): Command => ({
  type: "delete_sketch",
  sketch_id: sketchId,
});

export const renameSketch = (sketchId: string, name: string): Command => ({
  type: "rename_sketch",
  sketch_id: sketchId,
  name,
});

/** One command = one undo step, so a rectangle's 4 points + 4 lines go together */
export const addSketchEntities = (
  sketchId: string,
  entities: SketchEntity[],
): Command => ({
  type: "add_sketch_entities",
  sketch_id: sketchId,
  entities,
});

/** Replace an entity that already exists (moving a point, resizing a circle) */
export const updateSketchEntity = (
  sketchId: string,
  entity: SketchEntity,
): Command => ({
  type: "update_sketch_entity",
  sketch_id: sketchId,
  entity,
});

export const deleteSketchEntities = (
  sketchId: string,
  entityIds: string[],
): Command => ({
  type: "delete_sketch_entities",
  sketch_id: sketchId,
  entity_ids: entityIds,
});

/**
 * Constraints are keyed by ID, so re-adding one with the same ID replaces it —
 * that is how a dimension's value is edited.
 */
export const addSketchConstraint = (
  sketchId: string,
  constraint: SketchConstraint,
): Command => ({
  type: "add_sketch_constraint",
  sketch_id: sketchId,
  constraint,
});

export const deleteSketchConstraint = (
  sketchId: string,
  constraintId: string,
): Command => ({
  type: "delete_sketch_constraint",
  sketch_id: sketchId,
  constraint_id: constraintId,
});

export const solveSketch = (sketchId: string): Command => ({
  type: "solve_sketch",
  sketch_id: sketchId,
});

/** Construction geometry guides the sketch but encloses no region */
export const setSketchConstruction = (
  sketchId: string,
  entityIds: string[],
  construction: boolean,
): Command => ({
  type: "set_sketch_construction",
  sketch_id: sketchId,
  entity_ids: entityIds,
  construction,
});

// ---- features -----------------------------------------------------------

/** Relative to the sketch plane normal */
export type ExtrudeDirection = "Positive" | "Negative" | "Symmetric";
export type BooleanOp = "New" | "Join" | "Cut" | "Intersect";

/** `profiles` are region IDs; an empty list takes every region in the sketch */
export const addExtrude = (
  sketchId: string,
  distance: number,
  direction: ExtrudeDirection,
  booleanOp: BooleanOp,
  targetBody: string | null,
  name: string | null = null,
  profiles: string[] = [],
): Command => ({
  type: "add_extrude",
  id: null,
  name,
  sketch_id: sketchId,
  profiles,
  distance,
  direction,
  boolean_op: booleanOp,
  target_body: targetBody,
});

export const addRevolve = (
  sketchId: string,
  axisOrigin: Vec3,
  axisDirection: Vec3,
  angle: number,
  booleanOp: BooleanOp,
  targetBody: string | null,
  name: string | null = null,
  profiles: string[] = [],
): Command => ({
  type: "add_revolve",
  id: null,
  name,
  sketch_id: sketchId,
  profiles,
  axis_origin: axisOrigin,
  axis_direction: axisDirection,
  angle,
  boolean_op: booleanOp,
  target_body: targetBody,
});

export const deleteFeature = (featureId: string): Command => ({
  type: "delete_feature",
  feature_id: featureId,
});

export const renameFeature = (featureId: string, name: string): Command => ({
  type: "rename_feature",
  feature_id: featureId,
  name,
});

export const setFeatureSuppressed = (
  featureId: string,
  suppressed: boolean,
): Command => ({
  type: "set_feature_suppressed",
  feature_id: featureId,
  suppressed,
});

// ---- Feature groups (browser presentation only) --------------------------

export const groupFeatures = (
  featureIds: string[],
  name?: string,
  id?: string,
): Command => ({
  type: "group_features",
  id: id ?? null,
  name: name ?? null,
  feature_ids: featureIds,
});

export const ungroupFeatures = (groupId: string): Command => ({
  type: "ungroup_features",
  group_id: groupId,
});

export const renameFeatureGroup = (groupId: string, name: string): Command => ({
  type: "rename_feature_group",
  group_id: groupId,
  name,
});

export const setFeatureGroupCollapsed = (
  groupId: string,
  collapsed: boolean,
): Command => ({
  type: "set_feature_group_collapsed",
  group_id: groupId,
  collapsed,
});

/** `featureId: null` rolls forward to the end of the history */
export const rollbackTo = (featureId: string | null): Command => ({
  type: "rollback_to",
  feature_id: featureId,
});

export const undo = (): Command => ({ type: "undo" });
export const redo = (): Command => ({ type: "redo" });
