// Where each panel lives.
//
// The workspace is three docks and every panel — the model browser, the 3D
// view, the inspector — is a tab in one of them, or floating in its own OS
// window, or hidden. That is the whole model: the tab strip, the drag between
// docks and the tear-off all read and write this one structure.

import type { IconName } from "../components/icons";

export type PanelId = "browser" | "viewport" | "inspector";
export type DockId = "left" | "main" | "right";

export interface PanelSpec {
  title: string;
  icon: IconName;
}

export const PANELS: Record<PanelId, PanelSpec> = {
  browser: { title: "Model", icon: "browserPanel" },
  viewport: { title: "3D View", icon: "viewIso" },
  inspector: { title: "Properties", icon: "inspectorPanel" },
};

export const PANEL_IDS = Object.keys(PANELS) as PanelId[];
export const DOCK_IDS: DockId[] = ["left", "main", "right"];

export interface Layout {
  docks: Record<DockId, PanelId[]>;
  active: Record<DockId, PanelId | null>;
  /** Panels that live in their own window; they are in no dock */
  floating: PanelId[];
}

export const DEFAULT_LAYOUT: Layout = {
  docks: { left: ["browser"], main: ["viewport"], right: ["inspector"] },
  active: { left: "browser", main: "viewport", right: "inspector" },
  floating: [],
};

export function isPanelId(value: string): value is PanelId {
  return (PANEL_IDS as string[]).includes(value);
}

export function dockOf(layout: Layout, panel: PanelId): DockId | null {
  return DOCK_IDS.find((d) => layout.docks[d].includes(panel)) ?? null;
}

/** In a dock, floating, or hidden */
export function panelVisible(layout: Layout, panel: PanelId): boolean {
  return dockOf(layout, panel) !== null || layout.floating.includes(panel);
}

/**
 * The dock that stretches. The 3D view is the one panel that wants all the
 * room going, so its dock takes it wherever it has been dropped; with the
 * view floating, the centre dock keeps the layout from collapsing.
 */
export function dockGrows(layout: Layout, dock: DockId): boolean {
  const holder = dockOf(layout, "viewport");
  return holder ? holder === dock : dock === "main";
}

function without(layout: Layout, panel: PanelId): Layout {
  const docks = { ...layout.docks };
  const active = { ...layout.active };
  for (const dock of DOCK_IDS) {
    if (!docks[dock].includes(panel)) continue;
    docks[dock] = docks[dock].filter((p) => p !== panel);
    // A dock whose active tab left falls back to whatever is still there
    if (active[dock] === panel) active[dock] = docks[dock][0] ?? null;
  }
  return {
    docks,
    active,
    floating: layout.floating.filter((p) => p !== panel),
  };
}

/** Drop a panel into `dock` at `index`, taking it out of wherever it was */
export function movePanel(
  layout: Layout,
  panel: PanelId,
  dock: DockId,
  index: number,
): Layout {
  const next = without(layout, panel);
  const tabs = [...next.docks[dock]];
  tabs.splice(Math.max(0, Math.min(index, tabs.length)), 0, panel);
  return {
    ...next,
    docks: { ...next.docks, [dock]: tabs },
    active: { ...next.active, [dock]: panel },
  };
}

/** The panel moved into its own window */
export function floatPanel(layout: Layout, panel: PanelId): Layout {
  const next = without(layout, panel);
  return { ...next, floating: [...next.floating, panel] };
}

/** The panel came back — to the dock it is asked for, else the default one */
export function dockPanel(
  layout: Layout,
  panel: PanelId,
  dock?: DockId,
): Layout {
  const home =
    dock ?? (DOCK_IDS.find((d) => DEFAULT_LAYOUT.docks[d].includes(panel)) ?? "main");
  return movePanel(layout, panel, home, layout.docks[home].length);
}

export function hidePanel(layout: Layout, panel: PanelId): Layout {
  return without(layout, panel);
}

export function activatePanel(layout: Layout, panel: PanelId): Layout {
  const dock = dockOf(layout, panel);
  if (!dock) return layout;
  return { ...layout, active: { ...layout.active, [dock]: panel } };
}

// ---- persistence --------------------------------------------------------

const STORAGE_KEY = "rk.layout";

/**
 * Read the stored layout, repairing anything that no longer makes sense —
 * a panel that has since been renamed, or one listed in two docks at once.
 * A bad layout must never be the reason the window comes up blank.
 */
export function loadLayout(): Layout {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return DEFAULT_LAYOUT;
    const stored = JSON.parse(raw) as Partial<Layout>;
    const seen = new Set<PanelId>();
    const docks = {} as Record<DockId, PanelId[]>;
    for (const dock of DOCK_IDS) {
      docks[dock] = (stored.docks?.[dock] ?? []).filter((p) => {
        if (!isPanelId(p) || seen.has(p)) return false;
        seen.add(p);
        return true;
      });
    }
    const floating = (stored.floating ?? []).filter((p) => {
      if (!isPanelId(p) || seen.has(p)) return false;
      seen.add(p);
      return true;
    });
    const active = {} as Record<DockId, PanelId | null>;
    for (const dock of DOCK_IDS) {
      const wanted = stored.active?.[dock];
      active[dock] =
        wanted && docks[dock].includes(wanted) ? wanted : (docks[dock][0] ?? null);
    }
    return { docks, active, floating };
  } catch {
    return DEFAULT_LAYOUT;
  }
}

export function saveLayout(layout: Layout) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
  } catch {
    // Private mode or a full quota: the layout just will not persist
  }
}
