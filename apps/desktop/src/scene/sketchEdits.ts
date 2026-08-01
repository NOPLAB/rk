// The sketch tools that reshape geometry that is already there.
//
// Each one resolves from a single click, using what the viewport already
// worked out: `hit.entityId` is the curve under the pointer and `hit.pointId`
// the vertex it snapped to. Anything that needs more than one input takes it
// from the current selection (mirror, patterns) or from the ribbon's fields
// (fillet radius, offset distance, copy count) — the same division Fusion
// makes between picking and typing.

import type { Command, SketchGeometry, Vec2 } from "../engine/api";
import {
  addSketchEntities,
  deleteSketchEntities,
  sketchArc,
  sketchCircle,
  sketchEllipse,
  sketchLine,
  sketchPoint,
  sketchSpline,
  updateSketchEntity,
  type SketchEntity,
} from "../engine/commands";
import { newUuid } from "../engine/interaction";
import {
  add,
  angleOf,
  circleCircleIntersection,
  dist,
  dot,
  filletCorner,
  lineCircleIntersection,
  lineIntersection,
  norm,
  perp,
  polar,
  projectOnSegment,
  reflect,
  rotateAround,
  scale,
  sub,
  sweepBetween,
  TAU,
} from "./sketchGeom";
import type { ToolOptions } from "./sketchTools";
import type { SketchHit } from "./viewport";

export const EDIT_TOOLS = [
  "fillet",
  "trim",
  "extend",
  "offset",
  "mirror",
  "patternRect",
  "patternCirc",
] as const;

export type EditTool = (typeof EDIT_TOOLS)[number];

export interface EditContext {
  sketchId: string;
  geometry: SketchGeometry;
  hit: SketchHit;
  /** Entities picked with the select tool — what mirror and patterns act on */
  selection: string[];
  options: ToolOptions;
}

/** A curve in the one shape the edit tools reason about */
type Curve =
  | { kind: "line"; id: string; a: Vec2; b: Vec2; aId: string; bId: string }
  | { kind: "circle"; id: string; center: Vec2; radius: number }
  | {
      kind: "arc";
      id: string;
      center: Vec2;
      radius: number;
      from: number;
      to: number;
    };

const EPS = 1e-6;

/** What an edit tool did, or why it could not */
export interface EditResult {
  commands: Command[];
  /** Shown in the status bar when nothing happened */
  problem: string | null;
}

const nothing = (problem: string): EditResult => ({ commands: [], problem });
const did = (commands: Command[]): EditResult => ({ commands, problem: null });

export function editCommands(tool: EditTool, ctx: EditContext): EditResult {
  switch (tool) {
    case "fillet":
      return filletCommands(ctx);
    case "trim":
      return trimCommands(ctx);
    case "extend":
      return extendCommands(ctx);
    case "offset":
      return offsetCommands(ctx);
    case "mirror":
      return mirrorCommands(ctx);
    case "patternRect":
      return rectPatternCommands(ctx);
    case "patternCirc":
      return circPatternCommands(ctx);
  }
}

// ---- shared views over the geometry -------------------------------------

function curves(geometry: SketchGeometry): Curve[] {
  const out: Curve[] = [];
  for (const l of geometry.lines) {
    if (l.construction) continue;
    out.push({
      kind: "line",
      id: l.id,
      a: l.start,
      b: l.end,
      aId: l.start_id,
      bId: l.end_id,
    });
  }
  for (const c of geometry.circles) {
    if (c.construction) continue;
    out.push({ kind: "circle", id: c.id, center: c.center, radius: c.radius });
  }
  for (const a of geometry.arcs) {
    if (a.construction) continue;
    out.push({
      kind: "arc",
      id: a.id,
      center: a.center,
      radius: a.radius,
      from: a.start_angle,
      to: a.end_angle,
    });
  }
  return out;
}

function findCurve(geometry: SketchGeometry, id: string | null): Curve | null {
  if (!id) return null;
  return curves(geometry).find((c) => c.id === id) ?? null;
}

/** Points where `curve` meets `other` */
function intersections(curve: Curve, other: Curve): Vec2[] {
  if (curve.kind === "line") {
    if (other.kind === "line") {
      const p = lineIntersection(curve.a, curve.b, other.a, other.b);
      return p && onSegment(other, p) ? [p] : [];
    }
    return lineCircleIntersection(curve.a, curve.b, other.center, other.radius).filter(
      (p) => onSegment(other, p),
    );
  }
  if (other.kind === "line") {
    return lineCircleIntersection(other.a, other.b, curve.center, curve.radius).filter(
      (p) => onSegment(other, p),
    );
  }
  return circleCircleIntersection(
    curve.center,
    curve.radius,
    other.center,
    other.radius,
  ).filter((p) => onSegment(other, p));
}

/** Is `p` within the drawn extent of `curve` (not just its infinite support)? */
function onSegment(curve: Curve, p: Vec2): boolean {
  if (curve.kind === "line") {
    const t = projectOnSegment(curve.a, curve.b, p);
    return (
      t > -EPS &&
      t < 1 + EPS &&
      dist(p, add(curve.a, scale(sub(curve.b, curve.a), t))) < 1e-5
    );
  }
  if (curve.kind === "circle") return true;
  const sweep = sweepBetween(curve.from, curve.to);
  const at = normalizeAngle(angleOf(sub(p, curve.center)) - curve.from);
  return at <= sweep + EPS;
}

function normalizeAngle(a: number): number {
  let out = a % TAU;
  if (out < 0) out += TAU;
  return out;
}

// ---- fillet -------------------------------------------------------------

/**
 * Round the corner where two lines meet. The click picks the vertex, so the
 * two edges fall out of it — that is the shortest path to a fillet, and it
 * matches Fusion's vertex pick.
 */
function filletCommands(ctx: EditContext): EditResult {
  const { geometry, hit, sketchId, options } = ctx;
  if (hit.pointIds.length === 0) {
    return nothing("Fillet: click the corner where two lines meet");
  }

  // The nearest point is not always the corner — trimming and drawing
  // leave loose ends lying about, so take the closest one that is
  const corners = hit.pointIds
    .map((id) => ({ id, lines: linesAt(geometry, id) }))
    .filter((c) => c.lines.length === 2);
  if (corners.length === 0) {
    const n = linesAt(geometry, hit.pointIds[0]).length;
    return nothing(
      `Fillet: that point joins ${n} line${n === 1 ? "" : "s"}, and it takes 2`,
    );
  }
  const vertexId = corners[0].id;
  const touching = corners[0].lines;

  const [first, second] = touching;
  const corner = first.start_id === vertexId ? first.start : first.end;
  const awayA = first.start_id === vertexId ? first.end : first.start;
  const awayB = second.start_id === vertexId ? second.end : second.start;

  const round = filletCorner(corner, awayA, awayB, options.filletRadius);
  if (!round) return nothing("Fillet: the radius is too big for that corner");

  const tangentA = newUuid();
  const tangentB = newUuid();
  const centerId = newUuid();
  // Which way the arc sweeps depends on which side the corner turns
  const ccw =
    (awayA[0] - corner[0]) * (awayB[1] - corner[1]) -
      (awayA[1] - corner[1]) * (awayB[0] - corner[0]) <
    0;

  const entities: SketchEntity[] = [
    sketchPoint(tangentA, round.tangentA),
    sketchPoint(tangentB, round.tangentB),
    sketchPoint(centerId, round.center),
    sketchArc(
      newUuid(),
      centerId,
      ccw ? tangentA : tangentB,
      ccw ? tangentB : tangentA,
      options.filletRadius,
    ),
    // The two shortened edges replace the originals
    sketchLine(
      newUuid(),
      first.start_id === vertexId ? tangentA : first.start_id,
      first.start_id === vertexId ? first.end_id : tangentA,
    ),
    sketchLine(
      newUuid(),
      second.start_id === vertexId ? tangentB : second.start_id,
      second.start_id === vertexId ? second.end_id : tangentB,
    ),
  ];

  // The vertex goes too, unless something else still hangs off it
  const stillUsed =
    geometry.circles.some((c) => c.center_id === vertexId) ||
    geometry.arcs.some(
      (a) =>
        a.center_id === vertexId || a.start_id === vertexId || a.end_id === vertexId,
    ) ||
    geometry.splines.some((s) => s.point_ids.includes(vertexId)) ||
    geometry.lines.some(
      (l) =>
        l.id !== first.id &&
        l.id !== second.id &&
        (l.start_id === vertexId || l.end_id === vertexId),
    );
  const doomed = stillUsed
    ? [first.id, second.id]
    : [first.id, second.id, vertexId];

  return did([
    deleteSketchEntities(sketchId, doomed),
    addSketchEntities(sketchId, entities),
  ]);
}

// ---- trim ---------------------------------------------------------------

/**
 * Cut away the stretch of curve under the pointer, bounded by wherever the
 * curve crosses anything else. With nothing crossing it, the whole curve goes.
 */
function trimCommands(ctx: EditContext): EditResult {
  const { geometry, hit, sketchId } = ctx;
  const target = findCurve(geometry, hit.entityId);
  if (!target) return nothing("Trim: click the stretch of curve to remove");
  const others = curves(geometry).filter((c) => c.id !== target.id);
  const cuts = others.flatMap((other) => intersections(target, other));

  if (target.kind === "line") {
    return trimLine(geometry, sketchId, target, cuts, hit.position);
  }
  return trimArc(geometry, sketchId, target, cuts, hit.position);
}

function trimLine(
  geometry: SketchGeometry,
  sketchId: string,
  line: Extract<Curve, { kind: "line" }>,
  cuts: Vec2[],
  at: Vec2,
): EditResult {
  const params = [0, 1, ...cuts.map((p) => projectOnSegment(line.a, line.b, p))]
    .filter((t) => t > -EPS && t < 1 + EPS)
    .sort((x, y) => x - y);
  const clicked = projectOnSegment(line.a, line.b, at);

  const keep: [number, number][] = [];
  let removed = false;
  for (let i = 0; i + 1 < params.length; i++) {
    const [lo, hi] = [params[i], params[i + 1]];
    if (hi - lo < EPS) continue;
    if (!removed && clicked >= lo && clicked <= hi) {
      removed = true;
      continue;
    }
    keep.push([lo, hi]);
  }
  if (!removed) return nothing("Trim: nothing to cut there");

  const entities: SketchEntity[] = [];
  const along = (t: number): string => {
    if (t <= EPS) return line.aId;
    if (t >= 1 - EPS) return line.bId;
    const id = newUuid();
    entities.push(sketchPoint(id, add(line.a, scale(sub(line.b, line.a), t))));
    return id;
  };
  for (const [lo, hi] of keep) {
    entities.push(sketchLine(newUuid(), along(lo), along(hi)));
  }

  // Whatever endpoints the surviving pieces no longer use would otherwise
  // linger as loose points and hijack the next snap
  const kept = new Set(
    entities.flatMap((e) => ("Line" in e ? [e.Line.start, e.Line.end] : [])),
  );
  const doomed = [line.id, ...[line.aId, line.bId].filter((id) => !kept.has(id))];
  const commands: Command[] = [
    deleteSketchEntities(sketchId, orphansOnly(geometry, doomed, [line.id])),
  ];
  if (entities.length > 0) commands.push(addSketchEntities(sketchId, entities));
  return did(commands);
}

function trimArc(
  geometry: SketchGeometry,
  sketchId: string,
  curve: Extract<Curve, { kind: "circle" | "arc" }>,
  cuts: Vec2[],
  at: Vec2,
): EditResult {
  const base = curve.kind === "arc" ? curve.from : 0;
  const span = curve.kind === "arc" ? sweepBetween(curve.from, curve.to) : TAU;
  const relative = (p: Vec2) => normalizeAngle(angleOf(sub(p, curve.center)) - base);

  const marks = [
    ...(curve.kind === "arc" ? [0, span] : [0, TAU]),
    ...cuts.map(relative).filter((a) => a > EPS && a < span - EPS),
  ].sort((x, y) => x - y);
  const clicked = relative(at);

  const keep: [number, number][] = [];
  let removed = false;
  for (let i = 0; i + 1 < marks.length; i++) {
    const [lo, hi] = [marks[i], marks[i + 1]];
    if (hi - lo < EPS) continue;
    if (!removed && clicked >= lo && clicked <= hi) {
      removed = true;
      continue;
    }
    keep.push([lo, hi]);
  }
  // A full circle with cuts wraps: the piece straddling angle 0 is one arc
  if (curve.kind === "circle" && removed && keep.length > 1) {
    const first = keep[0];
    const last = keep[keep.length - 1];
    if (first[0] <= EPS && last[1] >= TAU - EPS) {
      keep.pop();
      keep.shift();
      keep.push([last[0], first[1] + TAU]);
    }
  }
  if (!removed) return nothing("Trim: nothing to cut there");

  const entities: SketchEntity[] = [];
  const centerId = newUuid();
  if (keep.length > 0) entities.push(sketchPoint(centerId, curve.center));
  for (const [lo, hi] of keep) {
    const from = newUuid();
    const to = newUuid();
    entities.push(sketchPoint(from, polar(curve.center, curve.radius, base + lo)));
    entities.push(sketchPoint(to, polar(curve.center, curve.radius, base + hi)));
    entities.push(sketchArc(newUuid(), centerId, from, to, curve.radius));
  }

  const own =
    curve.kind === "arc"
      ? [curve.id, ...arcPointsOf(geometry, curve.id)]
      : [curve.id, ...circlePointsOf(geometry, curve.id)];
  const commands: Command[] = [
    deleteSketchEntities(sketchId, orphansOnly(geometry, own, [curve.id])),
  ];
  if (entities.length > 0) commands.push(addSketchEntities(sketchId, entities));
  return did(commands);
}

function arcPointsOf(geometry: SketchGeometry, id: string): string[] {
  const arc = geometry.arcs.find((a) => a.id === id);
  return arc ? [arc.center_id, arc.start_id, arc.end_id] : [];
}

function circlePointsOf(geometry: SketchGeometry, id: string): string[] {
  const circle = geometry.circles.find((c) => c.id === id);
  return circle ? [circle.center_id] : [];
}

/**
 * Filter `wanted` down to the curves plus the points that nothing outside
 * `removed` still refers to. Trimming would otherwise leave the endpoints of
 * the piece it cut away floating in the sketch.
 */
function orphansOnly(
  geometry: SketchGeometry,
  wanted: string[],
  removed: string[],
): string[] {
  const going = new Set(removed);
  const stillUsed = new Set<string>();
  for (const l of geometry.lines) {
    if (!going.has(l.id)) stillUsed.add(l.start_id), stillUsed.add(l.end_id);
  }
  for (const c of geometry.circles) {
    if (!going.has(c.id)) stillUsed.add(c.center_id);
  }
  for (const a of geometry.arcs) {
    if (!going.has(a.id)) {
      stillUsed.add(a.center_id), stillUsed.add(a.start_id), stillUsed.add(a.end_id);
    }
  }
  for (const e of geometry.ellipses) {
    if (!going.has(e.id)) stillUsed.add(e.center_id);
  }
  for (const sp of geometry.splines) {
    if (!going.has(sp.id)) sp.point_ids.forEach((id) => stillUsed.add(id));
  }
  return wanted.filter((id) => going.has(id) || !stillUsed.has(id));
}

// ---- extend -------------------------------------------------------------

/** Push a line's nearer end out to the first thing it would run into */
function extendCommands(ctx: EditContext): EditResult {
  const { geometry, hit, sketchId } = ctx;
  const line = findCurve(geometry, hit.entityId);
  if (!line || line.kind !== "line") {
    return nothing("Extend: click a line near the end you want lengthened");
  }

  const forward = projectOnSegment(line.a, line.b, hit.position) > 0.5;
  const from = forward ? line.a : line.b;
  const tip = forward ? line.b : line.a;
  const tipId = forward ? line.bId : line.aId;
  const direction = norm(sub(tip, from));

  let best: { at: Vec2; travel: number } | null = null;
  for (const other of curves(geometry)) {
    if (other.id === line.id) continue;
    // Cast the line on forever, then keep only hits past the tip
    const far = add(tip, scale(direction, 1e4));
    for (const p of intersections(
      { kind: "line", id: line.id, a: from, b: far, aId: "", bId: "" },
      other,
    )) {
      const travel = dot(sub(p, tip), direction);
      if (travel <= EPS) continue;
      if (!best || travel < best.travel) best = { at: p, travel };
    }
  }
  if (!best) return nothing("Extend: that end runs into nothing");

  return did([updateSketchEntity(sketchId, sketchPoint(tipId, best.at))]);
}

// ---- offset -------------------------------------------------------------

/**
 * Copy the picked curve — or the whole selection — a fixed distance to the
 * side the pointer is on. Lines that shared a corner get their offsets
 * intersected so the copy keeps its corners instead of falling apart.
 */
function offsetCommands(ctx: EditContext): EditResult {
  const { geometry, hit, sketchId, selection, options } = ctx;
  const picked = findCurve(geometry, hit.entityId);
  const chosen = selection.length > 0 ? selection : picked ? [picked.id] : [];
  const targets = curves(geometry).filter((c) => chosen.includes(c.id));
  if (targets.length === 0) {
    return nothing("Offset: click a curve, or select several first");
  }

  const reference = picked ?? targets[0];
  const distance = options.offsetDistance;
  const side = offsetSide(reference, hit.position);
  if (side === 0) return nothing("Offset: click to one side of the curve");

  const entities: SketchEntity[] = [];
  const shifted = new Map<
    string,
    { a: Vec2; b: Vec2; line: Extract<Curve, { kind: "line" }> }
  >();

  for (const curve of targets) {
    if (curve.kind === "line") {
      const n = scale(norm(perp(sub(curve.b, curve.a))), distance * side);
      shifted.set(curve.id, {
        a: add(curve.a, n),
        b: add(curve.b, n),
        line: curve,
      });
    } else {
      const radius = curve.radius + distance * side;
      if (radius <= 1e-6) continue;
      const centerId = newUuid();
      entities.push(sketchPoint(centerId, curve.center));
      if (curve.kind === "circle") {
        entities.push(sketchCircle(newUuid(), centerId, radius));
      } else {
        const from = newUuid();
        const to = newUuid();
        entities.push(sketchPoint(from, polar(curve.center, radius, curve.from)));
        entities.push(sketchPoint(to, polar(curve.center, radius, curve.to)));
        entities.push(sketchArc(newUuid(), centerId, from, to, radius));
      }
    }
  }

  // Where two originals met, their offsets should meet too
  const corners = new Map<string, Vec2>();
  const originals = targets.filter(
    (c): c is Extract<Curve, { kind: "line" }> => c.kind === "line",
  );
  for (let i = 0; i < originals.length; i++) {
    for (let j = i + 1; j < originals.length; j++) {
      const shared = sharedVertex(originals[i], originals[j]);
      if (!shared) continue;
      const one = shifted.get(originals[i].id);
      const two = shifted.get(originals[j].id);
      if (!one || !two) continue;
      const meet = lineIntersection(one.a, one.b, two.a, two.b);
      if (meet) corners.set(shared, meet);
    }
  }

  const pointIds = new Map<string, string>();
  const vertex = (originalId: string, fallback: Vec2): string => {
    const cached = pointIds.get(originalId);
    if (cached) return cached;
    const id = newUuid();
    entities.push(sketchPoint(id, corners.get(originalId) ?? fallback));
    pointIds.set(originalId, id);
    return id;
  };
  for (const { a, b, line } of shifted.values()) {
    entities.push(
      sketchLine(newUuid(), vertex(line.aId, a), vertex(line.bId, b)),
    );
  }

  return entities.length > 0
    ? did([addSketchEntities(sketchId, entities)])
    : nothing("Offset: nothing to copy");
}

/** The non-construction lines that end at `pointId` */
function linesAt(geometry: SketchGeometry, pointId: string) {
  return geometry.lines.filter(
    (l) => !l.construction && (l.start_id === pointId || l.end_id === pointId),
  );
}

function sharedVertex(
  a: Extract<Curve, { kind: "line" }>,
  b: Extract<Curve, { kind: "line" }>,
): string | null {
  for (const id of [a.aId, a.bId]) {
    if (id === b.aId || id === b.bId) return id;
  }
  return null;
}

/** +1 to offset towards the pointer, −1 away from it */
function offsetSide(curve: Curve, at: Vec2): number {
  if (curve.kind === "line") {
    const n = perp(sub(curve.b, curve.a));
    const side = dot(sub(at, curve.a), n);
    return side === 0 ? 0 : Math.sign(side);
  }
  return dist(at, curve.center) > curve.radius ? 1 : -1;
}

// ---- mirror and patterns ------------------------------------------------

/** How a copy is placed: where its points go, and whether it turns inside out */
interface Placement {
  point(p: Vec2): Vec2;
  angle(a: number): number;
  flips: boolean;
}

function mirrorCommands(ctx: EditContext): EditResult {
  const { geometry, hit, sketchId, selection } = ctx;
  if (selection.length === 0) {
    return nothing("Mirror: select what to copy first, with the Select tool");
  }
  const axis = geometry.lines.find((l) => l.id === hit.entityId);
  if (!axis) return nothing("Mirror: click the line to reflect across");
  const wanted = selection.filter((id) => id !== axis.id);
  if (wanted.length === 0) {
    return nothing("Mirror: the selection is the axis itself");
  }

  const axisAngle = angleOf(sub(axis.end, axis.start));
  const entities = duplicate(geometry, wanted, {
    point: (p) => reflect(p, axis.start, axis.end),
    angle: (a) => 2 * axisAngle - a,
    flips: true,
  });
  return entities.length > 0
    ? did([addSketchEntities(sketchId, entities)])
    : nothing("Mirror: nothing in the selection to copy");
}

function rectPatternCommands(ctx: EditContext): EditResult {
  const { geometry, hit, sketchId, selection, options } = ctx;
  if (selection.length === 0) {
    return nothing("Pattern: select what to copy first, with the Select tool");
  }
  // The click sets the direction; the ribbon sets the step and the count
  const origin = anchorOf(geometry, selection) ?? hit.position;
  const direction = norm(sub(hit.position, origin));
  if (direction[0] === 0 && direction[1] === 0) {
    return nothing("Pattern: click away from the selection to set a direction");
  }

  const entities: SketchEntity[] = [];
  for (let i = 1; i < Math.max(2, options.patternCount); i++) {
    const step = scale(direction, options.patternSpacing * i);
    entities.push(
      ...duplicate(geometry, selection, {
        point: (p) => add(p, step),
        angle: (a) => a,
        flips: false,
      }),
    );
  }
  return entities.length > 0
    ? did([addSketchEntities(sketchId, entities)])
    : nothing("Pattern: nothing in the selection to copy");
}

function circPatternCommands(ctx: EditContext): EditResult {
  const { geometry, hit, sketchId, selection, options } = ctx;
  if (selection.length === 0) {
    return nothing("Pattern: select what to copy first, with the Select tool");
  }
  const center = hit.position;
  const count = Math.max(2, options.patternCount);
  const full = options.patternAngle >= TAU - 1e-3;
  const step = options.patternAngle / (full ? count : count - 1);

  const entities: SketchEntity[] = [];
  for (let i = 1; i < count; i++) {
    const turn = step * i;
    entities.push(
      ...duplicate(geometry, selection, {
        point: (p) => rotateAround(p, center, turn),
        angle: (a) => a + turn,
        flips: false,
      }),
    );
  }
  return entities.length > 0
    ? did([addSketchEntities(sketchId, entities)])
    : nothing("Pattern: nothing in the selection to copy");
}

/** Centre of the selection's points, used as the origin a pattern grows from */
function anchorOf(geometry: SketchGeometry, ids: string[]): Vec2 | null {
  const wanted = new Set(referencedPoints(geometry, ids));
  const points = geometry.points.filter((p) => wanted.has(p.id));
  if (points.length === 0) return null;
  const sum = points.reduce<Vec2>((acc, p) => add(acc, p.position), [0, 0]);
  return scale(sum, 1 / points.length);
}

function referencedPoints(geometry: SketchGeometry, ids: string[]): string[] {
  const wanted = new Set(ids);
  const out: string[] = [];
  for (const l of geometry.lines) {
    if (wanted.has(l.id)) out.push(l.start_id, l.end_id);
  }
  for (const c of geometry.circles) {
    if (wanted.has(c.id)) out.push(c.center_id);
  }
  for (const a of geometry.arcs) {
    if (wanted.has(a.id)) out.push(a.center_id, a.start_id, a.end_id);
  }
  for (const p of geometry.points) {
    if (wanted.has(p.id)) out.push(p.id);
  }
  return out;
}

/**
 * Copy a set of entities through `place`. Points shared by two originals stay
 * shared in the copy, which is what keeps a mirrored outline closed.
 */
function duplicate(
  geometry: SketchGeometry,
  ids: string[],
  place: Placement,
): SketchEntity[] {
  const wanted = new Set(ids);
  const entities: SketchEntity[] = [];
  const remapped = new Map<string, string>();
  const point = (originalId: string, at: Vec2): string => {
    const cached = remapped.get(originalId);
    if (cached) return cached;
    const id = newUuid();
    entities.push(sketchPoint(id, place.point(at)));
    remapped.set(originalId, id);
    return id;
  };
  for (const l of geometry.lines) {
    if (!wanted.has(l.id)) continue;
    entities.push(
      sketchLine(newUuid(), point(l.start_id, l.start), point(l.end_id, l.end)),
    );
  }
  for (const c of geometry.circles) {
    if (!wanted.has(c.id)) continue;
    entities.push(sketchCircle(newUuid(), point(c.center_id, c.center), c.radius));
  }
  for (const a of geometry.arcs) {
    if (!wanted.has(a.id)) continue;
    const from = point(a.start_id, polar(a.center, a.radius, a.start_angle));
    const to = point(a.end_id, polar(a.center, a.radius, a.end_angle));
    // A reflection reverses the sweep, and the engine only stores direction
    // through the order of the endpoints
    const [head, tail] = place.flips ? [to, from] : [from, to];
    entities.push(
      sketchArc(newUuid(), point(a.center_id, a.center), head, tail, a.radius),
    );
  }
  for (const e of geometry.ellipses) {
    if (!wanted.has(e.id)) continue;
    entities.push(
      sketchEllipse(
        newUuid(),
        point(e.center_id, e.center),
        e.major_radius,
        e.minor_radius,
        place.angle(e.rotation),
      ),
    );
  }
  for (const s of geometry.splines) {
    if (!wanted.has(s.id)) continue;
    const knots = s.point_ids.map((id, i) => point(id, s.points[i]));
    entities.push(sketchSpline(newUuid(), knots, s.closed));
  }
  for (const p of geometry.points) {
    // A bare point in the selection is copied on its own
    if (!wanted.has(p.id)) continue;
    if (!remapped.has(p.id)) point(p.id, p.position);
  }
  return entities;
}
