// Creating a sketch, from wherever the user asked for one.
//
// Three places start a sketch — the ribbon's pick-a-plane flow, the origin
// planes in the browser tree, and a double click on an existing sketch — and
// they all end up here so the naming and the "jump straight into editing"
// behaviour stay identical.

import type { SketchPlane } from "../engine/api";
import { createSketch } from "../engine/commands";
import { originPlane, type OriginPlane } from "../scene/planePicker";
import type { AppApi } from "./appApi";

/**
 * Create a sketch on `plane` and start editing it. `label` says where the
 * plane came from, so the browser tree can tell two sketches apart.
 */
export async function createSketchOn(
  api: AppApi,
  plane: SketchPlane,
  label: string,
): Promise<void> {
  const count = api.snapshot?.sketches.length ?? 0;
  const events = await api.run([
    createSketch(plane, `Sketch ${count + 1} (${label})`),
  ]);
  const added = events.find((e) => e.type === "sketch_added");
  if (added) api.activateSketch(added.sketch_id as string);
}

/** Start a sketch on one of the three origin planes */
export function createSketchOnOrigin(api: AppApi, which: OriginPlane) {
  return createSketchOn(api, originPlane(which), which);
}
