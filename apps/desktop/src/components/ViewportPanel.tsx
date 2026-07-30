// The 3D view as a dockable panel: the canvas, the navigation bar and the
// modeless dialogs that float over it.
//
// The canvas arrives through a callback ref because moving this panel to
// another dock remounts it — App watches for the new element and rebuilds the
// Viewport onto it, carrying the camera across.

import type { AppApi } from "../ui/appApi";
import { DimensionEntry } from "./DimensionEntry";
import { FeatureDialog } from "./FeatureDialog";
import { NavBar } from "./NavBar";

export function ViewportPanel({
  api,
  canvasRef,
  containerRef,
}: {
  api: AppApi;
  canvasRef: (el: HTMLCanvasElement | null) => void;
  containerRef: (el: HTMLDivElement | null) => void;
}) {
  return (
    <div className="viewport" ref={containerRef}>
      <canvas ref={canvasRef} />
      <NavBar api={api} />
      {api.dialog && <FeatureDialog api={api} kind={api.dialog} />}
      {api.pendingDimension && <DimensionEntry api={api} />}
    </div>
  );
}
