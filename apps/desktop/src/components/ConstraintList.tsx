// The constraints on the sketch being edited.
//
// Creating constraints lives on the ribbon's Sketch tab; this list is what
// you do afterwards — read them, retype a driven value, delete one. Editing
// works by re-sending the constraint under its own ID, which replaces it.

import type { SketchInfo } from "../engine/api";
import {
  addSketchConstraint,
  deleteSketchConstraint,
  solveSketch,
} from "../engine/commands";
import { fromDisplay, kindOf, toDisplay, unitOf, withValue } from "../engine/constraints";
import type { AppApi } from "../ui/appApi";
import { MiniNum } from "./MiniNum";

export function ConstraintList({
  api,
  sketch,
}: {
  api: AppApi;
  sketch: SketchInfo;
}) {
  const geometry = api.sketchGeometry;
  if (!geometry) return <div className="empty">Loading sketch…</div>;

  const selectedCount = api.sketchSelection.length;

  return (
    <div className="constraints">
      <div className="small">
        {selectedCount > 0
          ? `${selectedCount} selected — pick a constraint on the ribbon`
          : "Select entities with the Select tool (Shift adds)"}
      </div>
      <div className="small">
        {sketch.dof > 0 ? `${sketch.dof} DOF` : "Fully constrained"}
        {!sketch.is_solved && " · needs solve"}
      </div>

      {geometry.constraints.length === 0 && (
        <div className="empty">No constraints</div>
      )}
      {geometry.constraints.map((c) => {
        const unit = unitOf(kindOf(c.constraint));
        return (
          <div
            key={c.id}
            className="constraint-row"
            onMouseEnter={() => api.hoverSketch(c.entities)}
            onMouseLeave={() => api.hoverSketch([])}
          >
            <span className="name" title={c.label}>
              {c.label}
            </span>
            {c.value !== null && unit && (
              <MiniNum
                // Reset the input when the engine reports a new value
                key={`${c.id}:${c.value}`}
                value={toDisplay(c.value, unit)}
                step={unit === "deg" ? 5 : 1}
                title={unit}
                onCommit={(v) =>
                  // Adding and re-solving is one action → one undo step
                  void api.run(
                    [
                      addSketchConstraint(
                        sketch.id,
                        withValue(c.constraint, fromDisplay(v, unit)),
                      ),
                      solveSketch(sketch.id),
                    ],
                    true,
                  )
                }
              />
            )}
            <button
              className="mini"
              title="Delete constraint"
              onClick={() => {
                api.hoverSketch([]);
                void api.run(
                  [
                    deleteSketchConstraint(sketch.id, c.id),
                    solveSketch(sketch.id),
                  ],
                  true,
                );
              }}
            >
              ✕
            </button>
          </div>
        );
      })}
    </div>
  );
}
