// What each right-click offers.
//
// One module for every context menu in the app, so the same object gets the
// same commands wherever it is clicked — a part in the browser tree and the
// same part in the 3D view read identically. The menus follow Fusion: the
// object's own commands first, then what to do with the view.

import type { MenuEntry } from "../components/ContextMenu";
import type {
  FeatureGroupInfo,
  FeatureInfo,
  JointInfo,
  PartInfo,
  SketchInfo,
} from "../engine/api";
import {
  deleteFeature,
  deletePart,
  deleteSketch,
  deleteSketchEntities,
  disconnectPart,
  groupFeatures,
  renameFeature,
  renameFeatureGroup,
  renamePart,
  renameSketch,
  resetJointPosition,
  rollbackTo,
  setFeatureGroupCollapsed,
  setFeatureSuppressed,
  setJointType,
  setSketchConstruction,
  solveSketch,
  ungroupFeatures,
} from "../engine/commands";
import type { OriginPlane } from "../scene/planePicker";
import type { RegionPick } from "../scene/idleSketches";
import type { AppApi } from "./appApi";
import { DOCK_IDS, PANELS, dockOf, type DockId, type PanelId } from "./layout";
import { createSketchOnOrigin } from "./sketchActions";

const SEP: MenuEntry = { kind: "sep" };

/** Look straight at things, fit them, go home — offered from every menu */
function viewEntries(api: AppApi): MenuEntry[] {
  return [
    { icon: "fit", label: "Zoom Fit", onClick: () => api.viewport()?.fitCamera() },
    { icon: "home", label: "Home View", onClick: () => api.viewport()?.homeView() },
    {
      icon: "grid",
      label: "Grid",
      checked: api.showGrid,
      onClick: () => api.setShowGrid(!api.showGrid),
    },
    {
      icon: "sketch",
      label: "Show Sketches",
      checked: api.showSketches,
      onClick: () => api.setShowSketches(!api.showSketches),
    },
    {
      icon: "collision",
      label: "Show Collisions",
      checked: api.showCollisions,
      onClick: () => api.setShowCollisions(!api.showCollisions),
    },
  ];
}

// ---- parts ---------------------------------------------------------------

export function partMenu(api: AppApi, part: PartInfo): MenuEntry[] {
  const joint =
    api.snapshot?.joints.find((j) => j.child_part === part.id) ?? null;
  return [
    { kind: "header", label: part.name },
    {
      icon: "move",
      label: "Move",
      hint: "Show the move gizmo (W)",
      onClick: () => {
        api.select(part.id);
        api.setGizmoMode("translate");
      },
    },
    {
      icon: "rotate",
      label: "Rotate",
      hint: "Show the rotate gizmo (E)",
      onClick: () => {
        api.select(part.id);
        api.setGizmoMode("rotate");
      },
    },
    {
      icon: "inspectorPanel",
      label: "Properties",
      hint: "Open the inspector on this part",
      onClick: () => {
        api.select(part.id);
        if (!dockOf(api.layout, "inspector")) api.togglePanel("inspector");
      },
    },
    SEP,
    {
      icon: "rename",
      label: "Rename…",
      onClick: () =>
        api.askText({
          title: "Rename part",
          value: part.name,
          onAccept: (name) => void api.run([renamePart(part.id, name)]),
        }),
    },
    {
      icon: "disconnect",
      label: "Disconnect",
      hint: "Detach this part from its parent",
      disabled: !joint,
      onClick: () => void api.run([disconnectPart(part.id)]),
    },
    {
      icon: "trash",
      label: "Delete",
      danger: true,
      onClick: () => {
        if (api.selected === part.id) api.select(null);
        void api.run([deletePart(part.id)]);
      },
    },
    SEP,
    ...viewEntries(api),
  ];
}

// ---- sketches ------------------------------------------------------------

/** Extrude / Revolve, the two things a sketch's regions can become */
function buildEntries(api: AppApi, sketch: SketchInfo): MenuEntry[] {
  const picked = api.regionSelection.length;
  const open = (kind: "extrude" | "revolve") => () => {
    api.activateSketch(null);
    api.setDialog(kind);
  };
  return [
    {
      icon: "extrude",
      label: picked > 1 ? `Extrude (${picked} regions)` : "Extrude",
      hint:
        sketch.profile_count === 0
          ? "This sketch encloses no region yet"
          : "Build a solid from this sketch",
      disabled: sketch.profile_count === 0,
      onClick: open("extrude"),
    },
    {
      icon: "revolve",
      label: picked > 1 ? `Revolve (${picked} regions)` : "Revolve",
      disabled: sketch.profile_count === 0,
      onClick: open("revolve"),
    },
  ];
}

export function sketchMenu(api: AppApi, sketch: SketchInfo): MenuEntry[] {
  return [
    { kind: "header", label: sketch.name },
    ...sketchEntries(api, sketch),
    SEP,
    ...buildEntries(api, sketch),
    SEP,
    ...sketchAdminEntries(api, sketch),
  ];
}

/** Edit / align / solve — the sketch itself, without what it can build */
function sketchEntries(api: AppApi, sketch: SketchInfo): MenuEntry[] {
  const editing = api.activeSketch?.id === sketch.id;
  return [
    editing
      ? {
          icon: "finish",
          label: "Finish Sketch",
          onClick: () => api.activateSketch(null),
        }
      : {
          icon: "sketch",
          label: "Edit Sketch",
          onClick: () => api.activateSketch(sketch.id),
        },
    {
      icon: "align",
      label: "Align View",
      hint: "Look straight down this sketch's plane",
      onClick: () => {
        api.activateSketch(sketch.id);
        api.viewport()?.alignToSketch();
      },
    },
    {
      icon: "solve",
      label: "Solve",
      onClick: () => void api.run([solveSketch(sketch.id)]),
    },
  ];
}

/** Rename / delete the sketch itself */
function sketchAdminEntries(api: AppApi, sketch: SketchInfo): MenuEntry[] {
  return [
    {
      icon: "rename",
      label: "Rename…",
      onClick: () =>
        api.askText({
          title: "Rename sketch",
          value: sketch.name,
          onAccept: (name) => void api.run([renameSketch(sketch.id, name)]),
        }),
    },
    {
      icon: "trash",
      label: "Delete",
      danger: true,
      onClick: () => {
        if (api.activeSketch?.id === sketch.id) api.activateSketch(null);
        void api.run([deleteSketch(sketch.id)]);
      },
    },
  ];
}

/** Right-click inside a sketch being edited */
export function sketchEditMenu(api: AppApi): MenuEntry[] {
  const sketch = api.activeSketch;
  if (!sketch) return [];
  const selection = api.sketchSelection;
  const geometry = api.sketchGeometry;
  const curves = geometry
    ? [
        ...geometry.lines,
        ...geometry.circles,
        ...geometry.arcs,
        ...geometry.ellipses,
        ...geometry.splines,
      ]
    : [];
  const anyNormal = selection.some((id) =>
    curves.some((c) => c.id === id && !c.construction),
  );

  return [
    { kind: "header", label: sketch.name },
    {
      icon: "select",
      label: "Cancel Tool",
      hint: "Back to the select tool",
      disabled: api.sketchTool === "select",
      onClick: () => api.setSketchTool("select"),
    },
    {
      icon: "trash",
      label: `Delete Selection (${selection.length})`,
      disabled: selection.length === 0,
      danger: true,
      onClick: () => {
        api.setSketchSelection([]);
        void api.run([deleteSketchEntities(sketch.id, selection)]);
      },
    },
    {
      icon: "construction",
      label: anyNormal ? "Make Construction" : "Make Normal",
      disabled: selection.length === 0,
      onClick: () =>
        void api.run([setSketchConstruction(sketch.id, selection, anyNormal)]),
    },
    SEP,
    {
      icon: "solve",
      label: "Solve",
      onClick: () => void api.run([solveSketch(sketch.id)]),
    },
    {
      icon: "align",
      label: "Align View",
      onClick: () => api.viewport()?.alignToSketch(),
    },
    {
      icon: "finish",
      label: "Finish Sketch",
      onClick: () => api.activateSketch(null),
    },
  ];
}

/**
 * Right-click on a filled region of a finished sketch. What the region can
 * become comes first — that is why the region was clicked — and the sketch's
 * own commands follow, without repeating Extrude and Revolve.
 */
export function regionMenu(api: AppApi, pick: RegionPick): MenuEntry[] {
  const sketch = api.snapshot?.sketches.find((s) => s.id === pick.sketchId);
  if (!sketch) return [];
  return [
    { kind: "header", label: `${sketch.name} — region` },
    ...buildEntries(api, sketch),
    SEP,
    ...sketchEntries(api, sketch),
    SEP,
    ...sketchAdminEntries(api, sketch),
  ];
}

// ---- origin planes -------------------------------------------------------

export function originPlaneMenu(api: AppApi, plane: OriginPlane): MenuEntry[] {
  return [
    { kind: "header", label: `${plane} Plane` },
    {
      icon: "sketch",
      label: "Create Sketch",
      onClick: () => void createSketchOnOrigin(api, plane),
    },
    {
      icon: "viewFront",
      label: "Look At",
      onClick: () =>
        api
          .viewport()
          ?.lookFrom(
            plane === "XY" ? [0, 0, 1] : plane === "XZ" ? [0, -1, 0] : [1, 0, 0],
          ),
    },
  ];
}

// ---- features and groups -------------------------------------------------

export function featureMenu(
  api: AppApi,
  feature: FeatureInfo,
  selection: string[],
): MenuEntry[] {
  // Grouping acts on the whole multi-selection, everything else on the row
  const picked = selection.includes(feature.id) ? selection : [feature.id];
  const group = api.snapshot?.feature_groups.find((g) =>
    g.members.includes(feature.id),
  );
  return [
    { kind: "header", label: feature.name },
    {
      icon: "suppress",
      label: feature.suppressed ? "Unsuppress" : "Suppress",
      onClick: () =>
        void api.run([setFeatureSuppressed(feature.id, !feature.suppressed)]),
    },
    {
      icon: "rollback",
      label: "Roll Back To Here",
      onClick: () => void api.run([rollbackTo(feature.id)]),
    },
    SEP,
    {
      icon: "folder",
      label: picked.length > 1 ? `Group (${picked.length})` : "Group",
      hint: "Bundle these timeline features under one name",
      onClick: () =>
        api.askText({
          title: "Group features",
          value: `Group ${(api.snapshot?.feature_groups.length ?? 0) + 1}`,
          onAccept: (name) => void api.run([groupFeatures(picked, name)]),
        }),
    },
    {
      icon: "folder",
      label: "Ungroup",
      disabled: !group,
      onClick: () => group && void api.run([ungroupFeatures(group.id)]),
    },
    SEP,
    {
      icon: "rename",
      label: "Rename…",
      onClick: () =>
        api.askText({
          title: "Rename feature",
          value: feature.name,
          onAccept: (name) => void api.run([renameFeature(feature.id, name)]),
        }),
    },
    {
      icon: "trash",
      label: picked.length > 1 ? `Delete (${picked.length})` : "Delete",
      danger: true,
      onClick: () => void api.run(picked.map(deleteFeature), picked.length > 1),
    },
  ];
}

export function featureGroupMenu(
  api: AppApi,
  group: FeatureGroupInfo,
): MenuEntry[] {
  const members = (api.snapshot?.features ?? []).filter((f) =>
    group.members.includes(f.id),
  );
  const anyActive = members.some((f) => !f.suppressed);
  return [
    { kind: "header", label: group.name },
    {
      icon: "chevron",
      label: group.collapsed ? "Expand" : "Collapse",
      onClick: () =>
        void api.run([setFeatureGroupCollapsed(group.id, !group.collapsed)]),
    },
    {
      icon: "suppress",
      label: anyActive ? "Suppress Features" : "Unsuppress Features",
      onClick: () =>
        void api.run(
          members.map((f) => setFeatureSuppressed(f.id, anyActive)),
          true,
        ),
    },
    SEP,
    {
      icon: "rename",
      label: "Rename…",
      onClick: () =>
        api.askText({
          title: "Rename group",
          value: group.name,
          onAccept: (name) => void api.run([renameFeatureGroup(group.id, name)]),
        }),
    },
    {
      icon: "folder",
      label: "Ungroup",
      hint: "Dissolve the group; the features stay",
      onClick: () => void api.run([ungroupFeatures(group.id)]),
    },
    {
      icon: "trash",
      label: `Delete Features (${members.length})`,
      danger: true,
      onClick: () =>
        void api.run(
          members.map((f) => deleteFeature(f.id)),
          true,
        ),
    },
  ];
}

// ---- joints --------------------------------------------------------------

const JOINT_TYPES = ["Fixed", "Revolute", "Continuous", "Prismatic"] as const;

export function jointMenu(api: AppApi, joint: JointInfo): MenuEntry[] {
  return [
    { kind: "header", label: joint.name },
    ...JOINT_TYPES.map(
      (type): MenuEntry => ({
        icon: "joint",
        label: type,
        checked: joint.joint_type === type,
        onClick: () => void api.run([setJointType(joint.id, type)]),
      }),
    ),
    SEP,
    {
      icon: "reset",
      label: "Reset Position",
      onClick: () => void api.run([resetJointPosition(joint.id)]),
    },
    {
      icon: "disconnect",
      label: "Disconnect",
      danger: true,
      disabled: !joint.child_part,
      onClick: () =>
        joint.child_part && void api.run([disconnectPart(joint.child_part)]),
    },
  ];
}

// ---- viewport background -------------------------------------------------

export function viewportMenu(api: AppApi): MenuEntry[] {
  return [
    {
      icon: "sketch",
      label: "Create Sketch",
      hint: "Then click a plane or a flat face",
      onClick: () => api.beginPlanePick(),
    },
    SEP,
    { icon: "viewIso", label: "Isometric", onClick: () => api.viewport()?.setStandardView("iso") },
    { icon: "viewFront", label: "Front", onClick: () => api.viewport()?.setStandardView("front") },
    { icon: "viewTop", label: "Top", onClick: () => api.viewport()?.setStandardView("top") },
    { icon: "viewRight", label: "Right", onClick: () => api.viewport()?.setStandardView("right") },
    SEP,
    ...viewEntries(api),
  ];
}

// ---- panel tabs ----------------------------------------------------------

const DOCK_NAMES: Record<DockId, string> = {
  left: "Left",
  main: "Centre",
  right: "Right",
};

export function panelTabMenu(api: AppApi, panel: PanelId): MenuEntry[] {
  const here = dockOf(api.layout, panel);
  return [
    { kind: "header", label: PANELS[panel].title },
    {
      icon: "float",
      label: "Float in New Window",
      hint: "Open this panel on its own — drop it on a second display",
      onClick: () => void api.floatPanel(panel, 120, 120),
    },
    SEP,
    ...DOCK_IDS.map(
      (dock): MenuEntry => ({
        icon: "browserPanel",
        label: `Move to ${DOCK_NAMES[dock]}`,
        checked: here === dock,
        disabled: here === dock,
        onClick: () => api.movePanel(panel, dock, api.layout.docks[dock].length),
      }),
    ),
    SEP,
    {
      icon: "close",
      label: "Close Panel",
      onClick: () => api.hidePanel(panel),
    },
  ];
}
