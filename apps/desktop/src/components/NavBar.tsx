// Navigation bar down the right edge of the viewport, under the ViewCube —
// the same shortcuts Inventor keeps there.

import type { StandardView } from "../scene/viewport";
import type { AppApi } from "../ui/appApi";
import { Icon, type IconName } from "./icons";

const VIEWS: { view: StandardView; icon: IconName; title: string }[] = [
  { view: "front", icon: "viewFront", title: "Front view" },
  { view: "top", icon: "viewTop", title: "Top view" },
  { view: "right", icon: "viewRight", title: "Right view" },
  { view: "iso", icon: "viewIso", title: "Isometric view" },
];

export function NavBar({ api }: { api: AppApi }) {
  return (
    <div className="navbar">
      <button
        className="nav-btn"
        title="Home view"
        onClick={() => api.viewport()?.homeView()}
      >
        <Icon name="home" size={16} />
      </button>
      <button
        className="nav-btn"
        title="Zoom to fit"
        onClick={() => api.viewport()?.fitCamera()}
      >
        <Icon name="fit" size={16} />
      </button>
      <div className="nav-sep" />
      {VIEWS.map(({ view, icon, title }) => (
        <button
          key={view}
          className="nav-btn"
          title={title}
          onClick={() => api.viewport()?.setStandardView(view)}
        >
          <Icon name={icon} size={16} />
        </button>
      ))}
      <div className="nav-sep" />
      <button
        className={`nav-btn${api.showGrid ? " active" : ""}`}
        title="Show the ground grid"
        onClick={() => api.setShowGrid(!api.showGrid)}
      >
        <Icon name="grid" size={16} />
      </button>
    </div>
  );
}
