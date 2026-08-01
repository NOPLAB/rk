// Picking the plane a sketch starts on, in the 3D view.
//
// Fusion's flow: hit "Create Sketch", then click either one of the three
// origin planes or a flat face of something already built. The three origin
// quads only appear while a pick is running; faces come from `flatFaceAt`,
// which recovers them from the mesh because the kernel does not carry them
// through tessellation.

import * as THREE from "three";
import type { SketchPlane, Vec3 } from "../engine/api";
import { flatFaceAt } from "./facePlane";

export type OriginPlane = "XY" | "XZ" | "YZ";

export interface PlanePick {
  /** Shown in the status bar while hovering, and used to name the sketch */
  label: string;
  plane: SketchPlane;
}

const PLANE_COLORS: Record<OriginPlane, number> = {
  XY: 0x4d80e6,
  XZ: 0x4de680,
  YZ: 0xe6804d,
};

/** Half-width of the origin quads when the scene is empty, in metres */
const BASE_HALF_SIZE = 0.08;

export class PlanePicker {
  readonly group = new THREE.Group();
  private quads = new Map<OriginPlane, THREE.Mesh>();
  private highlight: THREE.Mesh;
  private highlightGeometry = new THREE.BufferGeometry();
  private hovered: OriginPlane | null = null;

  constructor() {
    this.group.visible = false;
    for (const which of ["XY", "XZ", "YZ"] as OriginPlane[]) {
      const mesh = new THREE.Mesh(
        new THREE.PlaneGeometry(1, 1),
        new THREE.MeshBasicMaterial({
          color: PLANE_COLORS[which],
          transparent: true,
          opacity: 0.16,
          side: THREE.DoubleSide,
          depthWrite: false,
        }),
      );
      mesh.userData.originPlane = which;
      mesh.renderOrder = 5;
      orientQuad(mesh, which);
      this.quads.set(which, mesh);
      this.group.add(mesh);
    }

    this.highlight = new THREE.Mesh(
      this.highlightGeometry,
      new THREE.MeshBasicMaterial({
        color: 0x7fc4ff,
        transparent: true,
        opacity: 0.4,
        side: THREE.DoubleSide,
        depthWrite: false,
        depthTest: false,
      }),
    );
    this.highlight.renderOrder = 6;
    this.highlight.visible = false;
    this.highlight.frustumCulled = false;
    this.group.add(this.highlight);
  }

  get active(): boolean {
    return this.group.visible;
  }

  /** Show the origin quads, sized to whatever is already in the scene */
  begin(bounds: THREE.Box3 | null) {
    let half = BASE_HALF_SIZE;
    if (bounds && !bounds.isEmpty()) {
      half = Math.max(half, bounds.getSize(new THREE.Vector3()).length() * 0.6);
    }
    for (const mesh of this.quads.values()) mesh.scale.setScalar(half * 2);
    this.group.visible = true;
    this.setHighlight(null);
  }

  end() {
    this.group.visible = false;
    this.hovered = null;
    this.highlight.visible = false;
    this.setHighlight(null);
  }

  /**
   * What the pointer is over: an origin quad if it hits one, otherwise the
   * flat face of whatever solid is behind it. `light` also paints the result,
   * which is what a pointer move wants and a click does not.
   */
  probe(
    raycaster: THREE.Raycaster,
    solids: THREE.Mesh[],
    light: boolean,
  ): PlanePick | null {
    const quads = raycaster.intersectObjects([...this.quads.values()], false);
    if (quads.length > 0) {
      const which = quads[0].object.userData.originPlane as OriginPlane;
      if (light) {
        this.setHighlight(which);
        this.highlight.visible = false;
      }
      return { label: `${which} Plane`, plane: originPlane(which) };
    }
    if (light) this.setHighlight(null);

    const hit = raycaster.intersectObjects(solids, false)[0];
    const face =
      hit && hit.faceIndex != null
        ? flatFaceAt(hit.object as THREE.Mesh, hit.faceIndex)
        : null;
    if (!hit || !face) {
      if (light) this.highlight.visible = false;
      return null;
    }

    if (light) {
      this.highlightGeometry.setAttribute(
        "position",
        new THREE.BufferAttribute(face.positions, 3),
      );
      this.highlightGeometry.computeBoundingSphere();
      this.highlight.visible = true;
    }
    const name =
      (hit.object.userData.partName as string | undefined) ??
      (hit.object.userData.bodyId ? "Body" : "Face");
    return { label: `Face of ${name}`, plane: planeFromFace(face.normal, face.origin) };
  }

  /** Called from the browser tree so hovering a plane row lights the quad */
  setHighlight(which: OriginPlane | null) {
    if (this.hovered === which) return;
    for (const [key, mesh] of this.quads) {
      const material = mesh.material as THREE.MeshBasicMaterial;
      material.opacity = key === which ? 0.36 : 0.16;
    }
    this.hovered = which;
  }

  dispose() {
    for (const mesh of this.quads.values()) {
      mesh.geometry.dispose();
      (mesh.material as THREE.Material).dispose();
    }
    this.highlightGeometry.dispose();
    (this.highlight.material as THREE.Material).dispose();
  }
}

/** Three builds its quads on XY; the other two need turning onto their axes */
function orientQuad(mesh: THREE.Mesh, which: OriginPlane) {
  if (which === "XZ") mesh.rotation.x = Math.PI / 2;
  else if (which === "YZ") mesh.rotation.y = Math.PI / 2;
}

const ORIGIN_PLANES: Record<OriginPlane, SketchPlane> = {
  XY: { origin: [0, 0, 0], normal: [0, 0, 1], x_axis: [1, 0, 0], y_axis: [0, 1, 0] },
  XZ: { origin: [0, 0, 0], normal: [0, 1, 0], x_axis: [1, 0, 0], y_axis: [0, 0, 1] },
  YZ: { origin: [0, 0, 0], normal: [1, 0, 0], x_axis: [0, 1, 0], y_axis: [0, 0, 1] },
};

export function originPlane(which: OriginPlane): SketchPlane {
  return ORIGIN_PLANES[which];
}

/**
 * A sketch frame for a face. The in-plane axes are free, so they are pinned
 * to the world axis least parallel to the normal — otherwise the same face
 * would give a different 2D frame from one session to the next.
 */
export function planeFromFace(
  normal: THREE.Vector3,
  origin: THREE.Vector3,
): SketchPlane {
  const n = normal.clone().normalize();
  const candidates = [
    new THREE.Vector3(0, 0, 1),
    new THREE.Vector3(1, 0, 0),
    new THREE.Vector3(0, 1, 0),
  ];
  let reference = candidates[0];
  let smallest = Infinity;
  for (const axis of candidates) {
    const alignment = Math.abs(axis.dot(n));
    if (alignment < smallest) {
      smallest = alignment;
      reference = axis;
    }
  }
  const x = reference.clone().cross(n).normalize();
  const y = n.clone().cross(x).normalize();
  return {
    origin: origin.toArray() as Vec3,
    normal: n.toArray() as Vec3,
    x_axis: x.toArray() as Vec3,
    y_axis: y.toArray() as Vec3,
  };
}
