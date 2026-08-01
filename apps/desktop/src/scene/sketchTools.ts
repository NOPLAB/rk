// Drawing-tool state machine for sketch mode.
//
// Nothing reaches the engine until a shape is complete: clicks accumulate as
// anchors and only the finished shape becomes an `add_sketch_entities`
// command. Cancelling therefore leaves no orphan points behind, and each
// shape is one undo step.
//
// Tools that continue from where they left off (line, spline) reuse point IDs
// between segments and close onto the chain's first point — that shared ID is
// what lets the engine find a closed region.

import type { Command, SketchGeometry, Vec2 } from "../engine/api";
import {
  addSketchEntities,
  setSketchConstruction,
  sketchArc,
  sketchCircle,
  sketchEllipse,
  sketchLine,
  sketchPoint,
  sketchSpline,
  type SketchEntity,
} from "../engine/commands";
import { newUuid } from "../engine/interaction";
import {
  editCommands,
  EDIT_TOOLS,
  type EditResult,
  type EditTool,
} from "./sketchEdits";
import {
  add,
  angleOf,
  arcPolyline,
  arcThroughPoints,
  centerRectCorners,
  circlePolyline,
  circumcircle,
  circumscribedVertices,
  dist,
  edgePolygonVertices,
  ellipsePolyline,
  mid,
  norm,
  perp,
  polar,
  polygonVertices,
  rectCorners,
  scale,
  splinePolyline,
  sub,
  sweepBetween,
  threePointRectCorners,
  TAU,
} from "./sketchGeom";
import type { SketchHit } from "./viewport";

/** What a click produced: commands to apply, or a reason nothing happened */
export type ClickResult = EditResult;

export type CreateTool =
  | "point"
  | "line"
  | "rect"
  | "rectCenter"
  | "rect3"
  | "circle"
  | "circle2"
  | "circle3"
  | "arc3"
  | "arcCenter"
  | "polygon"
  | "polygonCirc"
  | "polygonEdge"
  | "ellipse"
  | "slot"
  | "slotOverall"
  | "spline";

export type SketchTool = "select" | CreateTool | EditTool;

/** Knobs the ribbon exposes for the tools that need a number up front */
export interface ToolOptions {
  /** Polygon side count */
  sides: number;
  /** Sketch fillet radius, metres */
  filletRadius: number;
  /** Offset distance, metres */
  offsetDistance: number;
  /** Copies in a pattern, including the original */
  patternCount: number;
  /** Step between rectangular-pattern copies, metres */
  patternSpacing: number;
  /** Angle a circular pattern spans, radians */
  patternAngle: number;
  /** Draw as construction geometry */
  construction: boolean;
}

export const DEFAULT_TOOL_OPTIONS: ToolOptions = {
  sides: 6,
  filletRadius: 0.005,
  offsetDistance: 0.005,
  patternCount: 4,
  patternSpacing: 0.02,
  patternAngle: TAU,
  construction: false,
};

/** Shapes smaller than this are treated as a stray double click */
const MIN_SIZE = 1e-5;

/** A click that neither built anything nor had anything to complain about */
const quiet: ClickResult = { commands: [], problem: null };

interface Anchor {
  pos: Vec2;
  /** Set when the anchor landed on an existing engine point, so it can be reused */
  pointId: string | null;
}

/** Rubber-band geometry for the shape being drawn */
export interface SketchPreview {
  strokes: { pts: Vec2[]; closed: boolean }[];
  /** Anchors already placed, drawn as markers */
  marks?: Vec2[];
}

/** How many clicks a tool needs before it commits */
const STAGES: Record<CreateTool, number> = {
  point: 1,
  line: 2,
  rect: 2,
  rectCenter: 2,
  rect3: 3,
  circle: 2,
  circle2: 2,
  circle3: 3,
  arc3: 3,
  arcCenter: 3,
  polygon: 2,
  polygonCirc: 2,
  polygonEdge: 2,
  ellipse: 3,
  slot: 3,
  slotOverall: 3,
  spline: 0, // open-ended: finished with Enter or by closing the loop
};

const CREATE_TOOLS = new Set<string>(Object.keys(STAGES));

export function isCreateTool(tool: SketchTool): tool is CreateTool {
  return CREATE_TOOLS.has(tool);
}

export function isEditTool(tool: SketchTool): tool is EditTool {
  return (EDIT_TOOLS as readonly string[]).includes(tool);
}

const EMPTY_GEOMETRY: SketchGeometry = {
  points: [],
  lines: [],
  circles: [],
  arcs: [],
  ellipses: [],
  splines: [],
  regions: [],
  constraints: [],
};

export class SketchDrawing {
  private tool: SketchTool = "select";
  private sketchId: string | null = null;
  private geometry: SketchGeometry = EMPTY_GEOMETRY;
  private anchors: Anchor[] = [];
  /** First anchor of the current chain; clicking it closes the loop */
  private chainOrigin: Anchor | null = null;
  private options: ToolOptions = { ...DEFAULT_TOOL_OPTIONS };

  /** True while a shape is half-drawn (Escape has something to cancel) */
  get busy(): boolean {
    return this.anchors.length > 0;
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

  /** The tools that reshape existing curves need to see them */
  setGeometry(geometry: SketchGeometry) {
    this.geometry = geometry;
  }

  setOptions(options: ToolOptions) {
    this.options = options;
  }

  cancel() {
    this.anchors = [];
    this.chainOrigin = null;
  }

  /**
   * Finish an open-ended shape (Enter, or a double click). Splines are the
   * only tool that needs it; everything else commits on its last click.
   */
  finish(): Command[] {
    if (this.tool !== "spline" || this.anchors.length < 2) {
      this.cancel();
      return [];
    }
    const commands = this.commit(this.anchors, false);
    this.cancel();
    return commands;
  }

  /**
   * Feed a click. `commands` is empty while a shape is still being drawn;
   * `problem` explains why an edit tool declined, for the status bar.
   */
  click(hit: SketchHit, selection: string[] = []): ClickResult {
    const sketchId = this.sketchId;
    if (!sketchId || this.tool === "select") return quiet;

    if (isEditTool(this.tool)) {
      return editCommands(this.tool, {
        sketchId,
        geometry: this.geometry,
        hit,
        selection,
        options: this.options,
      });
    }
    if (!isCreateTool(this.tool)) return quiet;

    const here: Anchor = { pos: hit.position, pointId: hit.pointId };
    const previous = this.anchors[this.anchors.length - 1];
    // Ignore a double click that lands where the last one did
    if (previous && dist(previous.pos, here.pos) < MIN_SIZE) return quiet;

    if (this.anchors.length === 0) this.chainOrigin = here;
    this.anchors.push(here);

    if (this.tool === "spline") {
      // A click back on the first knot closes the curve
      const closed =
        this.anchors.length > 2 && here.pointId != null &&
        here.pointId === this.chainOrigin?.pointId;
      if (!closed) return quiet;
      const commands = this.commit(this.anchors.slice(0, -1), true);
      this.cancel();
      return { commands, problem: null };
    }

    const needed = STAGES[this.tool];
    if (this.anchors.length < needed) return quiet;

    const commands = this.commit(this.anchors, false);

    if (this.tool === "line") {
      // Keep drawing from the point just placed, unless the loop closed
      const closed =
        this.chainOrigin?.pointId != null &&
        this.chainOrigin.pointId === here.pointId;
      this.anchors = closed ? [] : [here];
      if (closed) this.chainOrigin = null;
    } else {
      this.anchors = [];
      this.chainOrigin = null;
    }
    return { commands, problem: null };
  }

  /** Rubber-band shape for the pointer's current position */
  preview(hit: SketchHit): SketchPreview | null {
    if (this.anchors.length === 0) return null;
    if (!isCreateTool(this.tool)) return null;
    const pts = [...this.anchors.map((a) => a.pos), hit.position];
    const strokes = previewStrokes(this.tool, pts, this.options);
    return strokes.length === 0
      ? null
      : { strokes, marks: this.anchors.map((a) => a.pos) };
  }

  /** Turn the collected anchors into engine commands */
  private commit(anchors: Anchor[], closed: boolean): Command[] {
    const sketchId = this.sketchId;
    if (!sketchId) return [];
    const entities: SketchEntity[] = [];
    /** Reference an existing point, or mint one and add it to the batch */
    const pointRef = (a: Anchor): string => {
      if (a.pointId) return a.pointId;
      const id = newUuid();
      entities.push(sketchPoint(id, a.pos));
      a.pointId = id;
      return id;
    };
    /** A brand-new point at a computed position (no anchor to reuse) */
    const freshPoint = (p: Vec2): string => {
      const id = newUuid();
      entities.push(sketchPoint(id, p));
      return id;
    };
    /** Close a point loop with lines */
    const loop = (corners: Vec2[]) => {
      const ids = corners.map(freshPoint);
      for (let i = 0; i < ids.length; i++) {
        entities.push(sketchLine(newUuid(), ids[i], ids[(i + 1) % ids.length]));
      }
    };

    const pos = anchors.map((a) => a.pos);
    switch (this.tool) {
      case "point":
        pointRef(anchors[0]);
        break;

      case "line":
        entities.push(
          sketchLine(newUuid(), pointRef(anchors[0]), pointRef(anchors[1])),
        );
        break;

      case "rect":
        loop(rectCorners(pos[0], pos[1]));
        break;

      case "rectCenter":
        loop(centerRectCorners(pos[0], pos[1]));
        break;

      case "rect3":
        loop(threePointRectCorners(pos[0], pos[1], pos[2]));
        break;

      case "circle": {
        const radius = dist(pos[0], pos[1]);
        if (radius < MIN_SIZE) return [];
        entities.push(sketchCircle(newUuid(), pointRef(anchors[0]), radius));
        break;
      }

      case "circle2": {
        const center = mid(pos[0], pos[1]);
        const radius = dist(pos[0], pos[1]) / 2;
        if (radius < MIN_SIZE) return [];
        entities.push(sketchCircle(newUuid(), freshPoint(center), radius));
        break;
      }

      case "circle3": {
        const circle = circumcircle(pos[0], pos[1], pos[2]);
        if (!circle || circle.radius < MIN_SIZE) return [];
        entities.push(
          sketchCircle(newUuid(), freshPoint(circle.center), circle.radius),
        );
        break;
      }

      case "arc3": {
        const arc = arcThroughPoints(pos[0], pos[2], pos[1]);
        if (!arc || arc.radius < MIN_SIZE) return [];
        entities.push(
          sketchArc(
            newUuid(),
            freshPoint(arc.center),
            freshPoint(arc.from),
            freshPoint(arc.to),
            arc.radius,
          ),
        );
        break;
      }

      case "arcCenter": {
        const radius = dist(pos[0], pos[1]);
        if (radius < MIN_SIZE) return [];
        // The third click only sets the direction; the end sits on the radius
        const end = polar(pos[0], radius, angleOf(sub(pos[2], pos[0])));
        entities.push(
          sketchArc(
            newUuid(),
            pointRef(anchors[0]),
            pointRef(anchors[1]),
            freshPoint(end),
            radius,
          ),
        );
        break;
      }

      case "polygon": {
        const radius = dist(pos[0], pos[1]);
        if (radius < MIN_SIZE) return [];
        loop(
          polygonVertices(
            pos[0],
            radius,
            this.options.sides,
            angleOf(sub(pos[1], pos[0])),
          ),
        );
        break;
      }

      case "polygonCirc": {
        const apothem = dist(pos[0], pos[1]);
        if (apothem < MIN_SIZE) return [];
        loop(
          circumscribedVertices(
            pos[0],
            apothem,
            this.options.sides,
            angleOf(sub(pos[1], pos[0])) - Math.PI / this.options.sides,
          ),
        );
        break;
      }

      case "polygonEdge": {
        if (dist(pos[0], pos[1]) < MIN_SIZE) return [];
        loop(edgePolygonVertices(pos[0], pos[1], this.options.sides));
        break;
      }

      case "ellipse": {
        const major = dist(pos[0], pos[1]);
        const axis = norm(sub(pos[1], pos[0]));
        const minor = Math.abs(
          (pos[2][0] - pos[0][0]) * -axis[1] + (pos[2][1] - pos[0][1]) * axis[0],
        );
        if (major < MIN_SIZE || minor < MIN_SIZE) return [];
        entities.push(
          sketchEllipse(
            newUuid(),
            pointRef(anchors[0]),
            major,
            minor,
            angleOf(axis),
          ),
        );
        break;
      }

      case "slot":
      case "slotOverall": {
        const built = slotEntities(
          pos[0],
          pos[1],
          pos[2],
          this.tool === "slotOverall",
        );
        if (!built) return [];
        for (const part of built) {
          if (part.kind === "line") {
            entities.push(
              sketchLine(newUuid(), freshPoint(part.a), freshPoint(part.b)),
            );
          } else {
            entities.push(
              sketchArc(
                newUuid(),
                freshPoint(part.center),
                freshPoint(part.from),
                freshPoint(part.to),
                part.radius,
              ),
            );
          }
        }
        break;
      }

      case "spline": {
        const knots = anchors.map(pointRef);
        if (closed && this.chainOrigin?.pointId) {
          entities.push(sketchSpline(newUuid(), knots, true));
        } else {
          entities.push(sketchSpline(newUuid(), knots, false));
        }
        break;
      }
    }

    if (entities.length === 0) return [];
    const commands: Command[] = [addSketchEntities(sketchId, entities)];
    if (this.options.construction) {
      // Points stay real; it is the curves that become guides
      const curves = entities
        .filter((e) => !("Point" in e))
        .map((e) => Object.values(e)[0].id as string);
      if (curves.length > 0) {
        commands.push(setSketchConstruction(sketchId, curves, true));
      }
    }
    return commands;
  }
}

/** A slot's straight sides and its two end caps */
type SlotPart =
  | { kind: "line"; a: Vec2; b: Vec2 }
  | { kind: "arc"; center: Vec2; from: Vec2; to: Vec2; radius: number };

/**
 * Two centres and a width point make a slot. `overall` measures the two clicks
 * as the total length including the round ends, the way Fusion's second slot
 * tool does.
 */
function slotEntities(
  a: Vec2,
  b: Vec2,
  widthPoint: Vec2,
  overall: boolean,
): SlotPart[] | null {
  const axis = norm(sub(b, a));
  if (axis[0] === 0 && axis[1] === 0) return null;
  const n = perp(axis);
  const radius = Math.abs(
    (widthPoint[0] - a[0]) * n[0] + (widthPoint[1] - a[1]) * n[1],
  );
  if (radius < MIN_SIZE) return null;

  let start = a;
  let end = b;
  if (overall) {
    // Pull the centres in so the caps land on the clicked points
    if (dist(a, b) <= radius * 2) return null;
    start = add(a, scale(axis, radius));
    end = sub(b, scale(axis, radius));
  }

  const offset = scale(n, radius);
  return [
    { kind: "line", a: add(start, offset), b: add(end, offset) },
    { kind: "line", a: sub(end, offset), b: sub(start, offset) },
    // Arcs sweep counter-clockwise from `from` to `to`, so each cap starts on
    // the side that makes it bulge away from the slot rather than into it
    {
      kind: "arc",
      center: end,
      from: sub(end, offset),
      to: add(end, offset),
      radius,
    },
    {
      kind: "arc",
      center: start,
      from: add(start, offset),
      to: sub(start, offset),
      radius,
    },
  ];
}

/** The rubber-band strokes for a half-drawn shape */
function previewStrokes(
  tool: CreateTool,
  pts: Vec2[],
  options: ToolOptions,
): { pts: Vec2[]; closed: boolean }[] {
  const open = (points: Vec2[]) => [{ pts: points, closed: false }];
  const shut = (points: Vec2[]) => [{ pts: points, closed: true }];
  const last = pts.length - 1;
  const cursor = pts[last];

  switch (tool) {
    case "point":
      return [];
    case "line":
      return open([pts[0], cursor]);
    case "rect":
      return shut(rectCorners(pts[0], cursor));
    case "rectCenter":
      return shut(centerRectCorners(pts[0], cursor));
    case "rect3":
      return pts.length < 3
        ? open([pts[0], cursor])
        : shut(threePointRectCorners(pts[0], pts[1], cursor));
    case "circle":
      return shut(circlePolyline(pts[0], dist(pts[0], cursor)));
    case "circle2":
      return shut(circlePolyline(mid(pts[0], cursor), dist(pts[0], cursor) / 2));
    case "circle3": {
      if (pts.length < 3) return open([pts[0], cursor]);
      const circle = circumcircle(pts[0], pts[1], cursor);
      return circle ? shut(circlePolyline(circle.center, circle.radius)) : [];
    }
    case "arc3": {
      if (pts.length < 3) return open([pts[0], cursor]);
      const arc = arcThroughPoints(pts[0], cursor, pts[1]);
      if (!arc) return open([pts[0], pts[1]]);
      const from = angleOf(sub(arc.from, arc.center));
      const to = angleOf(sub(arc.to, arc.center));
      return open(arcPolyline(arc.center, arc.radius, from, sweepBetween(from, to)));
    }
    case "arcCenter": {
      const radius = dist(pts[0], pts[1] ?? cursor);
      if (pts.length < 3) return shut(circlePolyline(pts[0], radius));
      const from = angleOf(sub(pts[1], pts[0]));
      const to = angleOf(sub(cursor, pts[0]));
      return open(arcPolyline(pts[0], radius, from, sweepBetween(from, to)));
    }
    case "polygon":
      return shut(
        polygonVertices(
          pts[0],
          dist(pts[0], cursor),
          options.sides,
          angleOf(sub(cursor, pts[0])),
        ),
      );
    case "polygonCirc":
      return shut(
        circumscribedVertices(
          pts[0],
          dist(pts[0], cursor),
          options.sides,
          angleOf(sub(cursor, pts[0])) - Math.PI / options.sides,
        ),
      );
    case "polygonEdge":
      return shut(edgePolygonVertices(pts[0], cursor, options.sides));
    case "ellipse": {
      const major = dist(pts[0], cursor);
      if (pts.length < 3) {
        return [
          { pts: [pts[0], cursor], closed: false },
          { pts: circlePolyline(pts[0], major), closed: true },
        ];
      }
      const axis = norm(sub(pts[1], pts[0]));
      const minor = Math.abs(
        (cursor[0] - pts[0][0]) * -axis[1] + (cursor[1] - pts[0][1]) * axis[0],
      );
      return shut(
        ellipsePolyline(pts[0], dist(pts[0], pts[1]), minor, angleOf(axis)),
      );
    }
    case "slot":
    case "slotOverall": {
      if (pts.length < 3) return open([pts[0], cursor]);
      const parts = slotEntities(pts[0], pts[1], cursor, tool === "slotOverall");
      if (!parts) return open([pts[0], pts[1]]);
      return parts.map((part) =>
        part.kind === "line"
          ? { pts: [part.a, part.b], closed: false }
          : {
              pts: arcPolyline(
                part.center,
                part.radius,
                angleOf(sub(part.from, part.center)),
                sweepBetween(
                  angleOf(sub(part.from, part.center)),
                  angleOf(sub(part.to, part.center)),
                ),
              ),
              closed: false,
            },
      );
    }
    case "spline":
      return open(splinePolyline([...pts], false));
  }
}
