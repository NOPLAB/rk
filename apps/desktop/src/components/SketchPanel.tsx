import { useState } from "react";
import type { RunCommands, SceneSnapshot, SketchInfo } from "../engine/api";
import {
  createSketch,
  deleteSketch,
  solveSketch,
  standardPlane,
} from "../engine/commands";
import type { SketchTool } from "../scene/sketchTools";

const PLANES = ["XY", "XZ", "YZ"] as const;

const TOOLS: { tool: SketchTool; label: string; hint: string }[] = [
  { tool: "select", label: "Select", hint: "Pick entities (Esc to cancel a shape)" },
  { tool: "line", label: "Line", hint: "Click point to point; close on the first point" },
  { tool: "rect", label: "Rect", hint: "Click two opposite corners" },
  { tool: "circle", label: "Circle", hint: "Click the center, then the radius" },
];

interface Props {
  snapshot: SceneSnapshot | null;
  active: SketchInfo | null;
  tool: SketchTool;
  onTool: (tool: SketchTool) => void;
  onActivate: (sketchId: string | null) => void;
  onAlign: () => void;
  run: RunCommands;
}

export function SketchPanel({
  snapshot,
  active,
  tool,
  onTool,
  onActivate,
  onAlign,
  run,
}: Props) {
  const [plane, setPlane] = useState<(typeof PLANES)[number]>("XY");
  const [offset, setOffset] = useState(0);

  if (!snapshot) return null;
  const sketches = snapshot.sketches;

  const onNew = async () => {
    // Every sketch would otherwise be called "Sketch"; the list needs to
    // tell them apart. The engine mints the ID and reports it in `sketch_added`
    const events = await run([
      createSketch(
        standardPlane(plane, offset / 1000),
        `Sketch ${sketches.length + 1} (${plane})`,
      ),
    ]);
    const added = events.find((e) => e.type === "sketch_added");
    if (added) onActivate(added.sketch_id as string);
  };

  return (
    <div className="sketches">
      <div className="connect-row">
        <select
          value={plane}
          onChange={(e) => setPlane(e.target.value as (typeof PLANES)[number])}
        >
          {PLANES.map((p) => (
            <option key={p} value={p}>
              {p} plane
            </option>
          ))}
        </select>
        <input
          className="mini-num"
          type="number"
          step="10"
          title="Offset along the plane normal (mm)"
          value={offset}
          onChange={(e) => setOffset(parseFloat(e.target.value) || 0)}
        />
        <button onClick={() => void onNew()}>New</button>
      </div>

      {sketches.length === 0 && <div className="empty">No sketches</div>}
      {sketches.map((s) => (
        <div
          key={s.id}
          className={`sketch-row${s.id === active?.id ? " selected" : ""}`}
          onClick={() => onActivate(s.id === active?.id ? null : s.id)}
        >
          <span className="name" title={s.name}>
            {s.name}
          </span>
          <span className="small counts">
            {s.entity_count}e · {s.profile_count}p
          </span>
          <button
            className="mini"
            title="Delete sketch"
            onClick={(e) => {
              e.stopPropagation();
              if (s.id === active?.id) onActivate(null);
              void run([deleteSketch(s.id)]);
            }}
          >
            ✕
          </button>
        </div>
      ))}

      {active && (
        <div className="sketch-edit">
          <div className="tool-row">
            {TOOLS.map(({ tool: t, label, hint }) => (
              <button
                key={t}
                title={hint}
                className={t === tool ? "active" : ""}
                onClick={() => onTool(t)}
              >
                {label}
              </button>
            ))}
          </div>
          <div className="small">
            {TOOLS.find((t) => t.tool === tool)?.hint}
            {active.dof > 0 && ` · ${active.dof} DOF`}
          </div>
          <div className="joint-actions">
            <button onClick={onAlign} title="Look down the sketch plane">
              Align View
            </button>
            <button onClick={() => void run([solveSketch(active.id)])}>
              Solve
            </button>
            <button onClick={() => onActivate(null)}>Finish</button>
          </div>
        </div>
      )}
    </div>
  );
}
