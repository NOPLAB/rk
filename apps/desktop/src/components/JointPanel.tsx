import { useRef, useState } from "react";
import type {
  Command,
  JointInfo,
  JointType,
  RunCommands,
  SceneSnapshot,
} from "../engine/api";
import { createCoalescer } from "../engine/interaction";
import {
  connectParts,
  disconnectPart,
  resetAllJointPositions,
  resetJointPosition,
  setJointAxis,
  setJointLimits,
  setJointPosition,
  setJointType,
} from "../engine/commands";

const JOINT_TYPES: JointType[] = [
  "Fixed",
  "Revolute",
  "Continuous",
  "Prismatic",
  "Floating",
  "Planar",
];

const RAD2DEG = 180 / Math.PI;

const isAngular = (t: JointType) => t === "Revolute" || t === "Continuous";
const hasPosition = (t: JointType) =>
  t === "Revolute" || t === "Continuous" || t === "Prismatic";
const hasLimits = (t: JointType) => t === "Revolute" || t === "Prismatic";

interface Props {
  snapshot: SceneSnapshot | null;
  run: RunCommands;
}

export function JointPanel({ snapshot, run }: Props) {
  const [parent, setParent] = useState("");
  const [child, setChild] = useState("");

  // Slider drags outrun the IPC round trip; only the newest value matters
  const slider = useRef(createCoalescer());
  const send = (cmd: Command) =>
    slider.current.push(async () => {
      await run([cmd]);
    });

  if (!snapshot) return null;
  const { parts, joints, links } = snapshot;

  const partName = (partId: string | null, linkId: string) =>
    (partId && parts.find((p) => p.id === partId)?.name) ||
    links.find((l) => l.id === linkId)?.name ||
    "?";

  const canConnect = parent !== "" && child !== "" && parent !== child;

  const onConnect = async () => {
    if (!canConnect) return;
    await run([connectParts(parent, child)]);
    setParent("");
    setChild("");
  };

  return (
    <div className="joints">
      <div className="connect-row">
        <select value={parent} onChange={(e) => setParent(e.target.value)}>
          <option value="">parent…</option>
          {parts.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
        <select value={child} onChange={(e) => setChild(e.target.value)}>
          <option value="">child…</option>
          {parts.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
        <button disabled={!canConnect} onClick={() => void onConnect()}>
          Connect
        </button>
      </div>

      {joints.length === 0 && <div className="empty">No joints</div>}
      {joints.map((j) => (
        <JointCard
          key={j.id}
          joint={j}
          parentName={partName(j.parent_part, j.parent_link)}
          childName={partName(j.child_part, j.child_link)}
          run={run}
          send={send}
        />
      ))}

      {joints.length > 1 && (
        <div className="joint-actions">
          <button onClick={() => void run([resetAllJointPositions()])}>
            Reset All Positions
          </button>
        </div>
      )}
    </div>
  );
}

function JointCard({
  joint,
  parentName,
  childName,
  run,
  send,
}: {
  joint: JointInfo;
  parentName: string;
  childName: string;
  run: RunCommands;
  send: (cmd: Command) => void;
}) {
  return (
    <div className="joint-card">
      <div className="joint-head">
        <span className="joint-name" title={joint.name}>
          {parentName} → {childName}
        </span>
        <select
          value={joint.joint_type}
          onChange={(e) =>
            void run([setJointType(joint.id, e.target.value as JointType)])
          }
        >
          {JOINT_TYPES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
      </div>

      {hasPosition(joint.joint_type) && (
        <PositionSlider joint={joint} run={run} send={send} />
      )}

      {hasPosition(joint.joint_type) && (
        <AxisFields joint={joint} run={run} />
      )}

      {hasLimits(joint.joint_type) && <LimitFields joint={joint} run={run} />}

      <div className="joint-actions">
        <button
          disabled={!joint.child_part}
          onClick={() =>
            joint.child_part && void run([disconnectPart(joint.child_part)])
          }
        >
          Disconnect
        </button>
      </div>
    </div>
  );
}

function PositionSlider({
  joint,
  run,
  send,
}: {
  joint: JointInfo;
  run: RunCommands;
  send: (cmd: Command) => void;
}) {
  // Local value while dragging; falls back to the engine value on release
  const [drag, setDrag] = useState<number | null>(null);
  const angular = isAngular(joint.joint_type);
  const min = joint.limits?.lower ?? (angular ? -Math.PI : -0.5);
  const max = joint.limits?.upper ?? (angular ? Math.PI : 0.5);
  const value = drag ?? joint.position;
  const label = angular
    ? `${(value * RAD2DEG).toFixed(1)}°`
    : `${value.toFixed(3)} m`;

  return (
    <div className="slider-row">
      <input
        type="range"
        min={min}
        max={max}
        step={(max - min) / 400}
        value={value}
        onChange={(e) => {
          const v = parseFloat(e.currentTarget.value);
          setDrag(v);
          send(setJointPosition(joint.id, v));
        }}
        onPointerUp={() => setDrag(null)}
        onBlur={() => setDrag(null)}
      />
      <span className="slider-value">{label}</span>
      <button
        className="mini"
        title="Reset position"
        onClick={() => void run([resetJointPosition(joint.id)])}
      >
        0
      </button>
    </div>
  );
}

function AxisFields({
  joint,
  run,
}: {
  joint: JointInfo;
  run: RunCommands;
}) {
  const commit = (i: number, v: number) => {
    const axis = [...joint.axis] as [number, number, number];
    axis[i] = v;
    void run([setJointAxis(joint.id, axis)]);
  };
  return (
    <div className="triple-row">
      <span>Axis</span>
      {joint.axis.map((a, i) => (
        <MiniNum
          key={`${joint.id}:axis${i}:${a}`}
          value={a}
          onCommit={(v) => commit(i, v)}
        />
      ))}
    </div>
  );
}

function LimitFields({
  joint,
  run,
}: {
  joint: JointInfo;
  run: RunCommands;
}) {
  const angular = isAngular(joint.joint_type);
  const scale = angular ? RAD2DEG : 1;
  const unit = angular ? "°" : "m";
  const limits = joint.limits;

  const toggle = (enabled: boolean) => {
    if (enabled) {
      const def = angular
        ? { lower: -Math.PI, upper: Math.PI, effort: 100, velocity: 1 }
        : { lower: -1, upper: 1, effort: 100, velocity: 1 };
      void run([setJointLimits(joint.id, limits ?? def)]);
    } else {
      void run([setJointLimits(joint.id, null)]);
    }
  };

  const commit = (patch: { lower?: number; upper?: number }) => {
    if (!limits) return;
    void run([setJointLimits(joint.id, { ...limits, ...patch })]);
  };

  return (
    <div className="triple-row">
      <label className="check">
        <input
          type="checkbox"
          checked={limits !== null}
          onChange={(e) => toggle(e.currentTarget.checked)}
        />
        <span>Limits ({unit})</span>
      </label>
      {limits && (
        <>
          <MiniNum
            key={`${joint.id}:lo:${limits.lower}`}
            value={limits.lower * scale}
            onCommit={(v) => commit({ lower: v / scale })}
          />
          <MiniNum
            key={`${joint.id}:hi:${limits.upper}`}
            value={limits.upper * scale}
            onCommit={(v) => commit({ upper: v / scale })}
          />
        </>
      )}
    </div>
  );
}

function MiniNum({
  value,
  onCommit,
}: {
  value: number;
  onCommit: (v: number) => void;
}) {
  const rounded = Math.round(value * 1e4) / 1e4;
  return (
    <input
      className="mini-num"
      type="number"
      step="0.1"
      defaultValue={rounded}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
      }}
      onBlur={(e) => {
        const v = parseFloat(e.currentTarget.value);
        if (Number.isFinite(v) && Math.abs(v - rounded) > 1e-9) onCommit(v);
      }}
    />
  );
}
