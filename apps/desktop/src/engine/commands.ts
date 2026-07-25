// Builders for the engine commands the desktop UI uses today.
// The full catalog lives in crates/rk-mcp/src/commands_reference.md.

import type { Command, Rgba } from "./api";

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

export const undo = (): Command => ({ type: "undo" });
export const redo = (): Command => ({ type: "redo" });
