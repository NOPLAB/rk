import { useState } from "react";
import * as THREE from "three";
import type {
  CollisionInfo,
  GeometryType,
  PartInfo,
  Pose,
  RunCommands,
  SceneSnapshot,
} from "../engine/api";
import {
  addCollision,
  identityPose,
  removeCollision,
  setCollisionGeometry,
  setCollisionOrigin,
} from "../engine/commands";
import { MiniNum } from "./MiniNum";

type ShapeKind = "Box" | "Cylinder" | "Sphere" | "Capsule";

const SHAPES: ShapeKind[] = ["Box", "Cylinder", "Sphere", "Capsule"];

/** Sizes are edited in mm; the engine works in meters */
const M2MM = 1000;
const RAD2DEG = 180 / Math.PI;
const DEG2RAD = Math.PI / 180;

const defaultGeometry = (kind: ShapeKind): GeometryType => {
  switch (kind) {
    case "Box":
      return { Box: { size: [0.05, 0.05, 0.05] } };
    case "Cylinder":
      return { Cylinder: { radius: 0.025, length: 0.05 } };
    case "Sphere":
      return { Sphere: { radius: 0.025 } };
    case "Capsule":
      return { Capsule: { radius: 0.02, length: 0.05 } };
  }
};

const kindOf = (g: GeometryType): string => Object.keys(g)[0];

interface Props {
  snapshot: SceneSnapshot | null;
  /** Collisions live on the link that owns the selected part */
  part: PartInfo | null;
  visible: boolean;
  onVisible: (visible: boolean) => void;
  run: RunCommands;
}

export function CollisionPanel({
  snapshot,
  part,
  visible,
  onVisible,
  run,
}: Props) {
  const [shape, setShape] = useState<ShapeKind>("Box");

  if (!snapshot) return null;
  const link = part
    ? (snapshot.links.find((l) => l.part_id === part.id) ?? null)
    : null;

  const show = (
    <label className="check">
      <input
        type="checkbox"
        checked={visible}
        onChange={(e) => onVisible(e.currentTarget.checked)}
      />
      <span>Show collisions</span>
    </label>
  );

  if (!part) {
    return (
      <div className="collisions">
        {show}
        <div className="small">Select a part</div>
      </div>
    );
  }
  if (!link) {
    // Links appear when a part joins the assembly; there is no standalone
    // "create link" command
    return (
      <div className="collisions">
        {show}
        <div className="small">
          {part.name} has no link yet — connect it to a joint first
        </div>
      </div>
    );
  }

  const size = [
    part.bbox_max[0] - part.bbox_min[0],
    part.bbox_max[1] - part.bbox_min[1],
    part.bbox_max[2] - part.bbox_min[2],
  ] as [number, number, number];
  const canFit = size.every((s) => s > 1e-6);

  /**
   * Box matching the part's bounding box. The box stays axis-aligned in the
   * link frame, so a rotated part origin gives an approximate fit.
   */
  const onFit = () => {
    const center = new THREE.Vector3(
      (part.bbox_min[0] + part.bbox_max[0]) / 2,
      (part.bbox_min[1] + part.bbox_max[1]) / 2,
      (part.bbox_min[2] + part.bbox_max[2]) / 2,
    ).applyMatrix4(new THREE.Matrix4().fromArray(part.origin_transform));
    void run([
      addCollision(
        link.id,
        { Box: { size } },
        { xyz: [center.x, center.y, center.z], rpy: [0, 0, 0] },
      ),
    ]);
  };

  return (
    <div className="collisions">
      {show}
      <div className="tool-row">
        <select
          value={shape}
          onChange={(e) => setShape(e.target.value as ShapeKind)}
        >
          {SHAPES.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <button
          onClick={() =>
            void run([addCollision(link.id, defaultGeometry(shape))])
          }
        >
          Add
        </button>
        <button
          disabled={!canFit}
          title="Add a box matching the part's bounding box"
          onClick={onFit}
        >
          Fit
        </button>
      </div>

      {link.collisions.length === 0 && <div className="empty">None</div>}
      {link.collisions.map((c) => (
        <CollisionCard
          key={`${link.id}:${c.index}:${kindOf(c.geometry)}`}
          linkId={link.id}
          collision={c}
          run={run}
        />
      ))}
    </div>
  );
}

function CollisionCard({
  linkId,
  collision,
  run,
}: {
  linkId: string;
  collision: CollisionInfo;
  run: RunCommands;
}) {
  const { index, geometry, origin } = collision;

  const setGeometry = (g: GeometryType) =>
    void run([setCollisionGeometry(linkId, index, g)]);
  const setOrigin = (patch: Partial<Pose>) =>
    void run([setCollisionOrigin(linkId, index, { ...origin, ...patch })]);

  const xyz = (i: number, v: number) => {
    const next = [...origin.xyz] as [number, number, number];
    next[i] = v / M2MM;
    setOrigin({ xyz: next });
  };
  const rpy = (i: number, v: number) => {
    const next = [...origin.rpy] as [number, number, number];
    next[i] = v * DEG2RAD;
    setOrigin({ rpy: next });
  };

  return (
    <div className="joint-card">
      <div className="joint-head">
        <span className="joint-name">{kindOf(geometry)}</span>
        <button
          className="mini"
          title="Remove collision"
          onClick={() => void run([removeCollision(linkId, index)])}
        >
          ✕
        </button>
      </div>

      <div className="triple-row">
        <span>Size</span>
        {"Box" in geometry &&
          geometry.Box.size.map((s, i) => (
            <MiniNum
              key={`box${i}:${s}`}
              value={s * M2MM}
              step={1}
              title="mm"
              onCommit={(v) => {
                const next = [...geometry.Box.size] as [number, number, number];
                next[i] = v / M2MM;
                setGeometry({ Box: { size: next } });
              }}
            />
          ))}
        {"Sphere" in geometry && (
          <MiniNum
            key={`r:${geometry.Sphere.radius}`}
            value={geometry.Sphere.radius * M2MM}
            step={1}
            title="radius (mm)"
            onCommit={(v) => setGeometry({ Sphere: { radius: v / M2MM } })}
          />
        )}
        {"Cylinder" in geometry && (
          <>
            <MiniNum
              key={`cr:${geometry.Cylinder.radius}`}
              value={geometry.Cylinder.radius * M2MM}
              step={1}
              title="radius (mm)"
              onCommit={(v) =>
                setGeometry({
                  Cylinder: { ...geometry.Cylinder, radius: v / M2MM },
                })
              }
            />
            <MiniNum
              key={`cl:${geometry.Cylinder.length}`}
              value={geometry.Cylinder.length * M2MM}
              step={1}
              title="length (mm)"
              onCommit={(v) =>
                setGeometry({
                  Cylinder: { ...geometry.Cylinder, length: v / M2MM },
                })
              }
            />
          </>
        )}
        {"Capsule" in geometry && (
          <>
            <MiniNum
              key={`kr:${geometry.Capsule.radius}`}
              value={geometry.Capsule.radius * M2MM}
              step={1}
              title="radius (mm)"
              onCommit={(v) =>
                setGeometry({
                  Capsule: { ...geometry.Capsule, radius: v / M2MM },
                })
              }
            />
            <MiniNum
              key={`kl:${geometry.Capsule.length}`}
              value={geometry.Capsule.length * M2MM}
              step={1}
              title="length (mm)"
              onCommit={(v) =>
                setGeometry({
                  Capsule: { ...geometry.Capsule, length: v / M2MM },
                })
              }
            />
          </>
        )}
        {"Mesh" in geometry && <span className="small">imported mesh</span>}
      </div>

      <div className="triple-row">
        <span>XYZ</span>
        {origin.xyz.map((v, i) => (
          <MiniNum
            key={`x${i}:${v}`}
            value={v * M2MM}
            step={1}
            title="mm"
            onCommit={(mm) => xyz(i, mm)}
          />
        ))}
      </div>
      <div className="triple-row">
        <span>RPY</span>
        {origin.rpy.map((v, i) => (
          <MiniNum
            key={`r${i}:${v}`}
            value={v * RAD2DEG}
            step={5}
            title="degrees"
            onCommit={(deg) => rpy(i, deg)}
          />
        ))}
      </div>

      <div className="joint-actions">
        <button
          className="mini"
          title="Reset origin"
          onClick={() => void run([setCollisionOrigin(linkId, index, identityPose())])}
        >
          Reset origin
        </button>
      </div>
    </div>
  );
}
