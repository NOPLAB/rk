import { useMemo } from "react";
import * as THREE from "three";
import type { InertiaMatrix, PartInfo, Rgba, RunCommands } from "../engine/api";
import {
  renamePart,
  setPartColor,
  setPartInertia,
  setPartMass,
  setPartTransform,
} from "../engine/commands";

/** Inertia tensor components, in the order URDF writes them */
const INERTIA_KEYS: (keyof InertiaMatrix)[] = [
  "ixx",
  "ixy",
  "ixz",
  "iyy",
  "iyz",
  "izz",
];

interface Props {
  part: PartInfo | null;
  run: RunCommands;
}

const RAD2DEG = 180 / Math.PI;
const DEG2RAD = Math.PI / 180;

export function PropertiesPanel({ part, run }: Props) {
  const pose = useMemo(() => {
    if (!part) return null;
    const m = new THREE.Matrix4().fromArray(part.origin_transform);
    const pos = new THREE.Vector3();
    const quat = new THREE.Quaternion();
    const scale = new THREE.Vector3();
    m.decompose(pos, quat, scale);
    const euler = new THREE.Euler().setFromQuaternion(quat, "XYZ");
    return { pos, euler, scale };
  }, [part]);

  if (!part || !pose) {
    return <div className="empty">Nothing selected</div>;
  }

  const commitPose = (patch: Partial<Record<PoseKey, number>>) => {
    const v = {
      x: patch.x ?? pose.pos.x,
      y: patch.y ?? pose.pos.y,
      z: patch.z ?? pose.pos.z,
      rx: patch.rx ?? pose.euler.x * RAD2DEG,
      ry: patch.ry ?? pose.euler.y * RAD2DEG,
      rz: patch.rz ?? pose.euler.z * RAD2DEG,
    };
    const quat = new THREE.Quaternion().setFromEuler(
      new THREE.Euler(v.rx * DEG2RAD, v.ry * DEG2RAD, v.rz * DEG2RAD, "XYZ"),
    );
    const m = new THREE.Matrix4().compose(
      new THREE.Vector3(v.x, v.y, v.z),
      quat,
      pose.scale,
    );
    void run([setPartTransform(part.id, m.toArray())]);
  };

  return (
    <div className="properties">
      <label className="field">
        <span>Name</span>
        <input
          key={`${part.id}:${part.name}`}
          type="text"
          defaultValue={part.name}
          onKeyDown={blurOnEnter}
          onBlur={(e) => {
            const name = e.currentTarget.value.trim();
            if (name && name !== part.name) void run([renamePart(part.id, name)]);
          }}
        />
      </label>

      <label className="field">
        <span>Color</span>
        <input
          key={`${part.id}:${rgbaToHex(part.color)}`}
          type="color"
          defaultValue={rgbaToHex(part.color)}
          onBlur={(e) => {
            const hex = e.currentTarget.value;
            if (hex !== rgbaToHex(part.color)) {
              const [r, g, b] = hexToRgb(hex);
              void run([setPartColor(part.id, [r, g, b, part.color[3]])]);
            }
          }}
        />
      </label>

      <div className="field-group">Position (m)</div>
      <NumField id={part.id} label="X" value={pose.pos.x} onCommit={(x) => commitPose({ x })} />
      <NumField id={part.id} label="Y" value={pose.pos.y} onCommit={(y) => commitPose({ y })} />
      <NumField id={part.id} label="Z" value={pose.pos.z} onCommit={(z) => commitPose({ z })} />

      <div className="field-group">Rotation (deg)</div>
      <NumField id={part.id} label="RX" value={pose.euler.x * RAD2DEG} onCommit={(rx) => commitPose({ rx })} />
      <NumField id={part.id} label="RY" value={pose.euler.y * RAD2DEG} onCommit={(ry) => commitPose({ ry })} />
      <NumField id={part.id} label="RZ" value={pose.euler.z * RAD2DEG} onCommit={(rz) => commitPose({ rz })} />

      <div className="field-group">Physics</div>
      <NumField
        id={part.id}
        label="Mass"
        value={part.mass}
        step="0.01"
        onCommit={(mass) => void run([setPartMass(part.id, mass)])}
      />
      {INERTIA_KEYS.map((key) => (
        <NumField
          key={key}
          id={part.id}
          label={key}
          value={part.inertia[key]}
          step="0.0001"
          onCommit={(v) =>
            void run([setPartInertia(part.id, { ...part.inertia, [key]: v })])
          }
        />
      ))}

      <div className="field-group">ID</div>
      <div className="mono small">{part.id}</div>
    </div>
  );
}

type PoseKey = "x" | "y" | "z" | "rx" | "ry" | "rz";

function NumField({
  id,
  label,
  value,
  step = "0.01",
  onCommit,
}: {
  id: string;
  label: string;
  value: number;
  step?: string;
  onCommit: (v: number) => void;
}) {
  const rounded = Math.round(value * 1e6) / 1e6;
  return (
    <label className="field">
      <span>{label}</span>
      <input
        key={`${id}:${label}:${rounded}`}
        type="number"
        step={step}
        defaultValue={rounded}
        onKeyDown={blurOnEnter}
        onBlur={(e) => {
          const v = parseFloat(e.currentTarget.value);
          if (Number.isFinite(v) && Math.abs(v - rounded) > 1e-9) onCommit(v);
        }}
      />
    </label>
  );
}

function blurOnEnter(e: React.KeyboardEvent<HTMLInputElement>) {
  if (e.key === "Enter") e.currentTarget.blur();
}

function rgbaToHex(color: Rgba): string {
  const to = (c: number) =>
    Math.round(Math.min(Math.max(c, 0), 1) * 255)
      .toString(16)
      .padStart(2, "0");
  return `#${to(color[0])}${to(color[1])}${to(color[2])}`;
}

function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}
