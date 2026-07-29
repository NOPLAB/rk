// The contents of each ribbon tab.
//
// Commands live here rather than in App so the tabs stay the single place
// that says what RK can do from the ribbon; they reach the engine through
// `api.run`, exactly like the panels do.

import { useMemo, useState } from "react";
import {
  addCollision,
  addSketchConstraint,
  connectParts,
  createBox,
  createCylinder,
  createSketch,
  createSphere,
  deletePart,
  disconnectPart,
  resetAllJointPositions,
  rollbackTo,
  solveSketch,
  standardPlane,
} from "../engine/commands";
import {
  DIMENSIONAL,
  GEOMETRIC,
  classify,
  matchSlots,
  toDisplay,
  type ConstraintDef,
} from "../engine/constraints";
import { newUuid } from "../engine/interaction";
import type { AppApi } from "../ui/appApi";
import { SHAPES, defaultGeometry } from "./CollisionPanel";
import type { IconName } from "./icons";
import { RibBig, RibCol, RibGroup, RibHint, RibSmall, chunk } from "./ribbonParts";

const PLANES = ["XY", "XZ", "YZ"] as const;

const CONSTRAINT_ICONS: Record<string, IconName> = {
  Coincident: "cnCoincident",
  Horizontal: "cnHorizontal",
  Vertical: "cnVertical",
  Parallel: "cnParallel",
  Perpendicular: "cnPerpendicular",
  EqualLength: "cnEqual",
  EqualRadius: "cnEqualRadius",
  Tangent: "cnTangent",
  PointOnCurve: "cnOnCurve",
  Midpoint: "cnMidpoint",
  Fixed: "cnFix",
  Length: "cnDimension",
  Distance: "cnDimension",
  HorizontalDistance: "cnDimension",
  VerticalDistance: "cnDimVertical",
  Radius: "cnRadius",
  Diameter: "cnDiameter",
  Angle: "cnAngle",
};

// ---- 3D Model -----------------------------------------------------------

export function ModelTab({ api }: { api: AppApi }) {
  const [plane, setPlane] = useState<(typeof PLANES)[number]>("XY");
  const [offset, setOffset] = useState(0);
  const snapshot = api.snapshot;
  const sketchCount = snapshot?.sketches.length ?? 0;

  const startSketch = async () => {
    // Every sketch would otherwise be called "Sketch"; the browser needs to
    // tell them apart. The engine mints the ID and reports it back.
    const events = await api.run([
      createSketch(
        standardPlane(plane, offset / 1000),
        `Sketch ${sketchCount + 1} (${plane})`,
      ),
    ]);
    const added = events.find((e) => e.type === "sketch_added");
    if (added) api.activateSketch(added.sketch_id as string);
  };

  return (
    <>
      <RibGroup name="Sketch">
        <RibBig
          icon="sketch"
          label="Start 2D Sketch"
          hint="Create a sketch on the chosen plane and edit it"
          onClick={() => void startSketch()}
        />
        <RibCol>
          <div className="rb-field">
            <select
              value={plane}
              title="Sketch plane"
              onChange={(e) =>
                setPlane(e.target.value as (typeof PLANES)[number])
              }
            >
              {PLANES.map((p) => (
                <option key={p} value={p}>
                  {p} plane
                </option>
              ))}
            </select>
          </div>
          <div className="rb-field">
            <input
              type="number"
              step="10"
              style={{ width: 62 }}
              title="Offset along the plane normal (mm)"
              value={offset}
              onChange={(e) => setOffset(parseFloat(e.target.value) || 0)}
            />
            <span>mm</span>
          </div>
        </RibCol>
      </RibGroup>

      <RibGroup name="Create">
        <RibBig
          icon="extrude"
          label="Extrude"
          hint="Extrude a sketch profile into a solid"
          disabled={sketchCount === 0}
          active={api.dialog === "extrude"}
          onClick={() => api.setDialog("extrude")}
        />
        <RibBig
          icon="revolve"
          label="Revolve"
          hint="Revolve a sketch profile around an axis"
          disabled={sketchCount === 0}
          active={api.dialog === "revolve"}
          onClick={() => api.setDialog("revolve")}
        />
        {sketchCount === 0 && <RibHint>Draw a sketch first</RibHint>}
      </RibGroup>

      <RibGroup name="Primitives">
        <RibCol>
          <RibSmall
            icon="box"
            label="Box"
            hint="100 mm cube"
            onClick={() => void api.run([createBox([0.1, 0.1, 0.1])])}
          />
          <RibSmall
            icon="cylinder"
            label="Cylinder"
            hint="Ø60 × 100 mm"
            onClick={() => void api.run([createCylinder(0.03, 0.1)])}
          />
          <RibSmall
            icon="sphere"
            label="Sphere"
            hint="Ø100 mm"
            onClick={() => void api.run([createSphere(0.05)])}
          />
        </RibCol>
      </RibGroup>

      <RibGroup name="Modify">
        <RibCol>
          <RibSmall
            icon="trash"
            label="Delete"
            hint="Delete the selected part"
            disabled={!api.selected}
            onClick={() => {
              const id = api.selected;
              if (!id) return;
              api.select(null);
              void api.run([deletePart(id)]);
            }}
          />
          <RibSmall
            icon="rollback"
            label="Roll to End"
            hint="Rebuild every feature in the history"
            disabled={snapshot?.rollback_position == null}
            onClick={() => void api.run([rollbackTo(null)])}
          />
        </RibCol>
      </RibGroup>

      <ManipulateGroup api={api} />
    </>
  );
}

// ---- Sketch -------------------------------------------------------------

export function SketchTab({ api }: { api: AppApi }) {
  const sketch = api.activeSketch;
  const geometry = api.sketchGeometry;

  const selected = useMemo(
    () => (geometry ? classify(geometry, api.sketchSelection) : []),
    [geometry, api.sketchSelection],
  );
  // Which constraint buttons the current selection enables
  const matches = useMemo(() => {
    const out = new Map<string, string[]>();
    for (const def of [...GEOMETRIC, ...DIMENSIONAL]) {
      const ids = matchSlots(def, selected);
      if (ids) out.set(def.kind, ids);
    }
    return out;
  }, [selected]);

  if (!sketch) {
    return (
      <RibGroup name="Sketch">
        <RibHint>
          No sketch is being edited. Start one from the 3D Model tab or
          double-click a sketch in the browser.
        </RibHint>
      </RibGroup>
    );
  }

  const start = (def: ConstraintDef) => {
    const ids = matches.get(def.kind);
    if (!ids || !geometry) return;
    if (!def.unit) {
      api.setSketchSelection([]);
      // Adding the constraint and re-solving is one action → one undo step
      void api.run(
        [
          addSketchConstraint(sketch.id, def.build(newUuid(), ids, 0, geometry)),
          solveSketch(sketch.id),
        ],
        true,
      );
      return;
    }
    // Open on what the geometry measures now, so accepting it moves nothing
    const measured = def.measure?.(ids, geometry) ?? 0;
    api.setPendingDimension({ def, ids, value: toDisplay(measured, def.unit) });
  };

  const constraintButton = (def: ConstraintDef) => (
    <RibSmall
      key={def.kind}
      icon={CONSTRAINT_ICONS[def.kind] ?? "cnDimension"}
      label={def.label}
      hint={def.hint}
      disabled={!matches.has(def.kind)}
      active={api.pendingDimension?.def.kind === def.kind}
      onClick={() => start(def)}
    />
  );

  return (
    <>
      <RibGroup name="Draw">
        <RibBig
          icon="line"
          label="Line"
          hint="Click point to point; close on the first point"
          active={api.sketchTool === "line"}
          onClick={() => api.setSketchTool("line")}
        />
        <RibBig
          icon="rect"
          label="Rectangle"
          hint="Click two opposite corners"
          active={api.sketchTool === "rect"}
          onClick={() => api.setSketchTool("rect")}
        />
        <RibBig
          icon="circle"
          label="Circle"
          hint="Click the centre, then the radius"
          active={api.sketchTool === "circle"}
          onClick={() => api.setSketchTool("circle")}
        />
        <RibCol>
          <RibSmall
            icon="select"
            label="Select"
            hint="Pick entities — Shift adds to the selection"
            active={api.sketchTool === "select"}
            onClick={() => api.setSketchTool("select")}
          />
        </RibCol>
      </RibGroup>

      <RibGroup name="Constrain">
        {chunk(GEOMETRIC, 3).map((column, i) => (
          <RibCol key={i}>{column.map(constraintButton)}</RibCol>
        ))}
      </RibGroup>

      <RibGroup name="Dimension">
        {chunk(DIMENSIONAL, 3).map((column, i) => (
          <RibCol key={i}>{column.map(constraintButton)}</RibCol>
        ))}
      </RibGroup>

      <RibGroup name="Exit">
        <RibCol>
          <RibSmall
            icon="solve"
            label="Solve"
            hint="Re-solve the sketch constraints"
            onClick={() => void api.run([solveSketch(sketch.id)])}
          />
          <RibSmall
            icon="align"
            label="Align View"
            hint="Look straight down the sketch plane"
            onClick={() => api.viewport()?.alignToSketch()}
          />
        </RibCol>
        <RibBig
          icon="finish"
          label="Finish Sketch"
          hint="Leave sketch mode"
          onClick={() => api.activateSketch(null)}
        />
      </RibGroup>
    </>
  );
}

// ---- Assembly -----------------------------------------------------------

export function AssemblyTab({ api }: { api: AppApi }) {
  const [parent, setParent] = useState("");
  const snapshot = api.snapshot;
  const parts = snapshot?.parts ?? [];
  const child = api.selected;
  const canConnect = !!child && parent !== "" && parent !== child;
  const link = child
    ? (snapshot?.links.find((l) => l.part_id === child) ?? null)
    : null;
  const jointOfChild = child
    ? (snapshot?.joints.find((j) => j.child_part === child) ?? null)
    : null;

  return (
    <>
      <RibGroup name="Joint">
        <RibBig
          icon="connect"
          label="Connect"
          hint="Join the selected part to a parent part"
          disabled={!canConnect}
          onClick={() => {
            if (!canConnect || !child) return;
            void api.run([connectParts(parent, child)]);
            setParent("");
          }}
        />
        <RibCol>
          <div className="rb-field">
            <span>to</span>
            <select
              value={parent}
              title="Parent part"
              style={{ width: 108 }}
              onChange={(e) => setParent(e.target.value)}
            >
              <option value="">parent…</option>
              {parts
                .filter((p) => p.id !== child)
                .map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
            </select>
          </div>
          <RibSmall
            icon="disconnect"
            label="Disconnect"
            hint="Detach the selected part from its parent"
            disabled={!jointOfChild}
            onClick={() => child && void api.run([disconnectPart(child)])}
          />
          <RibSmall
            icon="reset"
            label="Reset Poses"
            hint="Send every joint back to its zero position"
            disabled={(snapshot?.joints.length ?? 0) === 0}
            onClick={() => void api.run([resetAllJointPositions()])}
          />
        </RibCol>
        {!child && <RibHint>Select the part to connect</RibHint>}
      </RibGroup>

      <RibGroup name="Collision">
        {chunk(SHAPES, 2).map((column, i) => (
          <RibCol key={i}>
            {column.map((shape) => (
              <RibSmall
                key={shape}
                icon={
                  shape === "Box"
                    ? "box"
                    : shape === "Sphere"
                      ? "sphere"
                      : "cylinder"
                }
                label={shape}
                hint={`Add a ${shape.toLowerCase()} collision shape to this link`}
                disabled={!link}
                onClick={() =>
                  link &&
                  void api.run([addCollision(link.id, defaultGeometry(shape))])
                }
              />
            ))}
          </RibCol>
        ))}
        <RibCol>
          <RibSmall
            icon="eye"
            label="Show"
            hint="Draw collision shapes as wireframes"
            active={api.showCollisions}
            onClick={() => api.setShowCollisions(!api.showCollisions)}
          />
        </RibCol>
        {/* Collisions belong to links, which only exist once a part is in
            the assembly */}
        {!link && child && (
          <RibHint>Connect this part before adding collisions</RibHint>
        )}
      </RibGroup>
    </>
  );
}

// ---- View ---------------------------------------------------------------

export function ViewTab({ api }: { api: AppApi }) {
  const vp = api.viewport();
  return (
    <>
      <RibGroup name="Navigate">
        <RibBig
          icon="home"
          label="Home View"
          hint="Isometric view, framing the model"
          onClick={() => api.viewport()?.homeView()}
        />
        <RibCol>
          <RibSmall
            icon="fit"
            label="Zoom Fit"
            onClick={() => api.viewport()?.fitCamera()}
          />
          <RibSmall
            icon="viewIso"
            label="Isometric"
            onClick={() => api.viewport()?.setStandardView("iso")}
          />
        </RibCol>
        <RibCol>
          <RibSmall
            icon="viewFront"
            label="Front"
            onClick={() => api.viewport()?.setStandardView("front")}
          />
          <RibSmall
            icon="viewTop"
            label="Top"
            onClick={() => api.viewport()?.setStandardView("top")}
          />
          <RibSmall
            icon="viewRight"
            label="Right"
            onClick={() => api.viewport()?.setStandardView("right")}
          />
        </RibCol>
      </RibGroup>

      <RibGroup name="Appearance">
        <RibCol>
          <RibSmall
            icon="grid"
            label="Grid"
            hint="Show the ground grid"
            active={api.showGrid}
            onClick={() => api.setShowGrid(!api.showGrid)}
          />
          <RibSmall
            icon="collision"
            label="Collisions"
            hint="Draw collision shapes as wireframes"
            active={api.showCollisions}
            onClick={() => api.setShowCollisions(!api.showCollisions)}
          />
        </RibCol>
      </RibGroup>

      <ManipulateGroup api={api} />

      <RibGroup name="Windows">
        <RibCol>
          <RibSmall
            icon="browserPanel"
            label="Browser"
            active={api.showBrowser}
            onClick={() => api.setShowBrowser(!api.showBrowser)}
          />
          <RibSmall
            icon="inspectorPanel"
            label="Inspector"
            active={api.showInspector}
            onClick={() => api.setShowInspector(!api.showInspector)}
          />
        </RibCol>
      </RibGroup>
      {!vp && <RibHint>Viewport is still starting…</RibHint>}
    </>
  );
}

// ---- shared -------------------------------------------------------------

/** Gizmo modes; the same group appears on the 3D Model and View tabs */
function ManipulateGroup({ api }: { api: AppApi }) {
  const inSketch = api.activeSketch !== null;
  return (
    <RibGroup name="Manipulate">
      <RibCol>
        <RibSmall
          icon="select"
          label="Select"
          hint="Select only (Q)"
          active={api.gizmoMode === "none"}
          disabled={inSketch}
          onClick={() => api.setGizmoMode("none")}
        />
        <RibSmall
          icon="move"
          label="Move"
          hint="Move gizmo (W)"
          active={api.gizmoMode === "translate"}
          disabled={inSketch}
          onClick={() => api.setGizmoMode("translate")}
        />
        <RibSmall
          icon="rotate"
          label="Rotate"
          hint="Rotate gizmo (E)"
          active={api.gizmoMode === "rotate"}
          disabled={inSketch}
          onClick={() => api.setGizmoMode("rotate")}
        />
      </RibCol>
    </RibGroup>
  );
}
