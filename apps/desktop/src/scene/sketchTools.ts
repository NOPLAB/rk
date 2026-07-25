// Drawing-tool state machine for sketch mode.
//
// Nothing reaches the engine until a shape is complete: the first click only
// records an anchor. Cancelling therefore leaves no orphan points behind, and
// each finished shape is one `add_sketch_entities` command — one undo step.
//
// The line tool reuses point IDs between segments (and closes onto the first
// point of the chain), which is what lets the engine find closed profiles.

import type { Command, Vec2 } from "../engine/api";
import {
  addSketchEntities,
  sketchCircle,
  sketchLine,
  sketchPoint,
  type SketchEntity,
} from "../engine/commands";
import { newUuid } from "../engine/interaction";
import { rectPoints, type SketchPreview } from "./sketchLayer";
import type { SketchHit } from "./viewport";

export type SketchTool = "select" | "line" | "rect" | "circle";

/** Shapes smaller than this (in meters) are treated as a stray double click */
const MIN_SIZE = 1e-5;

interface Anchor {
  pos: Vec2;
  /** Set when the anchor is an existing engine point that can be referenced */
  pointId: string | null;
}

export class SketchDrawing {
  private tool: SketchTool = "select";
  private sketchId: string | null = null;
  private anchor: Anchor | null = null;
  /** First anchor of the current line chain; clicking it closes the loop */
  private chainOrigin: Anchor | null = null;

  /** True while a shape is half-drawn (Escape has something to cancel) */
  get busy(): boolean {
    return this.anchor !== null;
  }

  get activeTool(): SketchTool {
    return this.tool;
  }

  setTool(tool: SketchTool) {
    this.tool = tool;
    this.cancel();
  }

  setSketch(sketchId: string | null) {
    this.sketchId = sketchId;
    this.cancel();
  }

  cancel() {
    this.anchor = null;
    this.chainOrigin = null;
  }

  /** Feed a click; returns the commands to apply (empty while mid-shape) */
  click(hit: SketchHit): Command[] {
    const sketchId = this.sketchId;
    if (!sketchId || this.tool === "select") return [];

    const here: Anchor = { pos: hit.position, pointId: hit.pointId };
    if (!this.anchor) {
      this.anchor = here;
      if (this.tool === "line") this.chainOrigin = here;
      return [];
    }
    const from = this.anchor;
    if (dist(from.pos, here.pos) < MIN_SIZE) return [];

    const entities: SketchEntity[] = [];
    /** Reference an existing point, or mint one and add it to the batch */
    const pointRef = (a: Anchor): string => {
      if (a.pointId) return a.pointId;
      const id = newUuid();
      entities.push(sketchPoint(id, a.pos));
      a.pointId = id;
      return id;
    };

    switch (this.tool) {
      case "line": {
        const startId = pointRef(from);
        const endId = pointRef(here);
        entities.push(sketchLine(newUuid(), startId, endId));
        const closed = this.chainOrigin?.pointId === endId;
        // Keep drawing from the point just placed, unless the loop closed
        this.anchor = closed ? null : here;
        if (closed) this.chainOrigin = null;
        break;
      }
      case "rect": {
        const corners = rectPoints(from.pos, here.pos);
        const ids = corners.map((c) => {
          const id = newUuid();
          entities.push(sketchPoint(id, c));
          return id;
        });
        for (let i = 0; i < 4; i++) {
          entities.push(sketchLine(newUuid(), ids[i], ids[(i + 1) % 4]));
        }
        this.anchor = null;
        break;
      }
      case "circle": {
        const centerId = pointRef(from);
        entities.push(
          sketchCircle(newUuid(), centerId, dist(from.pos, here.pos)),
        );
        this.anchor = null;
        break;
      }
    }
    return [addSketchEntities(sketchId, entities)];
  }

  /** Rubber-band shape for the pointer's current position */
  preview(hit: SketchHit): SketchPreview | null {
    const from = this.anchor;
    if (!from) return null;
    switch (this.tool) {
      case "line":
        return { kind: "line", from: from.pos, to: hit.position };
      case "rect":
        return { kind: "rect", from: from.pos, to: hit.position };
      case "circle":
        return {
          kind: "circle",
          center: from.pos,
          radius: dist(from.pos, hit.position),
        };
      default:
        return null;
    }
  }
}

function dist(a: Vec2, b: Vec2): number {
  return Math.hypot(b[0] - a[0], b[1] - a[1]);
}
