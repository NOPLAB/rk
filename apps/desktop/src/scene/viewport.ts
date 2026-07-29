// Three.js viewport: the desktop counterpart of the egui renderer glue.
//
// Scene sync mirrors rk-frontend/src/sync.rs — engine events drive
// incremental updates; document_reset triggers a full rebuild from a
// snapshot. RK is Z-up; Three defaults to Y-up, so the grid is rotated
// onto the XY plane and every camera/up vector is Z-up.

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { TransformControls } from "three/addons/controls/TransformControls.js";
import {
  getBodyMesh,
  getPartMesh,
  sceneSnapshot,
  sketchGeometry,
  type EngineEvent,
  type GeometryType,
  type LinkInfo,
  type Mat4,
  type MeshPayload,
  type Rgba,
  type SceneSnapshot,
  type SketchGeometry,
  type SketchInfo,
  type Vec2,
} from "../engine/api";
import { SketchLayer, type Projector, type SketchPreview } from "./sketchLayer";

THREE.Object3D.DEFAULT_UP.set(0, 0, 1);

const SELECT_EMISSIVE = new THREE.Color(0x2a5db0);
const BLACK = new THREE.Color(0x000000);

/** "none" hides the gizmo and leaves click-selection alone */
export type GizmoMode = "none" | "translate" | "rotate";

/** Pointer position resolved against the active sketch */
export interface SketchHit {
  /** Sketch coordinates, snapped onto an existing point when close enough */
  position: Vec2;
  /** The snapped point's ID, if any — reuse it to keep profiles connected */
  pointId: string | null;
  /** Entity under the pointer (for the select tool) */
  entityId: string | null;
}

export class Viewport {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera: THREE.PerspectiveCamera;
  private controls: OrbitControls;
  private gizmo: TransformControls;
  private gizmoMode: GizmoMode = "none";
  private dragCanceled = false;
  private raycaster = new THREE.Raycaster();
  private partMeshes = new Map<string, THREE.Mesh>();
  private bodyMeshes = new Map<string, THREE.Mesh>();
  private selected: string | null = null;
  private sketch = new SketchLayer();
  private sketchInfo: SketchInfo | null = null;
  private collisions = new THREE.Group();
  private collisionMaterial = new THREE.MeshBasicMaterial({
    color: 0x4fd18b,
    wireframe: true,
    transparent: true,
    opacity: 0.8,
  });
  private disposed = false;

  /** Fired when the user clicks a part (or empty space → null) */
  onPick: ((partId: string | null) => void) | null = null;
  /** Gizmo drag started on `partId` */
  onTransformStart: ((partId: string) => void) | null = null;
  /** Gizmo moved `partId`; `world` is its new render transform (column-major) */
  onTransform: ((partId: string, world: number[]) => void) | null = null;
  /** Drag finished; `canceled` means the engine session must be rolled back */
  onTransformEnd: ((canceled: boolean) => void) | null = null;
  /** Click on the active sketch plane; `additive` is a shift-click */
  onSketchClick: ((hit: SketchHit, additive: boolean) => void) | null = null;
  /** Pointer moved over the active sketch plane (drives the drawing preview) */
  onSketchMove: ((hit: SketchHit) => void) | null = null;
  /** The active sketch's geometry, whenever it is (re-)pulled from the engine */
  onSketchGeometry: ((geometry: SketchGeometry) => void) | null = null;

  constructor(canvas: HTMLCanvasElement) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    this.renderer.setPixelRatio(window.devicePixelRatio);
    this.scene.background = new THREE.Color(0x191c20);

    this.camera = new THREE.PerspectiveCamera(50, 1, 0.001, 1000);
    this.camera.up.set(0, 0, 1);
    this.camera.position.set(0.5, -0.5, 0.35);

    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.target.set(0, 0, 0);

    const grid = new THREE.GridHelper(10, 100, 0x50555c, 0x2b2f34);
    grid.rotation.x = Math.PI / 2; // XZ (three default) → XY plane
    this.scene.add(grid);
    this.scene.add(new THREE.AxesHelper(0.15));

    const hemi = new THREE.HemisphereLight(0xffffff, 0x3a3d42, 0.9);
    hemi.position.set(0, 0, 1);
    this.scene.add(hemi);
    const dir = new THREE.DirectionalLight(0xffffff, 1.6);
    dir.position.set(1.5, -2.0, 3.0);
    this.scene.add(dir);
    const fill = new THREE.DirectionalLight(0xffffff, 0.4);
    fill.position.set(-2.0, 1.5, -1.0);
    this.scene.add(fill);

    this.scene.add(this.sketch.group);
    this.collisions.visible = false;
    this.scene.add(this.collisions);

    this.gizmo = new TransformControls(this.camera, canvas);
    this.gizmo.size = 0.8;
    const helper = this.gizmo.getHelper();
    helper.visible = false;
    this.scene.add(helper);

    // Orbiting and dragging must not happen at once
    this.gizmo.addEventListener("dragging-changed", (e) => {
      this.controls.enabled = !e.value;
      if (e.value) {
        this.dragCanceled = false;
        const partId = this.gizmo.object?.userData.partId as string | undefined;
        if (partId) this.onTransformStart?.(partId);
      } else {
        const canceled = this.dragCanceled;
        this.dragCanceled = false;
        this.onTransformEnd?.(canceled);
        // cancelDrag() detached the object to stop the drag; put it back
        if (canceled) this.attachGizmo();
      }
    });

    this.gizmo.addEventListener("objectChange", () => {
      if (this.dragCanceled) return;
      const mesh = this.gizmo.object;
      const partId = mesh?.userData.partId as string | undefined;
      if (!mesh || !partId) return;
      mesh.updateMatrix();
      this.onTransform?.(partId, mesh.matrix.toArray());
    });

    // Click-select with a small drag threshold so orbiting never selects
    let downAt: [number, number] | null = null;
    canvas.addEventListener("pointerdown", (e) => {
      // The gizmo's own listener runs first, so a grabbed handle already
      // shows up as an active axis — that press is not a selection gesture
      if (e.button === 0 && this.gizmo.axis === null) {
        downAt = [e.clientX, e.clientY];
      }
    });
    canvas.addEventListener("pointerup", (e) => {
      if (!downAt || e.button !== 0) return;
      const [x0, y0] = downAt;
      downAt = null;
      // A drag orbited the view; only a click edits
      if (Math.hypot(e.clientX - x0, e.clientY - y0) > 4) return;
      if (this.sketch.active) {
        const hit = this.sketchHit(e);
        if (hit) this.onSketchClick?.(hit, e.shiftKey);
        return;
      }
      this.onPick?.(this.pick(e));
    });
    canvas.addEventListener("pointermove", (e) => {
      if (!this.sketch.active) return;
      const hit = this.sketchHit(e);
      if (!hit) return;
      this.sketch.setCursor(hit.pointId ? hit.position : null);
      this.onSketchMove?.(hit);
    });

    this.renderer.setAnimationLoop(() => {
      if (this.disposed) return;
      this.controls.update();
      this.renderer.render(this.scene, this.camera);
    });
  }

  resize(width: number, height: number) {
    if (width <= 0 || height <= 0) return;
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
  }

  dispose() {
    this.disposed = true;
    this.renderer.setAnimationLoop(null);
    this.gizmo.detach();
    this.gizmo.dispose();
    this.controls.dispose();
    this.sketch.dispose();
    this.setCollisions([]);
    this.collisionMaterial.dispose();
    this.clearAll();
    this.renderer.dispose();
  }

  // ---- gizmo ------------------------------------------------------------

  setGizmoMode(mode: GizmoMode) {
    this.gizmoMode = mode;
    if (mode !== "none") this.gizmo.mode = mode;
    this.attachGizmo();
  }

  /**
   * Abort the drag in progress: restore the mesh to where it started and
   * detach so further pointer moves are ignored. The pointer-up that follows
   * still ends the drag normally, reporting the cancel to the engine.
   */
  cancelDrag() {
    if (!this.gizmo.dragging || this.dragCanceled) return;
    this.dragCanceled = true;
    this.gizmo.reset();
    this.gizmo.detach();
  }

  private attachGizmo() {
    const mesh = this.selected ? this.partMeshes.get(this.selected) : undefined;
    // Sketch mode owns the pointer; a gizmo on top of it would fight for clicks
    if (this.gizmoMode === "none" || this.sketch.active || !mesh) {
      this.gizmo.detach();
      this.gizmo.getHelper().visible = false;
      return;
    }
    this.gizmo.attach(mesh);
    this.gizmo.getHelper().visible = true;
  }

  // ---- collision shapes -------------------------------------------------

  /**
   * Rebuild the collision wireframes from the snapshot's links. Their
   * transforms already fold in the link's world pose, so this is called
   * on every refresh rather than driven by events.
   */
  setCollisions(links: LinkInfo[]) {
    for (const child of [...this.collisions.children]) {
      this.collisions.remove(child);
      const mesh = child as THREE.Mesh;
      mesh.geometry.dispose();
    }
    for (const link of links) {
      for (const collision of link.collisions) {
        const geo = collisionGeometry(collision.geometry);
        if (!geo) continue; // imported meshes have no primitive to draw
        const mesh = new THREE.Mesh(geo, this.collisionMaterial);
        mesh.matrixAutoUpdate = false;
        mesh.matrix.fromArray(collision.transform);
        this.collisions.add(mesh);
      }
    }
  }

  setCollisionsVisible(visible: boolean) {
    this.collisions.visible = visible;
  }

  // ---- sketch mode ------------------------------------------------------

  /** Enter sketch mode on `info`, or leave it with `null` */
  async setSketch(info: SketchInfo | null) {
    const entered = info !== null && info.id !== this.sketchInfo?.id;
    this.sketchInfo = info;
    this.sketch.setSketch(info);
    this.attachGizmo();
    if (!info) {
      this.camera.up.set(0, 0, 1);
      return;
    }
    await this.refreshSketch();
    if (entered) this.alignToSketch();
  }

  /** Re-pull the active sketch's entities from the engine */
  async refreshSketch() {
    const id = this.sketch.sketchId;
    if (!id) return;
    try {
      const geometry = await sketchGeometry(id);
      this.sketch.setGeometry(geometry);
      // The panels need the same geometry to classify a selection and to
      // measure dimensions; handing it over saves a second round trip
      this.onSketchGeometry?.(geometry);
    } catch (e) {
      console.warn(`sketch ${id} geometry:`, e);
    }
  }

  setSketchPreview(preview: SketchPreview | null) {
    this.sketch.setPreview(preview);
  }

  setSketchSelection(entityIds: string[]) {
    this.sketch.setHighlight(entityIds);
  }

  /** Entities of the constraint the pointer is over in the panel */
  setSketchHover(entityIds: string[]) {
    this.sketch.setHover(entityIds);
  }

  /** Look straight down the sketch plane's normal, keeping the zoom level */
  alignToSketch() {
    const info = this.sketchInfo;
    if (!info) return;
    const origin = new THREE.Vector3(...info.plane.origin);
    const normal = new THREE.Vector3(...info.plane.normal).normalize();
    const dist = Math.max(
      this.camera.position.distanceTo(this.controls.target),
      0.05,
    );
    this.camera.up.set(...info.plane.y_axis);
    this.camera.position.copy(origin.clone().add(normal.multiplyScalar(dist)));
    this.controls.target.copy(origin);
  }

  private sketchHit(e: PointerEvent): SketchHit | null {
    const rect = this.renderer.domElement.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    this.raycaster.setFromCamera(
      new THREE.Vector2((px / rect.width) * 2 - 1, -(py / rect.height) * 2 + 1),
      this.camera,
    );
    const raw = this.sketch.planeHit(this.raycaster);
    if (!raw) return null;
    const project = this.sketchProjector(rect.width, rect.height);
    const snapped = this.sketch.snap(raw, project);
    return {
      position: snapped.position,
      pointId: snapped.pointId,
      entityId: this.sketch.pick(px, py, project),
    };
  }

  /** Sketch coordinates → canvas pixels, for snapping and picking */
  private sketchProjector(width: number, height: number): Projector {
    const toWorld = this.sketch.group.matrix;
    const v = new THREE.Vector3();
    return (u, w) => {
      v.set(u, w, 0).applyMatrix4(toWorld).project(this.camera);
      return [((v.x + 1) / 2) * width, ((1 - v.y) / 2) * height];
    };
  }

  // ---- engine sync ------------------------------------------------------

  /** Mirror of sync.rs `apply_events` */
  async applyEvents(events: EngineEvent[]) {
    for (const ev of events) {
      switch (ev.type) {
        case "document_reset":
          // Undo/redo must not yank the camera around; a new document does
          await this.rebuildFromSnapshot(
            await sceneSnapshot(),
            ev.reason !== "undo_redo",
          );
          break;
        case "part_added":
        case "part_appearance_changed":
          await this.addOrUpdatePart(ev.part_id as string);
          break;
        case "part_removed":
          this.removeFrom(this.partMeshes, ev.part_id as string);
          break;
        case "world_transforms_changed":
          this.setTransforms(ev.transforms as [string, Mat4][]);
          break;
        case "bodies_rebuilt": {
          this.clearMap(this.bodyMeshes);
          for (const id of ev.body_ids as string[]) {
            await this.addBody(id);
          }
          break;
        }
        case "sketch_geometry_changed":
        case "sketch_solved":
          if (ev.sketch_id === this.sketch.sketchId) await this.refreshSketch();
          break;
        default:
          // list/history/joint changes are handled by the React layer
          break;
      }
    }
  }

  async rebuildFromSnapshot(snap: SceneSnapshot, fit = true) {
    this.clearAll();
    for (const part of snap.parts) {
      if (part.has_mesh) await this.addOrUpdatePart(part.id);
    }
    this.setTransforms(snap.transforms);
    for (const id of snap.body_ids) {
      await this.addBody(id);
    }
    // The sketch may have been undone away; the React layer decides whether
    // to stay in sketch mode, we just resync what is still there
    await this.refreshSketch();
    if (fit && (snap.parts.length > 0 || snap.body_ids.length > 0)) {
      this.fitCamera();
    }
  }

  setTransforms(transforms: [string, Mat4][]) {
    for (const [id, m] of transforms) {
      const mesh = this.partMeshes.get(id);
      if (!mesh) continue;
      // While dragging, the mesh under the gizmo leads and the engine
      // follows — echoing its own value back would fight the pointer
      if (this.gizmo.dragging && this.gizmo.object === mesh) continue;
      mesh.matrix.fromArray(m);
      mesh.matrix.decompose(mesh.position, mesh.quaternion, mesh.scale);
    }
  }

  setSelected(partId: string | null) {
    this.selected = partId;
    for (const [id, mesh] of this.partMeshes) {
      const mat = mesh.material as THREE.MeshStandardMaterial;
      mat.emissive.copy(id === partId ? SELECT_EMISSIVE : BLACK);
      mat.emissiveIntensity = 0.6;
    }
    this.attachGizmo();
  }

  fitCamera() {
    const box = new THREE.Box3();
    let any = false;
    for (const mesh of [...this.partMeshes.values(), ...this.bodyMeshes.values()]) {
      box.expandByObject(mesh);
      any = true;
    }
    if (!any || box.isEmpty()) return;
    const center = box.getCenter(new THREE.Vector3());
    const radius = Math.max(box.getSize(new THREE.Vector3()).length() / 2, 0.05);
    const dir = new THREE.Vector3(1, -1, 0.7).normalize();
    this.camera.position.copy(center.clone().add(dir.multiplyScalar(radius * 2.5)));
    this.controls.target.copy(center);
  }

  // ---- mesh management --------------------------------------------------

  private async addOrUpdatePart(partId: string) {
    const payload = await getPartMesh(partId);
    const prevMatrix = this.partMeshes.get(partId)?.matrix.clone();
    this.removeFrom(this.partMeshes, partId);
    if (payload.vertices.length === 0) return;
    const mesh = new THREE.Mesh(
      buildGeometry(payload),
      makeMaterial(payload.color),
    );
    // Position/rotation/scale stay authoritative so the gizmo can drive them;
    // engine transforms are decomposed into them in setTransforms
    if (prevMatrix) {
      prevMatrix.decompose(mesh.position, mesh.quaternion, mesh.scale);
    }
    mesh.userData.partId = partId;
    this.partMeshes.set(partId, mesh);
    this.scene.add(mesh);
    if (this.selected === partId) this.setSelected(partId);
  }

  private async addBody(bodyId: string) {
    try {
      const payload = await getBodyMesh(bodyId);
      const mesh = new THREE.Mesh(
        buildGeometry(payload),
        makeMaterial(payload.color),
      );
      mesh.matrixAutoUpdate = false;
      this.bodyMeshes.set(bodyId, mesh);
      this.scene.add(mesh);
    } catch (e) {
      console.warn(`skipping body ${bodyId}:`, e);
    }
  }

  private pick(e: PointerEvent): string | null {
    const rect = (e.target as HTMLCanvasElement).getBoundingClientRect();
    const ndc = new THREE.Vector2(
      ((e.clientX - rect.left) / rect.width) * 2 - 1,
      -((e.clientY - rect.top) / rect.height) * 2 + 1,
    );
    this.raycaster.setFromCamera(ndc, this.camera);
    const hits = this.raycaster.intersectObjects(
      [...this.partMeshes.values()],
      false,
    );
    return (hits[0]?.object.userData.partId as string | undefined) ?? null;
  }

  private removeFrom(map: Map<string, THREE.Mesh>, id: string) {
    const mesh = map.get(id);
    if (!mesh) return;
    if (this.gizmo.object === mesh) this.gizmo.detach();
    this.scene.remove(mesh);
    mesh.geometry.dispose();
    (mesh.material as THREE.Material).dispose();
    map.delete(id);
  }

  private clearMap(map: Map<string, THREE.Mesh>) {
    for (const id of [...map.keys()]) this.removeFrom(map, id);
  }

  private clearAll() {
    this.clearMap(this.partMeshes);
    this.clearMap(this.bodyMeshes);
  }
}

// ---- geometry helpers ---------------------------------------------------

/**
 * Wireframe stand-in for a collision element. Three builds cylinders and
 * capsules along Y; RK (like URDF) stands them on Z.
 */
function collisionGeometry(g: GeometryType): THREE.BufferGeometry | null {
  if ("Box" in g) {
    const [x, y, z] = g.Box.size;
    return new THREE.BoxGeometry(x, y, z);
  }
  if ("Sphere" in g) return new THREE.SphereGeometry(g.Sphere.radius, 16, 12);
  if ("Cylinder" in g) {
    const { radius, length } = g.Cylinder;
    return standUp(new THREE.CylinderGeometry(radius, radius, length, 20));
  }
  if ("Capsule" in g) {
    const { radius, length } = g.Capsule;
    return standUp(new THREE.CapsuleGeometry(radius, length, 6, 16));
  }
  return null;
}

function standUp(geo: THREE.BufferGeometry): THREE.BufferGeometry {
  geo.rotateX(Math.PI / 2);
  return geo;
}

function makeMaterial(color: Rgba): THREE.MeshStandardMaterial {
  return new THREE.MeshStandardMaterial({
    color: new THREE.Color(color[0], color[1], color[2]),
    metalness: 0.1,
    roughness: 0.7,
    transparent: color[3] < 1,
    opacity: color[3],
    side: THREE.DoubleSide,
  });
}

/**
 * Parts carry one normal per triangle; CAD bodies carry one per vertex.
 * Per-vertex → indexed geometry as-is. Per-triangle → expand to flat-shaded
 * non-indexed triangles. Missing normals → let Three compute them.
 */
function buildGeometry(payload: MeshPayload): THREE.BufferGeometry {
  const geo = new THREE.BufferGeometry();

  if (
    payload.normals.length === payload.vertices.length &&
    payload.normals.length > 0
  ) {
    geo.setAttribute(
      "position",
      new THREE.Float32BufferAttribute(payload.vertices.flat(), 3),
    );
    geo.setAttribute(
      "normal",
      new THREE.Float32BufferAttribute(payload.normals.flat(), 3),
    );
    if (payload.indices.length > 0) geo.setIndex(payload.indices);
  } else if (payload.normals.length > 0) {
    const indices =
      payload.indices.length > 0
        ? payload.indices
        : payload.vertices.map((_, i) => i);
    const positions: number[] = [];
    const normals: number[] = [];
    for (let t = 0; t * 3 < indices.length; t++) {
      const n = payload.normals[t] ?? [0, 0, 1];
      for (let k = 0; k < 3; k++) {
        const v = payload.vertices[indices[t * 3 + k]];
        positions.push(v[0], v[1], v[2]);
        normals.push(n[0], n[1], n[2]);
      }
    }
    geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
    geo.setAttribute("normal", new THREE.Float32BufferAttribute(normals, 3));
  } else {
    geo.setAttribute(
      "position",
      new THREE.Float32BufferAttribute(payload.vertices.flat(), 3),
    );
    if (payload.indices.length > 0) geo.setIndex(payload.indices);
    geo.computeVertexNormals();
  }

  geo.computeBoundingSphere();
  return geo;
}
