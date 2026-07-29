import { useMemo, useState } from "react";
import type {
  RunCommands,
  SketchConstraint,
  SketchGeometry,
  SketchInfo,
} from "../engine/api";
import {
  addSketchConstraint,
  deleteSketchConstraint,
  solveSketch,
} from "../engine/commands";
import {
  DIMENSIONAL,
  GEOMETRIC,
  classify,
  fromDisplay,
  kindOf,
  matchSlots,
  toDisplay,
  unitOf,
  withValue,
  type ConstraintDef,
} from "../engine/constraints";
import { newUuid } from "../engine/interaction";
import { MiniNum } from "./MiniNum";

/** A dimension waiting for the user to accept or change its value */
interface Pending {
  def: ConstraintDef;
  /** Entity IDs in slot order */
  ids: string[];
  /** In the definition's display unit */
  value: number;
}

interface Props {
  sketch: SketchInfo;
  geometry: SketchGeometry | null;
  selection: string[];
  onSelection: (entityIds: string[]) => void;
  /** Show a constraint's entities in the viewport while the pointer is on it */
  onHover: (entityIds: string[]) => void;
  run: RunCommands;
}

export function ConstraintPanel({
  sketch,
  geometry,
  selection,
  onSelection,
  onHover,
  run,
}: Props) {
  const [pending, setPending] = useState<Pending | null>(null);

  const selected = useMemo(
    () => (geometry ? classify(geometry, selection) : []),
    [geometry, selection],
  );
  // Which buttons the current selection enables
  const matches = useMemo(() => {
    const out = new Map<string, string[]>();
    for (const def of [...GEOMETRIC, ...DIMENSIONAL]) {
      const ids = matchSlots(def, selected);
      if (ids) out.set(def.kind, ids);
    }
    return out;
  }, [selected]);

  if (!geometry) return null;
  const geom = geometry;

  /** Adding a constraint and re-solving is one action, so it is one undo step */
  const apply = (constraint: SketchConstraint, clearSelection: boolean) => {
    setPending(null);
    if (clearSelection) onSelection([]);
    void run(
      [addSketchConstraint(sketch.id, constraint), solveSketch(sketch.id)],
      true,
    );
  };

  const start = (def: ConstraintDef) => {
    const ids = matches.get(def.kind);
    if (!ids) return;
    if (!def.unit) {
      apply(def.build(newUuid(), ids, 0, geom), true);
      return;
    }
    // Open on what the geometry measures now, so accepting it moves nothing
    const measured = def.measure?.(ids, geom) ?? 0;
    setPending({ def, ids, value: toDisplay(measured, def.unit) });
  };

  const commitPending = () => {
    if (!pending) return;
    const { def, ids, value } = pending;
    const engineValue = fromDisplay(value, def.unit ?? "mm");
    apply(def.build(newUuid(), ids, engineValue, geom), true);
  };

  const button = (def: ConstraintDef) => (
    <button
      key={def.kind}
      title={def.hint}
      disabled={!matches.has(def.kind)}
      className={pending?.def.kind === def.kind ? "active" : ""}
      onClick={() => start(def)}
    >
      {def.label}
    </button>
  );

  return (
    <div className="constraints">
      <div className="small">
        {selected.length > 0
          ? `${selected.length} selected`
          : "Select tool: click entities, Shift to add"}
        {sketch.dof > 0 && ` · ${sketch.dof} DOF`}
        {!sketch.is_solved && " · needs solve"}
      </div>

      <div className="cn-grid">{GEOMETRIC.map(button)}</div>
      <div className="cn-grid">{DIMENSIONAL.map(button)}</div>

      {pending && (
        <div className="tool-row">
          <span className="small">{pending.def.label}</span>
          <input
            className="mini-num"
            type="number"
            autoFocus
            step={pending.def.unit === "deg" ? 5 : 1}
            title={pending.def.unit}
            value={Math.round(pending.value * 1e4) / 1e4}
            onChange={(e) => {
              const value = parseFloat(e.target.value);
              setPending({ ...pending, value: Number.isFinite(value) ? value : 0 });
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitPending();
              if (e.key === "Escape") setPending(null);
            }}
          />
          <span className="small">{pending.def.unit}</span>
          <button onClick={commitPending}>Apply</button>
          <button onClick={() => setPending(null)}>✕</button>
        </div>
      )}

      {geom.constraints.length === 0 && <div className="empty">No constraints</div>}
      {geom.constraints.map((c) => {
        const unit = unitOf(kindOf(c.constraint));
        return (
          <div
            key={c.id}
            className="constraint-row"
            onMouseEnter={() => onHover(c.entities)}
            onMouseLeave={() => onHover([])}
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
                  apply(withValue(c.constraint, fromDisplay(v, unit)), false)
                }
              />
            )}
            <button
              className="mini"
              title="Delete constraint"
              onClick={() => {
                onHover([]);
                void run(
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
