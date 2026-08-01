// Document tab strip and the status line along the bottom of the window.

import { TOOLS } from "../scene/sketchToolInfo";
import type { AppApi } from "../ui/appApi";
import { Icon } from "./icons";

export function DocTabs({ api }: { api: AppApi }) {
  const snapshot = api.snapshot;
  const name = snapshot?.project_name || "Untitled";
  return (
    <div className="doc-tabs">
      <button className="doc-tab active" title={snapshot?.doc_path ?? "Unsaved"}>
        <Icon name="new" size={13} />
        <span>
          {name}
          {snapshot?.modified ? " *" : ""}
        </span>
      </button>
    </div>
  );
}

export function StatusBar({ api, status }: { api: AppApi; status: string }) {
  const snapshot = api.snapshot;
  const sketch = api.activeSketch;
  const failed = status.startsWith("Error");

  const regions = api.regionSelection.length;
  const message = status
    ? status
    : api.pickingPlane
      ? "Click a plane or a flat face to sketch on — Esc cancels"
      : sketch
        ? TOOLS[api.sketchTool].prompt
        : regions > 0
          ? `${regions} sketch region${regions > 1 ? "s" : ""} selected — Extrude or Revolve to build`
          : api.selectedPart
            ? `${api.selectedPart.name} selected`
            : "Ready";

  return (
    <footer className="status">
      <span className={failed ? "error" : ""}>{message}</span>
      <span className="spacer" />
      {sketch && (
        <span className="chip">
          {sketch.name} · {sketch.dof} DOF
          {sketch.profile_count > 0 && ` · ${sketch.profile_count}p`}
        </span>
      )}
      {snapshot && (
        <span className="chip">
          {snapshot.parts.length}p {snapshot.joints.length}j{" "}
          {snapshot.body_ids.length}b
        </span>
      )}
      <span className="chip">{snapshot?.doc_path ?? "unsaved"}</span>
    </footer>
  );
}
