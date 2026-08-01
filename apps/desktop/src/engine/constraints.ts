// The constraint catalog: one table describing every constraint the sketch UI
// can create — what selection it needs, what the current geometry measures,
// and the payload to send.
//
// Field names here are the serde field names of `rk_cad::SketchConstraint`;
// apps/desktop/src-tauri/tests/command_payloads.rs applies these payloads for
// real, which is what keeps the two sides honest.

import type {
  ConstraintKind,
  SketchConstraint,
  SketchGeometry,
  Vec2,
} from "./api";

export type EntityKind = "point" | "line" | "circle" | "arc";

/** An entity slot a constraint needs filled */
type Slot = "point" | "line" | "circle" | "curve";

export interface SelectedEntity {
  id: string;
  kind: EntityKind;
}

export interface ConstraintDef {
  kind: ConstraintKind;
  label: string;
  hint: string;
  /** Entities the constraint needs; the selection fills them in any order */
  slots: Slot[];
  /** Set for dimensional constraints: the unit its value is entered in */
  unit?: "mm" | "deg";
  /** What the geometry measures today, so applying it moves nothing */
  measure?: (ids: string[], geom: SketchGeometry) => number | null;
  /** `value` is in engine units (meters / radians) */
  build: (
    id: string,
    ids: string[],
    value: number,
    geom: SketchGeometry,
  ) => SketchConstraint;
}

export const GEOMETRIC: ConstraintDef[] = [
  {
    kind: "Coincident",
    label: "Coincident",
    hint: "Two points share a location",
    slots: ["point", "point"],
    build: (id, [point1, point2]) => ({ Coincident: { id, point1, point2 } }),
  },
  {
    kind: "Horizontal",
    label: "Horizontal",
    hint: "Line parallel to the sketch X axis",
    slots: ["line"],
    build: (id, [line]) => ({ Horizontal: { id, line } }),
  },
  {
    kind: "Vertical",
    label: "Vertical",
    hint: "Line parallel to the sketch Y axis",
    slots: ["line"],
    build: (id, [line]) => ({ Vertical: { id, line } }),
  },
  {
    kind: "Parallel",
    label: "Parallel",
    hint: "Two lines stay parallel",
    slots: ["line", "line"],
    build: (id, [line1, line2]) => ({ Parallel: { id, line1, line2 } }),
  },
  {
    kind: "Perpendicular",
    label: "Perpendicular",
    hint: "Two lines meet at a right angle",
    slots: ["line", "line"],
    build: (id, [line1, line2]) => ({ Perpendicular: { id, line1, line2 } }),
  },
  {
    kind: "EqualLength",
    label: "Equal",
    hint: "Two lines keep the same length",
    slots: ["line", "line"],
    build: (id, [line1, line2]) => ({ EqualLength: { id, line1, line2 } }),
  },
  {
    kind: "EqualRadius",
    label: "Equal radius",
    hint: "Two circles keep the same radius",
    slots: ["circle", "circle"],
    build: (id, [circle1, circle2]) => ({
      EqualRadius: { id, circle1, circle2 },
    }),
  },
  {
    kind: "Tangent",
    label: "Tangent",
    hint: "A circle touches a line or another circle",
    // The solver only has tangency equations involving a circle or arc
    slots: ["circle", "curve"],
    build: (id, [curve1, curve2]) => ({ Tangent: { id, curve1, curve2 } }),
  },
  {
    kind: "PointOnCurve",
    label: "On curve",
    hint: "A point lies on a line or circle",
    slots: ["point", "curve"],
    build: (id, [point, curve]) => ({ PointOnCurve: { id, point, curve } }),
  },
  {
    kind: "Midpoint",
    label: "Midpoint",
    hint: "A point sits at the middle of a line",
    slots: ["point", "line"],
    build: (id, [point, line]) => ({ Midpoint: { id, point, line } }),
  },
  {
    kind: "Fixed",
    label: "Fix",
    hint: "Pin a point where it is",
    slots: ["point"],
    build: (id, [point], _value, geom) => {
      const p = pointAt(geom, point) ?? [0, 0];
      return { Fixed: { id, point, x: p[0], y: p[1] } };
    },
  },
];

export const DIMENSIONAL: ConstraintDef[] = [
  {
    kind: "Length",
    label: "Length",
    hint: "Drive a line's length",
    slots: ["line"],
    unit: "mm",
    measure: ([line], geom) => {
      const l = lineAt(geom, line);
      return l && length(sub(l.end, l.start));
    },
    build: (id, [line], value) => ({ Length: { id, line, value } }),
  },
  {
    kind: "Distance",
    label: "Distance",
    hint: "Drive the distance between two points",
    slots: ["point", "point"],
    unit: "mm",
    measure: ([a, b], geom) => {
      const p = pointAt(geom, a);
      const q = pointAt(geom, b);
      return p && q ? length(sub(q, p)) : null;
    },
    build: (id, [entity1, entity2], value) => ({
      Distance: { id, entity1, entity2, value },
    }),
  },
  {
    kind: "HorizontalDistance",
    label: "Dx",
    hint: "Drive the X offset between two points",
    slots: ["point", "point"],
    unit: "mm",
    // The solver drives `p2 - p1`, so the measurement has to stay signed
    measure: ([a, b], geom) => {
      const p = pointAt(geom, a);
      const q = pointAt(geom, b);
      return p && q ? q[0] - p[0] : null;
    },
    build: (id, [point1, point2], value) => ({
      HorizontalDistance: { id, point1, point2, value },
    }),
  },
  {
    kind: "VerticalDistance",
    label: "Dy",
    hint: "Drive the Y offset between two points",
    slots: ["point", "point"],
    unit: "mm",
    measure: ([a, b], geom) => {
      const p = pointAt(geom, a);
      const q = pointAt(geom, b);
      return p && q ? q[1] - p[1] : null;
    },
    build: (id, [point1, point2], value) => ({
      VerticalDistance: { id, point1, point2, value },
    }),
  },
  {
    kind: "Radius",
    label: "Radius",
    hint: "Drive a circle's radius",
    slots: ["circle"],
    unit: "mm",
    measure: ([circle], geom) => radiusAt(geom, circle),
    build: (id, [circle], value) => ({ Radius: { id, circle, value } }),
  },
  {
    kind: "Diameter",
    label: "Diameter",
    hint: "Drive a circle's diameter",
    slots: ["circle"],
    unit: "mm",
    measure: ([circle], geom) => {
      const r = radiusAt(geom, circle);
      return r === null ? null : r * 2;
    },
    build: (id, [circle], value) => ({ Diameter: { id, circle, value } }),
  },
  {
    kind: "Angle",
    label: "Angle",
    hint: "Drive the angle between two lines",
    slots: ["line", "line"],
    unit: "deg",
    measure: ([a, b], geom) => {
      const l1 = lineAt(geom, a);
      const l2 = lineAt(geom, b);
      if (!l1 || !l2) return null;
      return normalizeAngle(direction(l1) - direction(l2));
    },
    build: (id, [line1, line2], value) => ({
      Angle: { id, line1, line2, value },
    }),
  },
];

export const ALL_CONSTRAINTS = [...GEOMETRIC, ...DIMENSIONAL];

/**
 * Assign the selection to the definition's slots, returning the entity IDs in
 * slot order — or `null` when the selection does not fit.
 */
export function matchSlots(
  def: ConstraintDef,
  selection: SelectedEntity[],
): string[] | null {
  if (selection.length !== def.slots.length) return null;
  const taken = selection.map(() => false);
  const out: string[] = [];
  // Arity is at most two, so trying the assignments outright is fine. Slots
  // are filled in selection order, which is what makes Dx/Dy signs follow the
  // order the user picked.
  const fill = (slot: number): boolean => {
    if (slot === def.slots.length) return true;
    for (let i = 0; i < selection.length; i++) {
      if (taken[i] || !accepts(def.slots[slot], selection[i].kind)) continue;
      taken[i] = true;
      out[slot] = selection[i].id;
      if (fill(slot + 1)) return true;
      taken[i] = false;
    }
    return false;
  };
  return fill(0) ? out : null;
}

function accepts(slot: Slot, kind: EntityKind): boolean {
  switch (slot) {
    case "point":
      return kind === "point";
    case "line":
      return kind === "line";
    case "circle":
      return kind === "circle" || kind === "arc";
    case "curve":
      return kind !== "point";
  }
}

/** Engine units → what the value input shows */
export function toDisplay(value: number, unit: "mm" | "deg"): number {
  return unit === "mm" ? value * 1000 : (value * 180) / Math.PI;
}

export function fromDisplay(value: number, unit: "mm" | "deg"): number {
  return unit === "mm" ? value / 1000 : (value * Math.PI) / 180;
}

/** The serde variant name of a constraint payload, e.g. `"Radius"` */
export const kindOf = (constraint: SketchConstraint): string =>
  Object.keys(constraint)[0];

/** Replace a dimensional constraint's value, keeping its ID (so it edits) */
export function withValue(
  constraint: SketchConstraint,
  value: number,
): SketchConstraint {
  const [kind, fields] = Object.entries(constraint)[0];
  return { [kind]: { ...fields, value } } as SketchConstraint;
}

/** The unit a constraint's value is displayed in, or `null` if geometric */
export function unitOf(kind: string): "mm" | "deg" | null {
  return ALL_CONSTRAINTS.find((d) => d.kind === kind)?.unit ?? null;
}

// ---- geometry lookups ---------------------------------------------------

export function entityKind(
  geom: SketchGeometry,
  id: string,
): EntityKind | null {
  if (geom.points.some((p) => p.id === id)) return "point";
  if (geom.lines.some((l) => l.id === id)) return "line";
  if (geom.circles.some((c) => c.id === id)) return "circle";
  if (geom.arcs.some((a) => a.id === id)) return "arc";
  return null;
}

/** Classify a selection, dropping IDs that are no longer in the sketch */
export function classify(
  geom: SketchGeometry,
  ids: string[],
): SelectedEntity[] {
  return ids.flatMap((id) => {
    const kind = entityKind(geom, id);
    return kind ? [{ id, kind }] : [];
  });
}

function pointAt(geom: SketchGeometry, id: string): Vec2 | null {
  return geom.points.find((p) => p.id === id)?.position ?? null;
}

function lineAt(
  geom: SketchGeometry,
  id: string,
): { start: Vec2; end: Vec2 } | null {
  return geom.lines.find((l) => l.id === id) ?? null;
}

function radiusAt(geom: SketchGeometry, id: string): number | null {
  const circle = geom.circles.find((c) => c.id === id);
  if (circle) return circle.radius;
  return geom.arcs.find((a) => a.id === id)?.radius ?? null;
}

function sub(a: Vec2, b: Vec2): Vec2 {
  return [a[0] - b[0], a[1] - b[1]];
}

function length(v: Vec2): number {
  return Math.hypot(v[0], v[1]);
}

function direction(line: { start: Vec2; end: Vec2 }): number {
  return Math.atan2(line.end[1] - line.start[1], line.end[0] - line.start[0]);
}

/** Fold an angle into (-π, π], matching how the solver compares angles */
function normalizeAngle(angle: number): number {
  const turn = Math.PI * 2;
  return angle - turn * Math.round(angle / turn);
}
