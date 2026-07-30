// The handle every chrome component works through.
//
// App owns the state; the ribbon, browser and inspector are given this one
// object instead of a dozen props each. Everything that mutates the document
// still goes through `run`, so the command/event path is unchanged.

import type { MenuEntry } from "../components/ContextMenu";
import type {
  PartInfo,
  RunCommands,
  SceneSnapshot,
  SketchGeometry,
  SketchInfo,
} from "../engine/api";
import type { StlUnit } from "../engine/commands";
import type { ConstraintDef } from "../engine/constraints";
import type { RegionPick } from "../scene/idleSketches";
import type { SketchTool, ToolOptions } from "../scene/sketchTools";
import type { GizmoMode, Viewport } from "../scene/viewport";
import type { DockId, Layout, PanelId } from "./layout";

/** A dimension the user started, waiting for its value to be accepted */
export interface PendingDimension {
  def: ConstraintDef;
  /** Entity IDs in slot order */
  ids: string[];
  /** In the definition's display unit (mm or degrees) */
  value: number;
}

/** Modeless command dialogs that float over the viewport */
export type DialogKind = "extrude" | "revolve";

/** A one-line text entry, for the Rename items on the context menus */
export interface TextPrompt {
  title: string;
  value: string;
  /** Cancelling simply never calls this */
  onAccept(value: string): void;
}

export interface AppApi {
  snapshot: SceneSnapshot | null;
  selected: string | null;
  selectedPart: PartInfo | null;
  select(partId: string | null): void;

  gizmoMode: GizmoMode;
  setGizmoMode(mode: GizmoMode): void;

  activeSketch: SketchInfo | null;
  activateSketch(sketchId: string | null): void;
  sketchTool: SketchTool;
  setSketchTool(tool: SketchTool): void;
  sketchSelection: string[];
  setSketchSelection(entityIds: string[]): void;
  sketchGeometry: SketchGeometry | null;
  /** Entities to light up in the viewport while hovering a constraint */
  hoverSketch(entityIds: string[]): void;
  /** Numbers the tools need up front: polygon sides, fillet radius, ... */
  toolOptions: ToolOptions;
  setToolOptions(options: Partial<ToolOptions>): void;
  /** Variant each ribbon tool family last ran, keyed by family id */
  toolVariant: Record<string, SketchTool>;

  /** Waiting for the user to click a plane or a face in the 3D view */
  pickingPlane: boolean;
  beginPlanePick(): void;
  cancelPlanePick(): void;

  /** Enclosed sketch areas the user has clicked, for the next extrude */
  regionSelection: RegionPick[];
  setRegionSelection(selection: RegionPick[]): void;

  pendingDimension: PendingDimension | null;
  setPendingDimension(pending: PendingDimension | null): void;
  dialog: DialogKind | null;
  setDialog(kind: DialogKind | null): void;

  showCollisions: boolean;
  setShowCollisions(visible: boolean): void;
  showGrid: boolean;
  setShowGrid(visible: boolean): void;
  showSketches: boolean;
  setShowSketches(visible: boolean): void;

  // ---- panels ----
  /** Which dock each panel sits in, and which of them are floating */
  layout: Layout;
  updateLayout(next: Layout): void;
  movePanel(panel: PanelId, dock: DockId, index: number): void;
  /** Tear a panel off into its own window at that spot on the desktop */
  floatPanel(panel: PanelId, screenX: number, screenY: number): Promise<void>;
  hidePanel(panel: PanelId): void;
  togglePanel(panel: PanelId): void;
  /**
   * Set when this window shows one torn-off panel rather than the whole app;
   * chrome (ribbon, title bar, status bar) is left out in that case
   */
  soloPanel: PanelId | null;

  // ---- transient UI ----
  openMenu(x: number, y: number, entries: MenuEntry[]): void;
  askText(prompt: TextPrompt): void;

  /** Unit meshes are interpreted in when imported */
  meshUnit: StlUnit;
  setMeshUnit(unit: StlUnit): void;

  viewport(): Viewport | null;
  run: RunCommands;
  setStatus(text: string): void;
}
