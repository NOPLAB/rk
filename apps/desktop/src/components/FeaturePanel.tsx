import { useState } from "react";
import type { RunCommands, SceneSnapshot, SketchInfo, Vec3 } from "../engine/api";
import {
  addExtrude,
  addRevolve,
  deleteFeature,
  rollbackTo,
  setFeatureSuppressed,
  type BooleanOp,
  type ExtrudeDirection,
} from "../engine/commands";

const DIRECTIONS: ExtrudeDirection[] = ["Positive", "Negative", "Symmetric"];
const BOOLEAN_OPS: BooleanOp[] = ["New", "Join", "Cut", "Intersect"];
const AXES: { label: string; dir: Vec3 }[] = [
  { label: "X", dir: [1, 0, 0] },
  { label: "Y", dir: [0, 1, 0] },
  { label: "Z", dir: [0, 0, 1] },
];

const DEG2RAD = Math.PI / 180;

interface Props {
  snapshot: SceneSnapshot | null;
  /** Sketch the new feature is built from (the one being edited, if any) */
  sketch: SketchInfo | null;
  run: RunCommands;
}

export function FeaturePanel({ snapshot, sketch, run }: Props) {
  const [kind, setKind] = useState<"extrude" | "revolve">("extrude");
  const [distance, setDistance] = useState(10);
  const [direction, setDirection] = useState<ExtrudeDirection>("Positive");
  const [angle, setAngle] = useState(360);
  const [axis, setAxis] = useState("Z");
  const [op, setOp] = useState<BooleanOp>("New");
  const [target, setTarget] = useState("");

  if (!snapshot) return null;
  const { features, body_ids, rollback_position } = snapshot;

  const targetBody = op === "New" ? null : target || body_ids[0] || null;
  // Cut/Join need a body to act on, and every profile needs to be closed
  const blocked = !sketch
    ? "Select a sketch to build from"
    : sketch.profile_count === 0
      ? "The sketch has no closed profile"
      : op !== "New" && !targetBody
        ? "No body to combine with"
        : null;

  const onCreate = () => {
    if (!sketch || blocked) return;
    // Numbered so the history list can tell repeated features apart
    const name = `${kind === "extrude" ? "Extrude" : "Revolve"} ${features.length + 1}`;
    if (kind === "extrude") {
      void run([
        addExtrude(sketch.id, distance / 1000, direction, op, targetBody, name),
      ]);
    } else {
      const dir = AXES.find((a) => a.label === axis)?.dir ?? [0, 0, 1];
      void run([
        addRevolve(
          sketch.id,
          [0, 0, 0],
          dir,
          angle * DEG2RAD,
          op,
          targetBody,
          name,
        ),
      ]);
    }
  };

  return (
    <div className="features">
      <div className="tool-row">
        <button
          className={kind === "extrude" ? "active" : ""}
          onClick={() => setKind("extrude")}
        >
          Extrude
        </button>
        <button
          className={kind === "revolve" ? "active" : ""}
          onClick={() => setKind("revolve")}
        >
          Revolve
        </button>
      </div>

      {kind === "extrude" ? (
        <>
          <label className="field">
            <span>Dist</span>
            <input
              type="number"
              step="1"
              title="Extrusion distance (mm)"
              value={distance}
              onChange={(e) => setDistance(parseFloat(e.target.value) || 0)}
            />
          </label>
          <label className="field">
            <span>Dir</span>
            <select
              value={direction}
              onChange={(e) =>
                setDirection(e.target.value as ExtrudeDirection)
              }
            >
              {DIRECTIONS.map((d) => (
                <option key={d} value={d}>
                  {d}
                </option>
              ))}
            </select>
          </label>
        </>
      ) : (
        <>
          <label className="field">
            <span>Angle</span>
            <input
              type="number"
              step="15"
              title="Revolve angle (degrees)"
              value={angle}
              onChange={(e) => setAngle(parseFloat(e.target.value) || 0)}
            />
          </label>
          <label className="field">
            <span>Axis</span>
            <select value={axis} onChange={(e) => setAxis(e.target.value)}>
              {AXES.map((a) => (
                <option key={a.label} value={a.label}>
                  World {a.label}
                </option>
              ))}
            </select>
          </label>
        </>
      )}

      <label className="field">
        <span>Op</span>
        <select value={op} onChange={(e) => setOp(e.target.value as BooleanOp)}>
          {BOOLEAN_OPS.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      </label>

      {op !== "New" && (
        <label className="field">
          <span>Body</span>
          <select value={target} onChange={(e) => setTarget(e.target.value)}>
            <option value="">first</option>
            {body_ids.map((id, i) => (
              <option key={id} value={id}>
                body {i + 1}
              </option>
            ))}
          </select>
        </label>
      )}

      <div className="joint-actions">
        <button disabled={blocked !== null} onClick={onCreate}>
          Create {kind === "extrude" ? "Extrude" : "Revolve"}
        </button>
      </div>
      {blocked && <div className="small">{blocked}</div>}

      <div className="panel-title">History</div>
      {features.length === 0 && <div className="empty">No features</div>}
      {features.map((f, i) => {
        const rolledBack =
          rollback_position !== null && i >= rollback_position;
        return (
          <div
            key={f.id}
            className={`feature-row${rolledBack ? " inactive" : ""}`}
          >
            <span className="name" title={`${f.kind} · ${f.id}`}>
              {f.name}
            </span>
            <button
              className="mini"
              title={f.suppressed ? "Unsuppress" : "Suppress"}
              onClick={() =>
                void run([setFeatureSuppressed(f.id, !f.suppressed)])
              }
            >
              {f.suppressed ? "○" : "●"}
            </button>
            <button
              className="mini"
              title="Roll back to just after this feature"
              onClick={() => void run([rollbackTo(f.id)])}
            >
              ⤴
            </button>
            <button
              className="mini"
              title="Delete feature"
              onClick={() => void run([deleteFeature(f.id)])}
            >
              ✕
            </button>
          </div>
        );
      })}
      {rollback_position !== null && (
        <div className="joint-actions">
          <button onClick={() => void run([rollbackTo(null)])}>
            Roll to End
          </button>
        </div>
      )}
    </div>
  );
}
