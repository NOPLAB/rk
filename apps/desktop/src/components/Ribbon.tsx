// Ribbon: the tab strip plus the panel row underneath it, and the File tab's
// drop-down menu. Entering a sketch switches to the Sketch tab and leaving it
// goes back to 3D Model, the way Inventor follows the modelling context.

import { useEffect, useState } from "react";
import type { StlUnit } from "../engine/commands";
import { undo } from "../engine/commands";
import type { AppApi } from "../ui/appApi";
import { fileActions } from "../ui/fileActions";
import { Icon, type IconName } from "./icons";
import {
  AssemblyTab,
  ModelTab,
  SketchExitGroup,
  SketchTab,
  ViewTab,
} from "./ribbonTabs";

type TabId = "model" | "sketch" | "assembly" | "view";

const TABS: { id: TabId; label: string }[] = [
  { id: "model", label: "3D Model" },
  { id: "sketch", label: "Sketch" },
  { id: "assembly", label: "Assembly" },
  { id: "view", label: "View" },
];

const STL_UNITS: StlUnit[] = ["Millimeters", "Meters", "Centimeters", "Inches"];

export function Ribbon({ api }: { api: AppApi }) {
  const [tab, setTab] = useState<TabId>("model");
  const [menuOpen, setMenuOpen] = useState(false);
  const sketchId = api.activeSketch?.id ?? null;

  // Follow the modelling context: a sketch being edited owns the ribbon
  useEffect(() => {
    setTab(sketchId ? "sketch" : "model");
  }, [sketchId]);

  // Leaving a sketch must never depend on scrolling the ribbon to find the
  // button, so that one group is pinned over the right edge
  const pinned = tab === "sketch" && api.activeSketch !== null;

  return (
    <div className="ribbon">
      <div className="ribbon-tabs">
        <button
          className="ribbon-tab file"
          onClick={() => setMenuOpen((open) => !open)}
        >
          File
        </button>
        {TABS.map(({ id, label }) => (
          <button
            key={id}
            className={`ribbon-tab${tab === id ? " active" : ""}`}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="ribbon-body">
        <div className={pinned ? "rb-scroll has-pinned" : "rb-scroll"}>
          {tab === "model" && <ModelTab api={api} />}
          {tab === "sketch" && <SketchTab api={api} />}
          {tab === "assembly" && <AssemblyTab api={api} />}
          {tab === "view" && <ViewTab api={api} />}
        </div>
        {/* The Sketch tab holds more than fits at any sane window width, so
            the way out of it rides on top of the scrolling area instead */}
        {pinned && <SketchExitGroup api={api} />}
      </div>

      {menuOpen && <FileMenu api={api} onClose={() => setMenuOpen(false)} />}
    </div>
  );
}

function FileMenu({ api, onClose }: { api: AppApi; onClose: () => void }) {
  const files = fileActions(api);
  const hasParts = (api.snapshot?.parts.length ?? 0) > 0;

  const item = (
    icon: IconName,
    label: string,
    action: () => Promise<unknown> | void,
    disabled = false,
  ) => (
    <button
      disabled={disabled}
      onClick={() => {
        onClose();
        void action();
      }}
    >
      <Icon name={icon} size={16} />
      <span>{label}</span>
    </button>
  );

  return (
    <>
      <div className="file-menu-backdrop" onClick={onClose} />
      <div className="file-menu">
        {item("new", "New", files.onNew)}
        {item("open", "Open…", files.onOpen)}
        {item("save", "Save", files.onSave)}
        {item("saveAs", "Save As…", files.onSaveAs)}
        <div className="sep" />
        {item("importMesh", "Import Mesh…", files.onImportMesh)}
        {item("importUrdf", "Import URDF…", files.onImportUrdf)}
        {item("exportUrdf", "Export URDF…", files.onExportUrdf, !hasParts)}
        <div className="menu-note">
          <span>Mesh unit</span>
          <select
            value={api.meshUnit}
            onChange={(e) => api.setMeshUnit(e.target.value as StlUnit)}
          >
            {STL_UNITS.map((unit) => (
              <option key={unit} value={unit}>
                {unit}
              </option>
            ))}
          </select>
        </div>
        <div className="sep" />
        {item(
          "undo",
          api.snapshot?.history.undo_description
            ? `Undo ${api.snapshot.history.undo_description}`
            : "Undo",
          () => api.run([undo()]),
          !api.snapshot?.history.can_undo,
        )}
      </div>
    </>
  );
}
