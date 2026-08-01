// Modeless dialog for the extrude and revolve commands, floating over the
// viewport the way Fusion's feature dialogs do.
//
// It builds whatever regions the user clicked in the 3D view. With nothing
// clicked it falls back to the sketch being edited and takes every region in
// it, so the quick path — draw, finish, extrude — still needs no picking.

import { useState } from "react";
import type { Vec3 } from "../engine/api";
import {
  addExtrude,
  addRevolve,
  type BooleanOp,
  type ExtrudeDirection,
} from "../engine/commands";
import type { AppApi, DialogKind } from "../ui/appApi";
import { Icon } from "./icons";

const DIRECTIONS: ExtrudeDirection[] = ["Positive", "Negative", "Symmetric"];
const BOOLEAN_OPS: BooleanOp[] = ["New", "Join", "Cut", "Intersect"];
const AXES: { label: string; dir: Vec3 }[] = [
  { label: "X", dir: [1, 0, 0] },
  { label: "Y", dir: [0, 1, 0] },
  { label: "Z", dir: [0, 0, 1] },
];

const DEG2RAD = Math.PI / 180;

export function FeatureDialog({
  api,
  kind,
}: {
  api: AppApi;
  kind: DialogKind;
}) {
  const [picked, setPicked] = useState<string | null>(null);
  const [distance, setDistance] = useState(10);
  const [direction, setDirection] = useState<ExtrudeDirection>("Positive");
  const [angle, setAngle] = useState(360);
  const [axis, setAxis] = useState("Z");
  const [op, setOp] = useState<BooleanOp>("New");
  const [target, setTarget] = useState("");

  const snapshot = api.snapshot;
  if (!snapshot) return null;
  const { sketches, body_ids, features } = snapshot;

  // Clicking a region in the 3D view is the primary way in; the sketch being
  // edited is the fallback, and the dropdown overrides both
  const clicked = api.regionSelection;
  const sketchId =
    picked ??
    clicked[0]?.sketchId ??
    api.activeSketch?.id ??
    sketches[sketches.length - 1]?.id ??
    "";
  const sketch = sketches.find((s) => s.id === sketchId) ?? null;
  // Only regions of the sketch actually being built count
  const regions = clicked
    .filter((r) => r.sketchId === sketchId)
    .map((r) => r.regionId);

  const targetBody = op === "New" ? null : target || body_ids[0] || null;
  // Cut/Join need a body to act on, and there has to be something enclosed
  const blocked = !sketch
    ? "Pick a sketch to build from"
    : sketch.profile_count === 0
      ? "That sketch encloses nothing to build from"
      : op !== "New" && !targetBody
        ? "No body to combine with"
        : null;

  const title = kind === "extrude" ? "Extrude" : "Revolve";

  const onCreate = () => {
    if (!sketch || blocked) return;
    // Numbered so the browser can tell repeated features apart
    const name = `${title} ${features.length + 1}`;
    if (kind === "extrude") {
      void api.run([
        addExtrude(
          sketch.id,
          distance / 1000,
          direction,
          op,
          targetBody,
          name,
          regions,
        ),
      ]);
    } else {
      const dir = AXES.find((a) => a.label === axis)?.dir ?? [0, 0, 1];
      void api.run([
        addRevolve(
          sketch.id,
          [0, 0, 0],
          dir,
          angle * DEG2RAD,
          op,
          targetBody,
          name,
          regions,
        ),
      ]);
    }
    api.setRegionSelection([]);
    api.setDialog(null);
  };

  return (
    <div className="dialog left">
      <div className="dialog-head">
        <Icon name={kind === "extrude" ? "extrude" : "revolve"} size={16} />
        <span>{title}</span>
        <span className="spacer" />
        <button
          className="qa-btn"
          title="Close"
          onClick={() => api.setDialog(null)}
        >
          <Icon name="close" size={13} />
        </button>
      </div>

      <div className="dialog-body">
        <label className="field">
          <span>Sketch</span>
          <select
            value={sketchId}
            onChange={(e) => setPicked(e.target.value)}
          >
            {sketches.length === 0 && <option value="">no sketches</option>}
            {sketches.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name} ({s.profile_count}p)
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span>Regions</span>
          <span className="small">
            {regions.length > 0
              ? `${regions.length} selected`
              : `all ${sketch?.profile_count ?? 0}`}
          </span>
        </label>

        {kind === "extrude" ? (
          <>
            <label className="field">
              <span>Distance</span>
              <input
                type="number"
                step="1"
                title="Extrusion distance (mm)"
                value={distance}
                onChange={(e) => setDistance(parseFloat(e.target.value) || 0)}
              />
            </label>
            <label className="field">
              <span>Direction</span>
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
          <span>Operation</span>
          <select
            value={op}
            onChange={(e) => setOp(e.target.value as BooleanOp)}
          >
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

        {blocked && <div className="small">{blocked}</div>}
      </div>

      <div className="dialog-foot">
        <button
          className="primary"
          disabled={blocked !== null}
          onClick={onCreate}
        >
          OK
        </button>
        <button onClick={() => api.setDialog(null)}>Cancel</button>
      </div>
    </div>
  );
}
