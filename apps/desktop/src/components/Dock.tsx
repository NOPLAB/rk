// One dock: a tab strip and the panels under it.
//
// Every panel of a dock stays mounted and the inactive ones are merely
// hidden — the 3D view owns a WebGL context that unmounting would throw
// away. Moving a panel to another dock does remount it, and the viewport
// carries its camera across; switching tabs never does.

import type { ReactNode } from "react";
import {
  DOCK_IDS,
  PANELS,
  activatePanel,
  dockGrows,
  type DockId,
  type PanelId,
} from "../ui/layout";
import { startPanelDrag } from "../ui/panelDrag";
import type { AppApi } from "../ui/appApi";
import { panelTabMenu } from "../ui/menus";
import { Icon } from "./icons";

export function Dock({
  api,
  dock,
  render,
}: {
  api: AppApi;
  dock: DockId;
  render: (panel: PanelId) => ReactNode;
}) {
  const layout = api.layout;
  const tabs = layout.docks[dock];
  const grows = dockGrows(layout, dock);

  // An empty dock stays as a thin rail so a tab can always be dragged back
  if (tabs.length === 0 && !grows) {
    return <div className="dock rail" data-dock={dock} />;
  }

  const active = layout.active[dock] ?? tabs[0] ?? null;

  return (
    <section className={grows ? "dock grows" : "dock"} data-dock={dock}>
      <div className="panel-tabs" data-tabstrip>
        {tabs.map((panel) => (
          <button
            key={panel}
            data-panel={panel}
            className={`panel-tab${panel === active ? " active" : ""}`}
            title={`${PANELS[panel].title} — drag to move, or out of the window to float`}
            // Activating happens here rather than in `onClick`: a click
            // fires after a drag too, and its handler still holds the layout
            // from before the move — using it would undo the drop
            onPointerDown={(e) =>
              startPanelDrag(e, PANELS[panel].title, (drop) => {
                if (!drop) {
                  api.updateLayout(activatePanel(layout, panel));
                } else if (drop.kind === "outside") {
                  void api.floatPanel(panel, drop.screenX, drop.screenY);
                } else if (DOCK_IDS.includes(drop.dock as DockId)) {
                  api.movePanel(panel, drop.dock as DockId, drop.index);
                }
              })
            }
            onContextMenu={(e) => {
              e.preventDefault();
              api.openMenu(e.clientX, e.clientY, panelTabMenu(api, panel));
            }}
          >
            <Icon name={PANELS[panel].icon} size={13} />
            <span>{PANELS[panel].title}</span>
            <span
              className="tab-close"
              title="Close this panel"
              onPointerDown={(e) => e.stopPropagation()}
              onClick={(e) => {
                e.stopPropagation();
                api.hidePanel(panel);
              }}
            >
              <Icon name="close" size={10} />
            </span>
          </button>
        ))}
      </div>
      <div className="dock-body">
        {tabs.map((panel) => (
          <div
            key={panel}
            className="dock-slot"
            style={panel === active ? undefined : { display: "none" }}
          >
            {render(panel)}
          </div>
        ))}
      </div>
    </section>
  );
}
