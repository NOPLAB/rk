// Viewport overlays drawn on top of the model: Inventor's ViewCube (top
// right) and the axis triad (bottom left).
//
// Both render into a scissored corner of the main canvas rather than their
// own WebGL context, so they cost one extra draw call and no second GPU
// device. RK is Z-up, so +Z is TOP and -Y is FRONT — the same convention
// URDF uses.

import * as THREE from "three";

/** Size of the ViewCube's square, in CSS pixels */
export const CUBE_SIZE = 92;
/** Margin from the top-right corner of the viewport */
export const CUBE_MARGIN = 10;

const HALF = 0.72;
/** How far off-centre a hit has to be to pull in an extra axis (edge/corner) */
const EDGE_BAND = 0.6;

const FACE_FILL = "#c2c8d1";
const FACE_LINE = "#767e8a";
const FACE_TEXT = "#22262c";

/** BoxGeometry material order: +X, -X, +Y, -Y, +Z, -Z */
const FACE_LABELS = ["RIGHT", "LEFT", "BACK", "FRONT", "TOP", "BOTTOM"];

/**
 * Rotation (in quarter turns) applied to each face's label so it reads
 * upright when the cube is seen from that side with Z pointing up.
 *
 * Three builds box faces for a Y-up world: the ±X faces put their texture's
 * top along +Y and the +Y face puts it along -Z, none of which is "up" here.
 */
const FACE_SPIN = [3, 1, 2, 0, 0, 0];

export class ViewCube {
  readonly scene = new THREE.Scene();
  readonly camera = new THREE.OrthographicCamera(-1.15, 1.15, 1.15, -1.15, 0.1, 20);
  private mesh: THREE.Mesh;
  private materials: THREE.MeshBasicMaterial[];
  private raycaster = new THREE.Raycaster();
  private hovered: number | null = null;

  constructor() {
    this.materials = FACE_LABELS.map((label, i) => {
      const map = faceTexture(label, FACE_SPIN[i]);
      return new THREE.MeshBasicMaterial({ map });
    });
    this.mesh = new THREE.Mesh(
      new THREE.BoxGeometry(HALF * 2, HALF * 2, HALF * 2),
      this.materials,
    );
    this.scene.add(this.mesh);

    const edges = new THREE.LineSegments(
      new THREE.EdgesGeometry(this.mesh.geometry),
      new THREE.LineBasicMaterial({ color: 0x5c646f }),
    );
    this.scene.add(edges);
  }

  /** Point the cube's camera the same way the model camera looks */
  sync(camera: THREE.Camera, target: THREE.Vector3) {
    const dir = camera.position.clone().sub(target);
    if (dir.lengthSq() < 1e-12) dir.set(0, -1, 0);
    this.camera.position.copy(dir.normalize().multiplyScalar(5));
    this.camera.up.copy(camera.up);
    this.camera.lookAt(0, 0, 0);
  }

  /**
   * Which direction the cube would send the camera, for a pointer at
   * `(x, y)` inside the cube's square (top-left origin). `null` if the
   * pointer missed the cube.
   *
   * Faces, edges and corners all fall out of the hit position: an axis
   * joins the direction once the hit is far enough along it.
   */
  hit(x: number, y: number): THREE.Vector3 | null {
    const ndc = new THREE.Vector2(
      (x / CUBE_SIZE) * 2 - 1,
      -(y / CUBE_SIZE) * 2 + 1,
    );
    this.raycaster.setFromCamera(ndc, this.camera);
    const hit = this.raycaster.intersectObject(this.mesh, false)[0];
    if (!hit) return null;
    const q = hit.point.clone().divideScalar(HALF);
    const dir = new THREE.Vector3(
      Math.abs(q.x) > EDGE_BAND ? Math.sign(q.x) : 0,
      Math.abs(q.y) > EDGE_BAND ? Math.sign(q.y) : 0,
      Math.abs(q.z) > EDGE_BAND ? Math.sign(q.z) : 0,
    );
    return dir.lengthSq() === 0 ? null : dir.normalize();
  }

  /** Light up the face under the pointer; pass `null` to clear */
  setHover(x: number | null, y: number) {
    let face: number | null = null;
    if (x !== null) {
      const ndc = new THREE.Vector2(
        (x / CUBE_SIZE) * 2 - 1,
        -(y / CUBE_SIZE) * 2 + 1,
      );
      this.raycaster.setFromCamera(ndc, this.camera);
      const hit = this.raycaster.intersectObject(this.mesh, false)[0];
      face = hit?.face?.materialIndex ?? null;
    }
    if (face === this.hovered) return;
    if (this.hovered !== null) {
      this.materials[this.hovered].color.setHex(0xffffff);
    }
    if (face !== null) this.materials[face].color.setHex(0x8fc4f0);
    this.hovered = face;
  }

  dispose() {
    this.mesh.geometry.dispose();
    for (const m of this.materials) {
      m.map?.dispose();
      m.dispose();
    }
  }
}

/** The little XYZ marker in the bottom-left corner */
export class AxisTriad {
  readonly scene = new THREE.Scene();
  readonly camera = new THREE.OrthographicCamera(-1.5, 1.5, 1.5, -1.5, 0.1, 20);
  private sprites: THREE.Sprite[] = [];
  private lines: THREE.LineSegments;

  constructor() {
    const axes: [THREE.Vector3, number, string][] = [
      [new THREE.Vector3(1, 0, 0), 0xe0575f, "X"],
      [new THREE.Vector3(0, 1, 0), 0x7ac555, "Y"],
      [new THREE.Vector3(0, 0, 1), 0x5b9bea, "Z"],
    ];
    const positions: number[] = [];
    const colors: number[] = [];
    for (const [dir, hex] of axes) {
      const c = new THREE.Color(hex);
      positions.push(0, 0, 0, dir.x, dir.y, dir.z);
      colors.push(c.r, c.g, c.b, c.r, c.g, c.b);
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
    geo.setAttribute("color", new THREE.Float32BufferAttribute(colors, 3));
    this.lines = new THREE.LineSegments(
      geo,
      new THREE.LineBasicMaterial({ vertexColors: true }),
    );
    this.scene.add(this.lines);

    for (const [dir, hex, label] of axes) {
      const sprite = new THREE.Sprite(
        new THREE.SpriteMaterial({ map: labelTexture(label, hex) }),
      );
      sprite.position.copy(dir).multiplyScalar(1.32);
      sprite.scale.setScalar(0.66);
      this.sprites.push(sprite);
      this.scene.add(sprite);
    }
  }

  sync(camera: THREE.Camera, target: THREE.Vector3) {
    const dir = camera.position.clone().sub(target);
    if (dir.lengthSq() < 1e-12) dir.set(0, -1, 0);
    this.camera.position.copy(dir.normalize().multiplyScalar(5));
    this.camera.up.copy(camera.up);
    this.camera.lookAt(0, 0, 0);
  }

  dispose() {
    this.lines.geometry.dispose();
    (this.lines.material as THREE.Material).dispose();
    for (const s of this.sprites) {
      const mat = s.material as THREE.SpriteMaterial;
      mat.map?.dispose();
      mat.dispose();
    }
  }
}

function faceTexture(label: string, quarterTurns: number): THREE.CanvasTexture {
  const size = 128;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  const g = canvas.getContext("2d")!;
  g.fillStyle = FACE_FILL;
  g.fillRect(0, 0, size, size);
  g.strokeStyle = FACE_LINE;
  g.lineWidth = 5;
  g.strokeRect(2.5, 2.5, size - 5, size - 5);

  g.translate(size / 2, size / 2);
  g.rotate((quarterTurns * Math.PI) / 2);
  g.fillStyle = FACE_TEXT;
  g.font = "600 26px 'Segoe UI', system-ui, sans-serif";
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.fillText(label, 0, 0);

  const texture = new THREE.CanvasTexture(canvas);
  texture.anisotropy = 4;
  return texture;
}

function labelTexture(label: string, hex: number): THREE.CanvasTexture {
  const size = 64;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  const g = canvas.getContext("2d")!;
  g.fillStyle = `#${hex.toString(16).padStart(6, "0")}`;
  g.font = "700 40px 'Segoe UI', system-ui, sans-serif";
  g.textAlign = "center";
  g.textBaseline = "middle";
  g.fillText(label, size / 2, size / 2);
  return new THREE.CanvasTexture(canvas);
}
