// Sketches that are not being edited.
//
// Finishing a sketch does not make it disappear: its curves stay in the
// viewport and its enclosed areas stay clickable, which is how an extrude
// gets told what to build. Each sketch owns a child group carrying its own
// plane basis, so children are authored in 2D at z = 0 exactly as the active
// layer does it.
//
// Unlike the active layer these draw with the depth test on, so a solid built
// from a sketch hides the sketch underneath instead of showing through it.

import * as THREE from "three";
import type { SketchEntry, SketchRegion, Vec2 } from "../engine/api";
import { outlinesOf, pushPolyline, RegionFills, setSegments } from "./sketchLayer";

/** A region the pointer resolved to, and the sketch that owns it */
export interface RegionPick {
  sketchId: string;
  regionId: string;
}

interface Entry {
  group: THREE.Group;
  curves: THREE.LineSegments;
  construction: THREE.LineSegments;
  fills: RegionFills;
  regions: SketchRegion[];
}

export class IdleSketches {
  readonly group = new THREE.Group();
  private entries = new Map<string, Entry>();
  private selection: RegionPick[] = [];

  /** Rebuild from the document, leaving `activeId` to the editing layer */
  set(sketches: SketchEntry[], activeId: string | null) {
    this.clear();
    for (const sketch of sketches) {
      if (sketch.id === activeId) continue;
      const group = new THREE.Group();
      group.matrixAutoUpdate = false;
      group.matrix.fromArray(sketch.transform);
      group.matrixWorldNeedsUpdate = true;

      const curves = lineObject(0x6f7d90);
      const construction = lineObject(0x4a515c);
      const solid: number[] = [];
      const dashed: number[] = [];
      for (const outline of outlinesOf(sketch.geometry)) {
        pushPolyline(outline.construction ? dashed : solid, outline.pts, outline.closed);
      }
      setSegments(curves, solid);
      setSegments(construction, dashed);

      const fills = new RegionFills(0x3f6da8, 0.1);
      fills.set(sketch.geometry.regions);

      group.add(fills.group, curves, construction);
      this.group.add(group);
      this.entries.set(sketch.id, {
        group,
        curves,
        construction,
        fills,
        regions: sketch.geometry.regions,
      });
    }
    this.applySelection();
  }

  setVisible(visible: boolean) {
    this.group.visible = visible;
  }

  /** Regions the user has clicked, across every sketch */
  setSelection(selection: RegionPick[]) {
    this.selection = selection;
    this.applySelection();
  }

  setHovered(pick: RegionPick | null) {
    for (const [id, entry] of this.entries) {
      entry.fills.setHovered(pick && pick.sketchId === id ? pick.regionId : null);
    }
  }

  /** The region under the pointer, nearest camera first */
  pick(raycaster: THREE.Raycaster): RegionPick | null {
    const hits = raycaster.intersectObject(this.group, true);
    for (const hit of hits) {
      const regionId = hit.object.userData.regionId as string | undefined;
      if (!regionId) continue;
      const sketchId = this.ownerOf(hit.object);
      if (sketchId) return { sketchId, regionId };
    }
    return null;
  }

  /** Where a region sits in 3D, for aiming a dialog or a camera at it */
  centroid(pick: RegionPick): THREE.Vector3 | null {
    const entry = this.entries.get(pick.sketchId);
    const region = entry?.regions.find((r) => r.id === pick.regionId);
    if (!entry || !region) return null;
    return localToWorld(entry.group, region.centroid);
  }

  regionsOf(sketchId: string): SketchRegion[] {
    return this.entries.get(sketchId)?.regions ?? [];
  }

  private ownerOf(object: THREE.Object3D): string | null {
    for (const [id, entry] of this.entries) {
      let node: THREE.Object3D | null = object;
      while (node) {
        if (node === entry.group) return id;
        node = node.parent;
      }
    }
    return null;
  }

  private applySelection() {
    for (const [id, entry] of this.entries) {
      entry.fills.setSelected(
        this.selection.filter((s) => s.sketchId === id).map((s) => s.regionId),
      );
    }
  }

  private clear() {
    for (const entry of this.entries.values()) {
      this.group.remove(entry.group);
      entry.fills.dispose();
      for (const obj of [entry.curves, entry.construction]) {
        obj.geometry.dispose();
        (obj.material as THREE.Material).dispose();
      }
    }
    this.entries.clear();
  }

  dispose() {
    this.clear();
  }
}

function localToWorld(group: THREE.Group, p: Vec2): THREE.Vector3 {
  return new THREE.Vector3(p[0], p[1], 0).applyMatrix4(group.matrix);
}

function lineObject(color: number): THREE.LineSegments {
  const obj = new THREE.LineSegments(
    new THREE.BufferGeometry(),
    new THREE.LineBasicMaterial({ color }),
  );
  obj.frustumCulled = false;
  return obj;
}
