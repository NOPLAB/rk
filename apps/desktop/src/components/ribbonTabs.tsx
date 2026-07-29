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
  createSphere,
  deletePart,
  disconnectPart,
  resetAllJointPositions,
  rollbackTo,
  setSketchConstruction,
  solveSketch,
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
import { TOOLS } from "../scene/sketchToolInfo";
import type { SketchTool } from "../scene/sketchTools";
import type { AppApi } from "../ui/appApi";
import { SHAPES, defaultGeometry } from "./CollisionPanel";
import type { IconName } from "./icons";
import { RibBig, RibCol, RibGroup, RibHint, RibSmall, chunk } from "./ribbonParts";

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
  const snapshot = api.snapshot;
  const sketchCount = snapshot?.sketches.length ?? 0;
  const picked = api.regionSelection.length;

  return (
    <>
      <RibGroup name="Sketch">
        <RibBig
          icon="sketch"
          label="Create Sketch"
          hint="Then click a plane or a flat face in the 3D view"
          active={api.pickingPlane}
          onClick={() =>
            api.pickingPlane ? api.cancelPlanePick() : api.beginPlanePick()
          }
        />
        {api.pickingPlane && <RibHint>Pick a plane or a face — Esc cancels</RibHint>}
      </RibGroup>

      <RibGroup name="Create">
        <RibBig
          icon="extrude"
          label="Extrude"
          hint="Turn the selected sketch region into a solid"
          disabled={sketchCount === 0}
          active={api.dialog === "extrude"}
          onClick={() => api.setDialog("extrude")}
        />
        <RibBig
          icon="revolve"
          label="Revolve"
          hint="Sweep the selected sketch region around an axis"
          disabled={sketchCount === 0}
          active={api.dialog === "revolve"}
          onClick={() => api.setDialog("revolve")}
        />
        {sketchCount === 0 ? (
          <RibHint>Draw a sketch first</RibHint>
        ) : (
          <RibHint>
            {picked > 0
              ? `${picked} region${picked > 1 ? "s" : ""} selected`
              : "Click a shaded region to choose what to build"}
          </RibHint>
        )}
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

  const tool = (name: SketchTool) => {
    const spec = TOOLS[name];
    return (
      <RibSmall
        key={name}
        icon={spec.icon}
        label={spec.label}
        hint={spec.hint}
        active={api.sketchTool === name}
        onClick={() => api.setSketchTool(name)}
      />
    );
  };
  const bigTool = (name: SketchTool) => {
    const spec = TOOLS[name];
    return (
      <RibBig
        icon={spec.icon}
        label={spec.label}
        hint={spec.hint}
        active={api.sketchTool === name}
        onClick={() => api.setSketchTool(name)}
      />
    );
  };
  const options = api.toolOptions;
  const numberField = (
    label: string,
    value: number,
    step: number,
    onChange: (value: number) => void,
    unit?: string,
  ) => (
    <div className="rb-field" key={label}>
      <span>{label}</span>
      <input
        type="number"
        step={step}
        style={{ width: 54 }}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
      />
      {unit && <span>{unit}</span>}
    </div>
  );

  return (
    <>
      <RibGroup name="Draw">
        {bigTool("line")}
        {bigTool("rect")}
        {bigTool("circle")}
        <RibCol>
          {tool("select")}
          {tool("rectCenter")}
          {tool("rect3")}
        </RibCol>
        <RibCol>
          {tool("circle2")}
          {tool("circle3")}
          {tool("point")}
        </RibCol>
        <RibCol>
          {tool("arc3")}
          {tool("arcCenter")}
          {tool("spline")}
        </RibCol>
        <RibCol>
          {tool("ellipse")}
          {tool("slot")}
          {tool("slotOverall")}
        </RibCol>
        <RibCol>
          {tool("polygon")}
          {tool("polygonCirc")}
          {tool("polygonEdge")}
        </RibCol>
        <RibCol>
          {numberField("Sides", options.sides, 1, (v) =>
            api.setToolOptions({ sides: Math.max(3, Math.round(v)) }),
          )}
          <RibSmall
            icon="construction"
            label="Construction"
            hint="Draw guides that never enclose a region"
            active={options.construction}
            onClick={() =>
              api.setToolOptions({ construction: !options.construction })
            }
          />
          <RibSmall
            icon="construction"
            label="Toggle Selected"
            hint="Switch the selected curves between normal and construction"
            disabled={api.sketchSelection.length === 0}
            onClick={() => {
              const ids = api.sketchSelection;
              const anyNormal = ids.some(
                (id) =>
                  !geometry ||
                  [
                    ...geometry.lines,
                    ...geometry.circles,
                    ...geometry.arcs,
                    ...geometry.ellipses,
                    ...geometry.splines,
                  ].some((e) => e.id === id && !e.construction),
              );
              void api.run([setSketchConstruction(sketch.id, ids, anyNormal)]);
            }}
          />
        </RibCol>
      </RibGroup>

      <RibGroup name="Modify">
        <RibCol>
          {tool("fillet")}
          {tool("trim")}
          {tool("extend")}
        </RibCol>
        <RibCol>
          {tool("offset")}
          {tool("mirror")}
          {tool("patternRect")}
        </RibCol>
        <RibCol>
          {tool("patternCirc")}
          {numberField(
            "Radius",
            options.filletRadius * 1000,
            1,
            (v) => api.setToolOptions({ filletRadius: v / 1000 }),
            "mm",
          )}
          {numberField(
            "Offset",
            options.offsetDistance * 1000,
            1,
            (v) => api.setToolOptions({ offsetDistance: v / 1000 }),
            "mm",
          )}
        </RibCol>
        <RibCol>
          {numberField("Copies", options.patternCount, 1, (v) =>
            api.setToolOptions({ patternCount: Math.max(2, Math.round(v)) }),
          )}
          {numberField(
            "Spacing",
            options.patternSpacing * 1000,
            1,
            (v) => api.setToolOptions({ patternSpacing: v / 1000 }),
            "mm",
          )}
          {numberField(
            "Sweep",
            Math.round((options.patternAngle * 180) / Math.PI),
            15,
            (v) => api.setToolOptions({ patternAngle: (v * Math.PI) / 180 }),
            "°",
          )}
        </RibCol>
        {(api.sketchTool === "mirror" ||
          api.sketchTool === "patternRect" ||
          api.sketchTool === "patternCirc") && (
          <RibHint>
            {api.sketchSelection.length === 0
              ? "Select what to copy first, with the Select tool"
              : api.sketchTool === "mirror"
                ? "Now click the line to mirror across"
                : api.sketchTool === "patternRect"
                  ? "Now click to set the direction"
                  : "Now click the centre to revolve around"}
          </RibHint>
        )}
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

    </>
  );
}

/**
 * Solve / align / finish. The ribbon renders this outside its scrolling area
 * so leaving a sketch never depends on scrolling to find the button.
 */
export function SketchExitGroup({ api }: { api: AppApi }) {
  const sketch = api.activeSketch;
  if (!sketch) return null;
  return (
    <RibGroup name="Exit" pinned>
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
          <RibSmall
            icon="sketch"
            label="Sketches"
            hint="Keep finished sketches visible in the 3D view"
            active={api.showSketches}
            onClick={() => api.setShowSketches(!api.showSketches)}
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
