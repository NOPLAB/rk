// Sketch overlay for the viewport.
//
// Everything is built in sketch coordinates (z = 0) inside a group whose
// matrix is the sketch plane's basis, so entity positions go in exactly as
// the engine reports them. Picking works the other way: the pointer ray is
// intersected with the plane and converted back to 2D.

import * as THREE from "three";
import type { SketchGeometry, SketchInfo, Vec2 } from "../engine/api";

const CURVE_SEGMENTS = 72;
/** Pixel radius for snapping onto an existing point / picking an entity */
const SNAP_PX = 11;
const PICK_PX = 8;

const GRID_EXTENT = 0.5;
const GRID_STEP = 0.025;

export type SketchPreview =
  | { kind: "line"; from: Vec2; to: Vec2 }
  | { kind: "rect"; from: Vec2; to: Vec2 }
  | { kind: "circle"; center: Vec2; radius: number };

export interface SnapResult {
  /** Sketch coordinates, moved onto an existing point when one is in reach */
  position: Vec2;
  /** The point to reuse — sharing IDs is what makes a profile closed */
  pointId: string | null;
}

/** Sketch coordinates → canvas pixels */
export type Projector = (u: number, v: number) => [number, number];

const EMPTY: SketchGeometry = {
  points: [],
  lines: [],
  circles: [],
  arcs: [],
  constraints: [],
};

export class SketchLayer {
  readonly group = new THREE.Group();

  private grid = lineObject(0x363b42, true);
  private curves = lineObject(0x8fb8ff, false);
  private construction = lineObject(0x5d6570, false);
  private highlight = lineObject(0xffa53c, false);
  /** Entities of the constraint the pointer is over in the panel */
  private hover = lineObject(0x4fd18b, false);
  private preview = lineObject(0xffc857, false);
  private points = pointObject(0xc8d2e0, 5);
  /** Snap indicator under the pointer */
  private cursor = pointObject(0xffc857, 9);
  /** Selected and hovered entities that are bare points */
  private marker = pointObject(0xffa53c, 9);
  private hoverMarker = pointObject(0x4fd18b, 9);

  private info: SketchInfo | null = null;
  private geom: SketchGeometry = EMPTY;
  private selection: string[] = [];
  private hovered: string[] = [];
  private plane = new THREE.Plane();
  private toLocal = new THREE.Matrix4();
  /** Hit-test polylines in sketch coordinates, one per curve entity */
  private outlines: { id: string; pts: Vec2[] }[] = [];

  constructor() {
    this.group.matrixAutoUpdate = false;
    this.group.visible = false;
    this.group.add(
      this.grid,
      this.curves,
      this.construction,
      this.hover,
      this.highlight,
      this.preview,
      this.points,
      this.cursor,
      this.hoverMarker,
      this.marker,
    );
    setSegments(this.grid, gridSegments());
  }

  get active(): boolean {
    return this.info !== null;
  }

  get sketchId(): string | null {
    return this.info?.id ?? null;
  }

  setSketch(info: SketchInfo | null) {
    this.info = info;
    this.group.visible = info !== null;
    if (!info) {
      this.selection = [];
      this.hovered = [];
      this.setGeometry(EMPTY);
      this.setPreview(null);
      this.setCursor(null);
      return;
    }
    // The group carries the plane basis; children are built in 2D on z = 0
    this.group.matrix.fromArray(info.transform);
    this.group.matrixWorldNeedsUpdate = true;
    this.toLocal.copy(this.group.matrix).invert();
    const n = new THREE.Vector3(...info.plane.normal);
    this.plane.setFromNormalAndCoplanarPoint(
      n,
      new THREE.Vector3(...info.plane.origin),
    );
  }

  setGeometry(geom: SketchGeometry) {
    this.geom = geom;
    const solid: number[] = [];
    const dashed: number[] = [];
    this.outlines = [];

    for (const l of geom.lines) {
      const pts: Vec2[] = [l.start, l.end];
      this.outlines.push({ id: l.id, pts });
      pushPolyline(l.construction ? dashed : solid, pts, false);
    }
    for (const c of geom.circles) {
      const pts = arcPoints(c.center, c.radius, 0, Math.PI * 2);
      this.outlines.push({ id: c.id, pts });
      pushPolyline(c.construction ? dashed : solid, pts, true);
    }
    for (const a of geom.arcs) {
      const pts = arcPoints(a.center, a.radius, a.start_angle, a.end_angle);
      this.outlines.push({ id: a.id, pts });
      pushPolyline(a.construction ? dashed : solid, pts, false);
    }

    setSegments(this.curves, solid);
    setSegments(this.construction, dashed);
    setPoints(
      this.points,
      geom.points.flatMap((p) => [p.position[0], p.position[1], 0]),
    );
    // Re-apply against the rebuilt outlines; entities that were deleted or
    // undone away simply stop matching
    this.setHighlight(this.selection);
    this.setHover(this.hovered);
  }

  setPreview(preview: SketchPreview | null) {
    if (!preview) {
      setSegments(this.preview, []);
      return;
    }
    const out: number[] = [];
    if (preview.kind === "line") {
      pushPolyline(out, [preview.from, preview.to], false);
    } else if (preview.kind === "rect") {
      pushPolyline(out, rectPoints(preview.from, preview.to), true);
    } else {
      pushPolyline(
        out,
        arcPoints(preview.center, preview.radius, 0, Math.PI * 2),
        true,
      );
    }
    setSegments(this.preview, out);
  }

  /** Marker under the pointer; drawn only when it snapped onto a point */
  setCursor(position: Vec2 | null) {
    setPoints(this.cursor, position ? [position[0], position[1], 0] : []);
  }

  /** The current selection (constraints act on several entities at once) */
  setHighlight(entityIds: string[]) {
    this.selection = entityIds;
    this.paint(this.highlight, this.marker, entityIds);
  }

  /** Entities of the constraint being pointed at, shown alongside the selection */
  setHover(entityIds: string[]) {
    this.hovered = entityIds;
    this.paint(this.hover, this.hoverMarker, entityIds);
  }

  private paint(
    curves: THREE.LineSegments,
    points: THREE.Points,
    entityIds: string[],
  ) {
    const wanted = new Set(entityIds);
    const out: number[] = [];
    for (const outline of this.outlines) {
      if (!wanted.has(outline.id)) continue;
      const closed =
        this.geom.circles.some((c) => c.id === outline.id) &&
        outline.pts.length > 2;
      pushPolyline(out, outline.pts, closed);
    }
    setSegments(curves, out);
    setPoints(
      points,
      this.geom.points
        .filter((p) => wanted.has(p.id))
        .flatMap((p) => [p.position[0], p.position[1], 0]),
    );
  }

  /** Where the pointer ray meets the sketch plane, in sketch coordinates */
  planeHit(raycaster: THREE.Raycaster): Vec2 | null {
    if (!this.info) return null;
    const hit = raycaster.ray.intersectPlane(this.plane, new THREE.Vector3());
    if (!hit) return null;
    const local = hit.applyMatrix4(this.toLocal);
    return [local.x, local.y];
  }

  /** Pull `position` onto a nearby existing point, reporting its ID */
  snap(position: Vec2, project: Projector): SnapResult {
    const [px, py] = project(position[0], position[1]);
    let best: SnapResult = { position, pointId: null };
    let bestDist = SNAP_PX;
    for (const p of this.geom.points) {
      const [qx, qy] = project(p.position[0], p.position[1]);
      const d = Math.hypot(qx - px, qy - py);
      if (d < bestDist) {
        bestDist = d;
        best = { position: p.position, pointId: p.id };
      }
    }
    return best;
  }

  /** Entity under the pointer: points win over curves */
  pick(px: number, py: number, project: Projector): string | null {
    let bestId: string | null = null;
    let bestDist = PICK_PX;
    for (const o of this.outlines) {
      for (let i = 0; i + 1 < o.pts.length; i++) {
        const a = project(o.pts[i][0], o.pts[i][1]);
        const b = project(o.pts[i + 1][0], o.pts[i + 1][1]);
        const d = distToSegment(px, py, a, b);
        if (d < bestDist) {
          bestDist = d;
          bestId = o.id;
        }
      }
    }
    bestDist = SNAP_PX;
    for (const p of this.geom.points) {
      const [qx, qy] = project(p.position[0], p.position[1]);
      const d = Math.hypot(qx - px, qy - py);
      if (d < bestDist) {
        bestDist = d;
        bestId = p.id;
      }
    }
    return bestId;
  }

  dispose() {
    for (const obj of [
      this.grid,
      this.curves,
      this.construction,
      this.highlight,
      this.hover,
      this.preview,
      this.points,
      this.cursor,
      this.marker,
      this.hoverMarker,
    ]) {
      obj.geometry.dispose();
      (obj.material as THREE.Material).dispose();
    }
  }
}

// ---- geometry helpers ---------------------------------------------------

/** Sketch geometry sits on top of solids so it stays visible while drawing */
function lineObject(color: number, depthTest: boolean): THREE.LineSegments {
  const obj = new THREE.LineSegments(
    new THREE.BufferGeometry(),
    new THREE.LineBasicMaterial({ color, depthTest, transparent: !depthTest }),
  );
  obj.renderOrder = depthTest ? 1 : 10;
  obj.frustumCulled = false;
  return obj;
}

function pointObject(color: number, size: number): THREE.Points {
  const obj = new THREE.Points(
    new THREE.BufferGeometry(),
    new THREE.PointsMaterial({
      color,
      size,
      sizeAttenuation: false,
      depthTest: false,
      transparent: true,
    }),
  );
  obj.renderOrder = 11;
  obj.frustumCulled = false;
  return obj;
}

function setSegments(obj: THREE.LineSegments, positions: number[]) {
  obj.geometry.dispose();
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  obj.geometry = geo;
  obj.visible = positions.length > 0;
}

function setPoints(obj: THREE.Points, positions: number[]) {
  obj.geometry.dispose();
  const geo = new THREE.BufferGeometry();
  geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  obj.geometry = geo;
  obj.visible = positions.length > 0;
}

/** Append a polyline as line-segment pairs (`closed` adds the wrap-around) */
function pushPolyline(out: number[], pts: Vec2[], closed: boolean) {
  const n = pts.length;
  const last = closed ? n : n - 1;
  for (let i = 0; i < last; i++) {
    const a = pts[i];
    const b = pts[(i + 1) % n];
    out.push(a[0], a[1], 0, b[0], b[1], 0);
  }
}

function arcPoints(
  center: Vec2,
  radius: number,
  startAngle: number,
  endAngle: number,
): Vec2[] {
  let sweep = endAngle - startAngle;
  // Normalize to a positive (counter-clockwise) sweep; a full circle is
  // passed in as 0 → 2π and must not collapse to nothing
  while (sweep <= 1e-6) sweep += Math.PI * 2;
  const steps = Math.max(8, Math.round((CURVE_SEGMENTS * sweep) / (Math.PI * 2)));
  const pts: Vec2[] = [];
  for (let i = 0; i <= steps; i++) {
    const a = startAngle + (sweep * i) / steps;
    pts.push([center[0] + radius * Math.cos(a), center[1] + radius * Math.sin(a)]);
  }
  return pts;
}

/** Axis-aligned rectangle corners, counter-clockwise from `a` */
export function rectPoints(a: Vec2, b: Vec2): Vec2[] {
  return [a, [b[0], a[1]], b, [a[0], b[1]]];
}

function gridSegments(): number[] {
  const out: number[] = [];
  const n = Math.round(GRID_EXTENT / GRID_STEP);
  for (let i = -n; i <= n; i++) {
    const t = i * GRID_STEP;
    out.push(t, -GRID_EXTENT, 0, t, GRID_EXTENT, 0);
    out.push(-GRID_EXTENT, t, 0, GRID_EXTENT, t, 0);
  }
  return out;
}

function distToSegment(
  px: number,
  py: number,
  a: [number, number],
  b: [number, number],
): number {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const lenSq = dx * dx + dy * dy;
  const t =
    lenSq === 0
      ? 0
      : Math.max(
          0,
          Math.min(1, ((px - a[0]) * dx + (py - a[1]) * dy) / lenSq),
        );
  return Math.hypot(px - (a[0] + t * dx), py - (a[1] + t * dy));
}
