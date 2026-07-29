// Value box for a dimension started from the ribbon.
//
// It opens on what the geometry measures right now, so pressing Enter
// without typing constrains the sketch without moving anything.

import { addSketchConstraint, solveSketch } from "../engine/commands";
import { fromDisplay } from "../engine/constraints";
import { newUuid } from "../engine/interaction";
import type { AppApi } from "../ui/appApi";
import { Icon } from "./icons";

export function DimensionEntry({ api }: { api: AppApi }) {
  const pending = api.pendingDimension;
  const sketch = api.activeSketch;
  const geometry = api.sketchGeometry;
  if (!pending || !sketch || !geometry) return null;

  const unit = pending.def.unit ?? "mm";

  const commit = () => {
    api.setPendingDimension(null);
    api.setSketchSelection([]);
    void api.run(
      [
        addSketchConstraint(
          sketch.id,
          pending.def.build(
            newUuid(),
            pending.ids,
            fromDisplay(pending.value, unit),
            geometry,
          ),
        ),
        solveSketch(sketch.id),
      ],
      true,
    );
  };

  return (
    <div className="dim-entry">
      <Icon name="cnDimension" size={16} />
      <span>{pending.def.label}</span>
      <input
        type="number"
        autoFocus
        step={unit === "deg" ? 5 : 1}
        value={Math.round(pending.value * 1e4) / 1e4}
        onChange={(e) => {
          const value = parseFloat(e.target.value);
          api.setPendingDimension({
            ...pending,
            value: Number.isFinite(value) ? value : 0,
          });
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") commit();
          if (e.key === "Escape") api.setPendingDimension(null);
        }}
      />
      <span className="small">{unit}</span>
      <button className="primary" onClick={commit}>
        Apply
      </button>
      <button onClick={() => api.setPendingDimension(null)}>Cancel</button>
    </div>
  );
}
