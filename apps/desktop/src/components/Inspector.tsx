// Right-hand dock. What it shows follows the modelling context: the sketch's
// constraints while sketching, otherwise the selected part's properties and
// the assembly panels.

import type { AppApi } from "../ui/appApi";
import { CollisionPanel } from "./CollisionPanel";
import { ConstraintList } from "./ConstraintList";
import { Icon } from "./icons";
import { JointPanel } from "./JointPanel";
import { PropertiesPanel } from "./PropertiesPanel";

export function Inspector({ api }: { api: AppApi }) {
  const sketch = api.activeSketch;

  // The dock's tab strip names and closes the panel now
  return (
    <aside className="inspector">
      {sketch ? (
        <>
          <div className="panel-title">
            <Icon name="sketch" size={14} />
            {sketch.name}
          </div>
          <ConstraintList api={api} sketch={sketch} />
        </>
      ) : (
        <>
          <div className="panel-title">
            <Icon name="part" size={14} />
            {api.selectedPart?.name ?? "Part"}
          </div>
          <PropertiesPanel part={api.selectedPart} run={api.run} />

          <div className="panel-title">
            <Icon name="joint" size={14} />
            Joints
          </div>
          <JointPanel snapshot={api.snapshot} run={api.run} />

          <div className="panel-title">
            <Icon name="collision" size={14} />
            Collisions
          </div>
          <CollisionPanel
            snapshot={api.snapshot}
            part={api.selectedPart}
            visible={api.showCollisions}
            onVisible={api.setShowCollisions}
            run={api.run}
          />
        </>
      )}
    </aside>
  );
}
