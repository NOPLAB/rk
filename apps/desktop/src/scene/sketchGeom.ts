// 2D helpers shared by the sketch tools.
//
// Everything works in sketch coordinates (metres). The engine's entities are
// points, lines, arcs and circles, so the job here is turning the clicks a
// Fusion-style tool collects into those four things.

import type { Vec2 } from "../engine/api";

export const TAU = Math.PI * 2;

export const sub = (a: Vec2, b: Vec2): Vec2 => [a[0] - b[0], a[1] - b[1]];
export const add = (a: Vec2, b: Vec2): Vec2 => [a[0] + b[0], a[1] + b[1]];
export const scale = (a: Vec2, k: number): Vec2 => [a[0] * k, a[1] * k];
export const mid = (a: Vec2, b: Vec2): Vec2 => scale(add(a, b), 0.5);
export const dot = (a: Vec2, b: Vec2): number => a[0] * b[0] + a[1] * b[1];
export const cross = (a: Vec2, b: Vec2): number => a[0] * b[1] - a[1] * b[0];
export const len = (a: Vec2): number => Math.hypot(a[0], a[1]);
export const dist = (a: Vec2, b: Vec2): number => len(sub(b, a));
export const angleOf = (a: Vec2): number => Math.atan2(a[1], a[0]);
/** Left-hand normal, i.e. the direction 90° counter-clockwise */
export const perp = (a: Vec2): Vec2 => [-a[1], a[0]];

export function norm(a: Vec2): Vec2 {
  const l = len(a);
  return l < 1e-12 ? [0, 0] : [a[0] / l, a[1] / l];
}

export function polar(center: Vec2, radius: number, angle: number): Vec2 {
  return [center[0] + radius * Math.cos(angle), center[1] + radius * Math.sin(angle)];
}

/** Points along an arc, counter-clockwise from `from` through `sweep` radians */
export function arcPolyline(
  center: Vec2,
  radius: number,
  from: number,
  sweep: number,
  segments = 64,
): Vec2[] {
  const steps = Math.max(4, Math.ceil((segments * Math.abs(sweep)) / TAU));
  const pts: Vec2[] = [];
  for (let i = 0; i <= steps; i++) {
    pts.push(polar(center, radius, from + (sweep * i) / steps));
  }
  return pts;
}

export function circlePolyline(center: Vec2, radius: number, segments = 64): Vec2[] {
  return Array.from({ length: segments }, (_, i) =>
    polar(center, radius, (i / segments) * TAU),
  );
}

export function ellipsePolyline(
  center: Vec2,
  major: number,
  minor: number,
  rotation: number,
  segments = 64,
): Vec2[] {
  const cos = Math.cos(rotation);
  const sin = Math.sin(rotation);
  return Array.from({ length: segments }, (_, i) => {
    const t = (i / segments) * TAU;
    const x = major * Math.cos(t);
    const y = minor * Math.sin(t);
    return [center[0] + x * cos - y * sin, center[1] + x * sin + y * cos] as Vec2;
  });
}

/** Catmull-Rom through the given knots — the curve a fit-point spline draws */
export function splinePolyline(knots: Vec2[], closed: boolean, perSpan = 16): Vec2[] {
  const n = knots.length;
  if (n < 3) return closed ? knots : knots.slice();
  const at = (i: number): Vec2 =>
    closed ? knots[((i % n) + n) % n] : knots[Math.min(Math.max(i, 0), n - 1)];
  const out: Vec2[] = [];
  const spans = closed ? n : n - 1;
  for (let s = 0; s < spans; s++) {
    const [p0, p1, p2, p3] = [at(s - 1), at(s), at(s + 1), at(s + 2)];
    for (let step = 0; step < perSpan; step++) {
      const t = step / perSpan;
      const t2 = t * t;
      const t3 = t2 * t;
      out.push([
        0.5 *
          (2 * p1[0] +
            (-p0[0] + p2[0]) * t +
            (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 +
            (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3),
        0.5 *
          (2 * p1[1] +
            (-p0[1] + p2[1]) * t +
            (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 +
            (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3),
      ]);
    }
  }
  if (!closed) out.push(knots[n - 1]);
  return out;
}

/** Axis-aligned rectangle corners, counter-clockwise from `a` */
export function rectCorners(a: Vec2, b: Vec2): Vec2[] {
  return [a, [b[0], a[1]], b, [a[0], b[1]]];
}

/** Rectangle centred on `c` with `corner` as one of its corners */
export function centerRectCorners(c: Vec2, corner: Vec2): Vec2[] {
  const d = sub(corner, c);
  return rectCorners(sub(c, d), corner);
}

/**
 * Rectangle from a baseline and a width: `a`→`b` is one edge, and `through`
 * sets how far the opposite edge sits from it. This is the tool that draws a
 * rectangle at an angle.
 */
export function threePointRectCorners(a: Vec2, b: Vec2, through: Vec2): Vec2[] {
  const along = sub(b, a);
  const n = norm(perp(along));
  const height = dot(sub(through, a), n);
  const offset = scale(n, height);
  return [a, b, add(b, offset), add(a, offset)];
}

/** Centre and radius of the circle through three points, if they are not collinear */
export function circumcircle(
  a: Vec2,
  b: Vec2,
  c: Vec2,
): { center: Vec2; radius: number } | null {
  const d = 2 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
  if (Math.abs(d) < 1e-12) return null;
  const a2 = a[0] * a[0] + a[1] * a[1];
  const b2 = b[0] * b[0] + b[1] * b[1];
  const c2 = c[0] * c[0] + c[1] * c[1];
  const center: Vec2 = [
    (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d,
    (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d,
  ];
  return { center, radius: dist(center, a) };
}

/**
 * The arc through three points, as the engine stores one: a counter-clockwise
 * sweep from `start` to `end`. When the points run clockwise the endpoints
 * swap, which is the only way to express direction without a flag.
 */
export function arcThroughPoints(
  start: Vec2,
  through: Vec2,
  end: Vec2,
): { center: Vec2; radius: number; from: Vec2; to: Vec2 } | null {
  const circle = circumcircle(start, through, end);
  if (!circle) return null;
  const ccw = cross(sub(through, start), sub(end, through)) > 0;
  return {
    ...circle,
    from: ccw ? start : end,
    to: ccw ? end : start,
  };
}

/** Counter-clockwise sweep from `from` to `to`, always in (0, TAU] */
export function sweepBetween(from: number, to: number): number {
  let sweep = to - from;
  while (sweep <= 1e-9) sweep += TAU;
  return sweep;
}

/** Regular polygon vertices; `radius` is to a vertex (inscribed in the circle) */
export function polygonVertices(
  center: Vec2,
  radius: number,
  sides: number,
  rotation: number,
): Vec2[] {
  return Array.from({ length: sides }, (_, i) =>
    polar(center, radius, rotation + (i / sides) * TAU),
  );
}

/**
 * Regular polygon whose *edges* touch a circle of `apothem`. Fusion's
 * circumscribed polygon: the cursor sets the distance to an edge, not a corner.
 */
export function circumscribedVertices(
  center: Vec2,
  apothem: number,
  sides: number,
  rotation: number,
): Vec2[] {
  const radius = apothem / Math.cos(Math.PI / sides);
  return polygonVertices(center, radius, sides, rotation + Math.PI / sides);
}

/** Regular polygon built from one edge, growing to the left of `a`→`b` */
export function edgePolygonVertices(a: Vec2, b: Vec2, sides: number): Vec2[] {
  const edge = sub(b, a);
  const step = TAU / sides;
  const out: Vec2[] = [a];
  let cursor = a;
  let heading = angleOf(edge);
  const length = len(edge);
  for (let i = 0; i < sides - 1; i++) {
    cursor = [
      cursor[0] + length * Math.cos(heading),
      cursor[1] + length * Math.sin(heading),
    ];
    out.push(cursor);
    heading += step;
  }
  return out;
}

/** Where two infinite lines meet, or `null` when they are parallel */
export function lineIntersection(
  a0: Vec2,
  a1: Vec2,
  b0: Vec2,
  b1: Vec2,
): Vec2 | null {
  const da = sub(a1, a0);
  const db = sub(b1, b0);
  const denom = cross(da, db);
  if (Math.abs(denom) < 1e-12) return null;
  const t = cross(sub(b0, a0), db) / denom;
  return add(a0, scale(da, t));
}

/** Parameter along `a0`→`a1` of the closest point to `p`, clamped to the segment */
export function projectOnSegment(a0: Vec2, a1: Vec2, p: Vec2): number {
  const d = sub(a1, a0);
  const lenSq = dot(d, d);
  if (lenSq < 1e-18) return 0;
  return Math.min(1, Math.max(0, dot(sub(p, a0), d) / lenSq));
}

export function distToSegment(a0: Vec2, a1: Vec2, p: Vec2): number {
  const t = projectOnSegment(a0, a1, p);
  return dist(p, add(a0, scale(sub(a1, a0), t)));
}

/** Intersections of an infinite line with a circle (0, 1 or 2 points) */
export function lineCircleIntersection(
  a0: Vec2,
  a1: Vec2,
  center: Vec2,
  radius: number,
): Vec2[] {
  const d = sub(a1, a0);
  const f = sub(a0, center);
  const a = dot(d, d);
  if (a < 1e-18) return [];
  const b = 2 * dot(f, d);
  const c = dot(f, f) - radius * radius;
  const disc = b * b - 4 * a * c;
  if (disc < 0) return [];
  const root = Math.sqrt(disc);
  const ts = disc < 1e-18 ? [-b / (2 * a)] : [(-b - root) / (2 * a), (-b + root) / (2 * a)];
  return ts.map((t) => add(a0, scale(d, t)));
}

/** Intersections of two circles */
export function circleCircleIntersection(
  c0: Vec2,
  r0: number,
  c1: Vec2,
  r1: number,
): Vec2[] {
  const d = dist(c0, c1);
  if (d < 1e-12 || d > r0 + r1 || d < Math.abs(r0 - r1)) return [];
  const a = (r0 * r0 - r1 * r1 + d * d) / (2 * d);
  const h2 = r0 * r0 - a * a;
  const base = add(c0, scale(norm(sub(c1, c0)), a));
  if (h2 <= 0) return [base];
  const h = Math.sqrt(h2);
  const n = scale(norm(perp(sub(c1, c0))), h);
  return [add(base, n), sub(base, n)];
}

/** Mirror a point across the line through `a` and `b` */
export function reflect(p: Vec2, a: Vec2, b: Vec2): Vec2 {
  const d = norm(sub(b, a));
  const v = sub(p, a);
  const along = scale(d, dot(v, d));
  return add(a, sub(scale(along, 2), v));
}

export function rotateAround(p: Vec2, center: Vec2, angle: number): Vec2 {
  const v = sub(p, center);
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return [center[0] + v[0] * cos - v[1] * sin, center[1] + v[0] * sin + v[1] * cos];
}

/**
 * The fillet between two segments that meet at `corner`: where each has to be
 * trimmed back to, and the arc that bridges the gap.
 */
export function filletCorner(
  corner: Vec2,
  towardsA: Vec2,
  towardsB: Vec2,
  radius: number,
): { center: Vec2; tangentA: Vec2; tangentB: Vec2 } | null {
  const da = norm(sub(towardsA, corner));
  const db = norm(sub(towardsB, corner));
  const cosine = Math.min(1, Math.max(-1, dot(da, db)));
  const angle = Math.acos(cosine);
  // Parallel or doubled-back edges have no corner to round
  if (angle < 1e-4 || Math.PI - angle < 1e-4) return null;
  const setback = radius / Math.tan(angle / 2);
  if (setback > dist(corner, towardsA) || setback > dist(corner, towardsB)) return null;
  const tangentA = add(corner, scale(da, setback));
  const tangentB = add(corner, scale(db, setback));
  const bisector = norm(add(da, db));
  const center = add(corner, scale(bisector, radius / Math.sin(angle / 2)));
  return { center, tangentA, tangentB };
}
