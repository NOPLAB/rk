// Working out which flat face of a solid the pointer is on.
//
// The kernel does not hand its B-Rep faces through tessellation — a mesh
// arrives as a triangle soup with nothing saying which face a triangle came
// from. So the face is recovered from the geometry: flood-fill outwards from
// the triangle under the pointer, taking in every neighbour that lies in the
// same plane. That works on imported meshes too, which have no B-Rep at all.
//
// Adjacency is keyed on welded positions, not vertex indices, because part
// meshes arrive flat-shaded (every triangle owning its own three vertices)
// while CAD bodies arrive indexed.

import * as THREE from "three";

/** How far a neighbour's normal may tilt and still count as the same face */
const NORMAL_TOLERANCE = Math.cos((0.5 * Math.PI) / 180);
/** How far off the plane a neighbour's centre may sit, in metres */
const PLANE_TOLERANCE = 1e-5;
/** Positions nearer than this are the same vertex */
const WELD = 1e-6;

export interface FlatFace {
  /** Plane normal in world space */
  normal: THREE.Vector3;
  /** A point on the face, in world space — its area-weighted centre */
  origin: THREE.Vector3;
  /** World-space triangle soup, for drawing the highlight */
  positions: Float32Array;
  /** How many triangles the face turned out to have */
  triangles: number;
}

interface Topology {
  /** Three welded vertex ids per triangle */
  corners: Int32Array;
  /** Welded vertex positions, xyz triples */
  points: Float32Array;
  /** Triangles touching each welded edge */
  byEdge: Map<number, number[]>;
}

/**
 * The flat face containing triangle `faceIndex`, or `null` when the surface
 * curves away immediately — a cylinder's side has no face to sketch on.
 */
export function flatFaceAt(mesh: THREE.Mesh, faceIndex: number): FlatFace | null {
  const topology = topologyOf(mesh);
  const triangleCount = topology.corners.length / 3;
  if (faceIndex < 0 || faceIndex >= triangleCount) return null;

  const seedNormal = triangleNormal(topology, faceIndex);
  if (!seedNormal) return null;
  const seedCentre = triangleCentre(topology, faceIndex);

  const taken = new Set<number>([faceIndex]);
  const queue = [faceIndex];
  while (queue.length > 0) {
    const current = queue.pop()!;
    for (const neighbour of neighboursOf(topology, current)) {
      if (taken.has(neighbour)) continue;
      const normal = triangleNormal(topology, neighbour);
      if (!normal || normal.dot(seedNormal) < NORMAL_TOLERANCE) continue;
      const offset = triangleCentre(topology, neighbour).sub(seedCentre).dot(seedNormal);
      if (Math.abs(offset) > PLANE_TOLERANCE) continue;
      taken.add(neighbour);
      queue.push(neighbour);
    }
  }

  // One lone triangle is as likely to be a facet of a curved surface as a
  // face in its own right; refusing it keeps sketches off cylinder walls
  if (taken.size < 2) return null;

  const positions = new Float32Array(taken.size * 9);
  const centre = new THREE.Vector3();
  let totalArea = 0;
  let slot = 0;
  const corner = new THREE.Vector3();
  for (const index of taken) {
    const [a, b, c] = trianglePoints(topology, index);
    for (const p of [a, b, c]) {
      positions[slot++] = p.x;
      positions[slot++] = p.y;
      positions[slot++] = p.z;
    }
    const area = b.clone().sub(a).cross(c.clone().sub(a)).length() / 2;
    totalArea += area;
    corner.copy(a).add(b).add(c).multiplyScalar(area / 3);
    centre.add(corner);
  }
  if (totalArea <= 0) return null;
  centre.multiplyScalar(1 / totalArea);

  // Everything above is in mesh space; the caller works in world space
  const toWorld = mesh.matrixWorld;
  const normalMatrix = new THREE.Matrix3().getNormalMatrix(toWorld);
  const worldPositions = new Float32Array(positions.length);
  const scratch = new THREE.Vector3();
  for (let i = 0; i < positions.length; i += 3) {
    scratch.set(positions[i], positions[i + 1], positions[i + 2]).applyMatrix4(toWorld);
    worldPositions[i] = scratch.x;
    worldPositions[i + 1] = scratch.y;
    worldPositions[i + 2] = scratch.z;
  }

  return {
    normal: seedNormal.clone().applyMatrix3(normalMatrix).normalize(),
    origin: centre.applyMatrix4(toWorld),
    positions: worldPositions,
    triangles: taken.size,
  };
}

/** Built once per mesh and kept on it — a face pick would otherwise rebuild it per click */
function topologyOf(mesh: THREE.Mesh): Topology {
  const cached = mesh.userData.topology as Topology | undefined;
  if (cached) return cached;

  const position = mesh.geometry.getAttribute("position");
  const index = mesh.geometry.getIndex();
  const count = index ? index.count : position.count;

  const points: number[] = [];
  const lookup = new Map<string, number>();
  const weld = (i: number): number => {
    const x = position.getX(i);
    const y = position.getY(i);
    const z = position.getZ(i);
    const key = `${Math.round(x / WELD)},${Math.round(y / WELD)},${Math.round(z / WELD)}`;
    const found = lookup.get(key);
    if (found !== undefined) return found;
    const id = points.length / 3;
    points.push(x, y, z);
    lookup.set(key, id);
    return id;
  };

  const corners = new Int32Array(count);
  for (let i = 0; i < count; i++) {
    corners[i] = weld(index ? index.getX(i) : i);
  }

  const byEdge = new Map<number, number[]>();
  const vertexCount = points.length / 3;
  for (let t = 0; t * 3 < corners.length; t++) {
    for (let e = 0; e < 3; e++) {
      const key = edgeKey(corners[t * 3 + e], corners[t * 3 + ((e + 1) % 3)], vertexCount);
      const bucket = byEdge.get(key);
      if (bucket) bucket.push(t);
      else byEdge.set(key, [t]);
    }
  }

  const topology: Topology = {
    corners,
    points: new Float32Array(points),
    byEdge,
  };
  mesh.userData.topology = topology;
  return topology;
}

function edgeKey(a: number, b: number, vertexCount: number): number {
  return a < b ? a * vertexCount + b : b * vertexCount + a;
}

function neighboursOf(topology: Topology, triangle: number): number[] {
  const vertexCount = topology.points.length / 3;
  const out: number[] = [];
  for (let e = 0; e < 3; e++) {
    const key = edgeKey(
      topology.corners[triangle * 3 + e],
      topology.corners[triangle * 3 + ((e + 1) % 3)],
      vertexCount,
    );
    for (const other of topology.byEdge.get(key) ?? []) {
      if (other !== triangle) out.push(other);
    }
  }
  return out;
}

function trianglePoints(
  topology: Topology,
  triangle: number,
): [THREE.Vector3, THREE.Vector3, THREE.Vector3] {
  const at = (slot: number) => {
    const v = topology.corners[triangle * 3 + slot] * 3;
    return new THREE.Vector3(
      topology.points[v],
      topology.points[v + 1],
      topology.points[v + 2],
    );
  };
  return [at(0), at(1), at(2)];
}

function triangleNormal(topology: Topology, triangle: number): THREE.Vector3 | null {
  const [a, b, c] = trianglePoints(topology, triangle);
  const normal = b.sub(a).cross(c.sub(a));
  if (normal.lengthSq() < 1e-24) return null;
  return normal.normalize();
}

function triangleCentre(topology: Topology, triangle: number): THREE.Vector3 {
  const [a, b, c] = trianglePoints(topology, triangle);
  return a.add(b).add(c).multiplyScalar(1 / 3);
}
