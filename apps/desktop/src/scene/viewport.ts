// Three.js viewport: everything the 3D view draws and everything it
// resolves a click to.
//
// Scene sync is event-driven — engine events drive
// incremental updates; document_reset triggers a full rebuild from a
// snapshot. RK is Z-up; Three defaults to Y-up, so the grid is rotated
// onto the XY plane and every camera/up vector is Z-up.

import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { TransformControls } from "three/addons/controls/TransformControls.js";
import {
  allSketchGeometry,
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
import { IdleSketches, type RegionPick } from "./idleSketches";
import { PlanePicker, type OriginPlane, type PlanePick } from "./planePicker";
import { SketchLayer, type Projector } from "./sketchLayer";
import type { SketchPreview } from "./sketchTools";
import { AxisTriad, CUBE_MARGIN, CUBE_SIZE, ViewCube } from "./viewCube";

THREE.Object3D.DEFAULT_UP.set(0, 0, 1);

const SELECT_EMISSIVE = new THREE.Color(0x2a5db0);
const BLACK = new THREE.Color(0x000000);

/** Bottom-left axis marker */
const TRIAD_SIZE = 74;
const TRIAD_MARGIN = 8;

/** "none" hides the gizmo and leaves click-selection alone */
export type GizmoMode = "none" | "translate" | "rotate";

/** Camera directions the ViewCube and the ribbon's view commands share */
export const STANDARD_VIEWS = {
  front: [0, -1, 0],
  back: [0, 1, 0],
  left: [-1, 0, 0],
  right: [1, 0, 0],
  top: [0, 0, 1],
  bottom: [0, 0, -1],
  iso: [1, -1, 0.8],
} as const;

export type StandardView = keyof typeof STANDARD_VIEWS;

/** Pointer position resolved against the active sketch */
export interface SketchHit {
  /** Sketch coordinates, snapped onto an existing point when close enough */
  position: Vec2;
  /** The snapped point's ID, if any — reuse it to keep profiles connected */
  pointId: string | null;
  /** Every point within snapping distance, nearest first */
  pointIds: string[];
  /** Entity under the pointer (for the select tool) */
  entityId: string | null;
  /** Enclosed area under the pointer, when the click missed every curve */
  regionId: string | null;
}

/** Camera pose, carried from one Viewport to the next across a dock move */
export interface CameraState {
  position: number[];
  target: number[];
  up: number[];
}

/** What a right-click in the 3D view landed on */
export interface ViewportTarget {
  /** Part under the pointer, when not sketching */
  partId: string | null;
  /** Filled region of a finished sketch, when not sketching */
  region: RegionPick | null;
  /** Where the pointer sits on the active sketch's plane, while sketching */
  sketchHit: SketchHit | null;
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
  private idle = new IdleSketches();
  private planes = new PlanePicker();
  /** Modal "click something in the 3D view" state */
  private pickingPlane = false;
  private sketchInfo: SketchInfo | null = null;
  private grid: THREE.GridHelper;
  private cube = new ViewCube();
  private triad = new AxisTriad();
  /** Drops the canvas listeners on dispose — a dev reload builds a new Viewport
   *  onto the same canvas, and the old one's handlers would linger */
  private listeners = new AbortController();
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
  /** A plane was picked in the 3D view; the pick mode has already ended */
  onPlanePick: ((pick: PlanePick) => void) | null = null;
  /** What the pointer is over while picking a plane (`null` = nothing) */
  onPlaneHover: ((pick: PlanePick | null) => void) | null = null;
  /** A filled region of a finished sketch was clicked (`null` = empty space) */
  onRegionPick: ((pick: RegionPick | null, additive: boolean) => void) | null =
    null;
  /** Right-click, with whatever the pointer was over resolved for the menu */
  onContextMenu: ((target: ViewportTarget, e: MouseEvent) => void) | null =
    null;

  constructor(canvas: HTMLCanvasElement) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    this.renderer.setPixelRatio(window.devicePixelRatio);
    this.scene.background = gradientBackground();

    this.camera = new THREE.PerspectiveCamera(50, 1, 0.001, 1000);
    this.camera.up.set(0, 0, 1);
    this.camera.position.set(0.5, -0.5, 0.35);

    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.target.set(0, 0, 0);

    this.grid = new THREE.GridHelper(10, 100, 0x69707b, 0x3f454e);
    this.grid.rotation.x = Math.PI / 2; // XZ (three default) → XY plane
    this.scene.add(this.grid);
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
    this.scene.add(this.idle.group);
    this.scene.add(this.planes.group);
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
    const { signal } = this.listeners;
    let downAt: [number, number] | null = null;
    canvas.addEventListener(
      "pointerdown",
      (e) => {
        // The gizmo's own listener runs first, so a grabbed handle already
        // shows up as an active axis — that press is not a selection gesture
        if (e.button === 0 && this.gizmo.axis === null) {
          downAt = [e.clientX, e.clientY];
        }
      },
      { signal },
    );
    canvas.addEventListener(
      "pointerup",
      (e) => {
        if (!downAt || e.button !== 0) return;
        const [x0, y0] = downAt;
        downAt = null;
        // A drag orbited the view; only a click edits
        if (Math.hypot(e.clientX - x0, e.clientY - y0) > 4) return;
        // The ViewCube sits on top of the scene, so it gets the click first
        const onCube = this.cubeLocal(e);
        if (onCube) {
          const dir = this.cube.hit(onCube.x, onCube.y);
          if (dir) this.lookFrom(dir);
          return;
        }
        if (this.pickingPlane) {
          const pick = this.planes.probe(
            this.rayThrough(e),
            this.solids(),
            false,
          );
          if (pick) {
            this.setPlanePick(false);
            this.onPlanePick?.(pick);
          }
          return;
        }
        if (this.sketch.active) {
          const hit = this.sketchHit(e);
          if (hit) this.onSketchClick?.(hit, e.shiftKey);
          return;
        }
        // A filled region wins over the solid behind it: clicking one is how a
        // finished sketch is aimed at an extrude
        const region = this.idle.pick(this.rayThrough(e));
        if (region) {
          this.onRegionPick?.(region, e.shiftKey);
          return;
        }
        this.onRegionPick?.(null, e.shiftKey);
        this.onPick?.(this.pick(e));
      },
      { signal },
    );
    canvas.addEventListener(
      "pointermove",
      (e) => {
        const onCube = this.cubeLocal(e);
        this.cube.setHover(onCube ? onCube.x : null, onCube?.y ?? 0);
        if (this.pickingPlane) {
          this.onPlaneHover?.(
            this.planes.probe(this.rayThrough(e), this.solids(), true),
          );
          return;
        }
        if (!this.sketch.active) {
          this.idle.setHovered(this.idle.pick(this.rayThrough(e)));
          return;
        }
        const hit = this.sketchHit(e);
        if (!hit) return;
        this.sketch.setCursor(hit.pointId ? hit.position : null);
        this.onSketchMove?.(hit);
      },
      { signal },
    );
    canvas.addEventListener("pointerleave", () => this.cube.setHover(null, 0), {
      signal,
    });
    // Right-click opens the menu on whatever is under the pointer. The pick
    // runs here rather than in the menu so the caller never needs the camera.
    canvas.addEventListener(
      "contextmenu",
      (e) => {
        e.preventDefault();
        if (!this.onContextMenu) return;
        const sketching = this.sketch.active;
        this.onContextMenu(
          {
            partId: sketching ? null : this.pick(e),
            region: sketching ? null : this.idle.pick(this.rayThrough(e)),
            sketchHit: sketching ? this.sketchHit(e) : null,
          },
          e,
        );
      },
      { signal },
    );

    this.renderer.setAnimationLoop(() => {
      if (this.disposed) return;
      this.controls.update();
      this.renderer.render(this.scene, this.camera);
      this.renderOverlays();
    });
  }

  /**
   * Where the camera is looking. Dragging the 3D view into another dock
   * remounts the canvas and therefore builds a new Viewport, and coming back
   * to a reset camera would feel like the model had moved.
   */
  cameraState(): CameraState {
    return {
      position: this.camera.position.toArray(),
      target: this.controls.target.toArray(),
      up: this.camera.up.toArray(),
    };
  }

  restoreCamera(state: CameraState) {
    this.camera.position.fromArray(state.position);
    this.camera.up.fromArray(state.up);
    this.controls.target.fromArray(state.target);
    this.camera.lookAt(this.controls.target);
    this.controls.update();
  }

  resize(width: number, height: number) {
    if (width <= 0 || height <= 0) return;
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / height;
    this.camera.updateProjectionMatrix();
  }

  dispose() {
    this.disposed = true;
    this.listeners.abort();
    this.renderer.setAnimationLoop(null);
    this.gizmo.detach();
    this.gizmo.dispose();
    this.controls.dispose();
    this.sketch.dispose();
    this.idle.dispose();
    this.planes.dispose();
    this.cube.dispose();
    this.triad.dispose();
    this.setCollisions([]);
    this.collisionMaterial.dispose();
    this.clearAll();
    this.renderer.dispose();
  }

  // ---- overlays & standard views ----------------------------------------

  /** Draw the ViewCube and axis triad into their corners of the canvas */
  private renderOverlays() {
    const size = this.renderer.getSize(new THREE.Vector2());
    this.renderer.autoClear = false;

    this.cube.sync(this.camera, this.controls.target);
    // Three's viewport origin is bottom-left; the cube reads as top-right
    this.corner(
      size.x - CUBE_SIZE - CUBE_MARGIN,
      size.y - CUBE_SIZE - CUBE_MARGIN,
      CUBE_SIZE,
    );
    this.renderer.render(this.cube.scene, this.cube.camera);

    this.triad.sync(this.camera, this.controls.target);
    this.corner(TRIAD_MARGIN, TRIAD_MARGIN, TRIAD_SIZE);
    this.renderer.render(this.triad.scene, this.triad.camera);

    this.renderer.setScissorTest(false);
    this.renderer.setViewport(0, 0, size.x, size.y);
    this.renderer.autoClear = true;
  }

  private corner(x: number, y: number, size: number) {
    this.renderer.setViewport(x, y, size, size);
    this.renderer.setScissor(x, y, size, size);
    this.renderer.setScissorTest(true);
    this.renderer.clearDepth();
  }

  /** Pointer position inside the ViewCube's square, or `null` if outside */
  private cubeLocal(e: PointerEvent): { x: number; y: number } | null {
    const rect = this.renderer.domElement.getBoundingClientRect();
    const x = e.clientX - rect.left - (rect.width - CUBE_SIZE - CUBE_MARGIN);
    const y = e.clientY - rect.top - CUBE_MARGIN;
    const inside = x >= 0 && y >= 0 && x <= CUBE_SIZE && y <= CUBE_SIZE;
    return inside ? { x, y } : null;
  }

  /** Orbit to look from `dir` towards the current target, keeping the zoom */
  lookFrom(dir: THREE.Vector3 | readonly [number, number, number]) {
    const v = (
      Array.isArray(dir)
        ? new THREE.Vector3(...dir)
        : (dir as THREE.Vector3).clone()
    ).normalize();
    const dist = Math.max(
      this.camera.position.distanceTo(this.controls.target),
      0.05,
    );
    // Straight down Z would leave `up` parallel to the view direction
    if (Math.abs(v.z) > 0.99) this.camera.up.set(0, 1, 0);
    else this.camera.up.set(0, 0, 1);
    this.camera.position.copy(
      this.controls.target.clone().add(v.multiplyScalar(dist)),
    );
    this.camera.lookAt(this.controls.target);
    this.controls.update();
  }

  setStandardView(view: StandardView) {
    this.lookFrom(STANDARD_VIEWS[view]);
  }

  /** Isometric view framing everything in the scene */
  homeView() {
    this.camera.up.set(0, 0, 1);
    if (!this.fitCamera()) {
      this.controls.target.set(0, 0, 0);
      this.camera.position.set(0.5, -0.5, 0.35);
      this.camera.lookAt(this.controls.target);
    }
    this.controls.update();
  }

  setGridVisible(visible: boolean) {
    this.grid.visible = visible;
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
    // Sketch mode and plane picking own the pointer; a gizmo on top of them
    // would fight for clicks
    if (
      this.gizmoMode === "none" ||
      this.sketch.active ||
      this.pickingPlane ||
      !mesh
    ) {
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

  // ---- picking a plane --------------------------------------------------

  /** Start (or abandon) the modal "click a plane or a face" interaction */
  setPlanePick(active: boolean) {
    this.pickingPlane = active;
    if (active) this.planes.begin(this.sceneBounds());
    else this.planes.end();
    this.renderer.domElement.style.cursor = active ? "crosshair" : "";
    this.attachGizmo();
  }

  get pickingPlaneActive(): boolean {
    return this.pickingPlane;
  }

  /** Light up an origin plane from outside — the browser tree hovers them */
  setPlaneHighlight(which: OriginPlane | null) {
    this.planes.setHighlight(which);
  }

  /** Everything a face pick may land on */
  private solids(): THREE.Mesh[] {
    return [...this.bodyMeshes.values(), ...this.partMeshes.values()];
  }

  private rayThrough(e: MouseEvent): THREE.Raycaster {
    const rect = this.renderer.domElement.getBoundingClientRect();
    this.raycaster.setFromCamera(
      new THREE.Vector2(
        ((e.clientX - rect.left) / rect.width) * 2 - 1,
        -((e.clientY - rect.top) / rect.height) * 2 + 1,
      ),
      this.camera,
    );
    return this.raycaster;
  }

  private sceneBounds(): THREE.Box3 | null {
    const box = new THREE.Box3();
    let any = false;
    for (const mesh of this.solids()) {
      box.expandByObject(mesh);
      any = true;
    }
    return any && !box.isEmpty() ? box : null;
  }

  // ---- sketch mode ------------------------------------------------------

  /** Enter sketch mode on `info`, or leave it with `null` */
  async setSketch(info: SketchInfo | null) {
    const entered = info !== null && info.id !== this.sketchInfo?.id;
    this.sketchInfo = info;
    this.sketch.setSketch(info);
    this.attachGizmo();
    await this.refreshIdleSketches();
    if (!info) {
      this.camera.up.set(0, 0, 1);
      return;
    }
    await this.refreshSketch();
    if (entered) this.alignToSketch();
  }

  /** Which sketch is being edited, if any */
  get activeSketchId(): string | null {
    return this.sketch.sketchId;
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

  /** Redraw every sketch that is not being edited */
  async refreshIdleSketches() {
    try {
      this.idle.set(await allSketchGeometry(), this.sketch.sketchId);
    } catch (e) {
      console.warn("sketch geometry:", e);
    }
  }

  setSketchesVisible(visible: boolean) {
    this.idle.setVisible(visible);
  }

  /** Regions picked for an extrude, in the active sketch and outside it */
  setRegionSelection(selection: RegionPick[]) {
    this.idle.setSelection(selection);
    const active = this.sketch.sketchId;
    this.sketch.setSelectedRegions(
      active
        ? selection.filter((s) => s.sketchId === active).map((s) => s.regionId)
        : [],
    );
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

  private sketchHit(e: MouseEvent): SketchHit | null {
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
    const entityId = this.sketch.pick(px, py, project);
    return {
      position: snapped.position,
      pointId: snapped.pointId,
      pointIds: snapped.nearby,
      entityId,
      // Curves win; the region is what a click on open space inside one means
      regionId: entityId ? null : this.sketch.pickRegion(raw),
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
          await this.refreshIdleSketches();
          break;
        case "sketch_added":
        case "sketch_removed":
          await this.refreshIdleSketches();
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
    await this.refreshIdleSketches();
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

  /** Frame everything; `false` when the scene is empty and nothing moved */
  fitCamera(): boolean {
    const box = new THREE.Box3();
    let any = false;
    for (const mesh of [
      ...this.partMeshes.values(),
      ...this.bodyMeshes.values(),
    ]) {
      box.expandByObject(mesh);
      any = true;
    }
    if (!any || box.isEmpty()) return false;
    const center = box.getCenter(new THREE.Vector3());
    const radius = Math.max(
      box.getSize(new THREE.Vector3()).length() / 2,
      0.05,
    );
    const dir = new THREE.Vector3(1, -1, 0.7).normalize();
    this.camera.position.copy(
      center.clone().add(dir.multiplyScalar(radius * 2.5)),
    );
    this.controls.target.copy(center);
    return true;
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
      mesh.userData.bodyId = bodyId;
      this.bodyMeshes.set(bodyId, mesh);
      this.scene.add(mesh);
    } catch (e) {
      console.warn(`skipping body ${bodyId}:`, e);
    }
  }

  private pick(e: MouseEvent): string | null {
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

/** Vertical gradient behind the model, the way Inventor lights its canvas */
function gradientBackground(): THREE.Texture {
  const canvas = document.createElement("canvas");
  canvas.width = 2;
  canvas.height = 256;
  const g = canvas.getContext("2d")!;
  const gradient = g.createLinearGradient(0, 0, 0, 256);
  gradient.addColorStop(0, "#2a2f38");
  gradient.addColorStop(1, "#525c6b");
  g.fillStyle = gradient;
  g.fillRect(0, 0, 2, 256);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

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
    geo.setAttribute(
      "position",
      new THREE.Float32BufferAttribute(positions, 3),
    );
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
