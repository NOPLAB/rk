// Builders for the engine commands the desktop UI uses today.
// The full catalog lives in crates/rk-mcp/src/commands_reference.md.

import type { Command, JointLimits, JointType, Rgba } from "./api";

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

export const undo = (): Command => ({ type: "undo" });
export const redo = (): Command => ({ type: "redo" });
