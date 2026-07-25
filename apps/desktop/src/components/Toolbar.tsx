import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Command, SceneSnapshot } from "../engine/api";
import {
  createBox,
  createCylinder,
  createSphere,
  deletePart,
  exportUrdf,
  importMesh,
  importUrdf,
  loadDocument,
  newDocument,
  redo,
  saveDocument,
  undo,
  type StlUnit,
} from "../engine/commands";

const RK_FILTER = [{ name: "RK Project", extensions: ["rk"] }];
const MESH_FILTER = [{ name: "Mesh", extensions: ["stl", "obj", "dae"] }];
const URDF_FILTER = [{ name: "URDF", extensions: ["urdf", "xml"] }];
const STL_UNITS: StlUnit[] = [
  "Millimeters",
  "Meters",
  "Centimeters",
  "Inches",
];

interface Props {
  snapshot: SceneSnapshot | null;
  selected: string | null;
  run: (commands: Command[]) => Promise<void>;
  onDeselect: () => void;
}

export function Toolbar({ snapshot, selected, run, onDeselect }: Props) {
  const [unit, setUnit] = useState<StlUnit>("Millimeters");

  const onOpen = async () => {
    const path = await open({ filters: RK_FILTER, multiple: false });
    if (typeof path === "string") await run([loadDocument(path)]);
  };

  const onSaveAs = async () => {
    const path = await save({ filters: RK_FILTER });
    if (path) await run([saveDocument(path)]);
  };

  const onSave = async () => {
    if (snapshot?.doc_path) await run([saveDocument(null)]);
    else await onSaveAs();
  };

  const onDelete = async () => {
    if (!selected) return;
    onDeselect();
    await run([deletePart(selected)]);
  };

  const onImportMesh = async () => {
    const path = await open({ filters: MESH_FILTER, multiple: false });
    if (typeof path === "string") await run([importMesh(path, unit)]);
  };

  const onImportUrdf = async () => {
    const path = await open({ filters: URDF_FILTER, multiple: false });
    if (typeof path === "string") await run([importUrdf(path, unit)]);
  };

  const onExportUrdf = async () => {
    const path = await save({
      filters: [{ name: "URDF", extensions: ["urdf"] }],
    });
    if (!path) return;
    const name = (snapshot?.project_name || "robot").replace(/\s+/g, "_");
    await run([exportUrdf(path, name)]);
  };

  return (
    <header className="toolbar">
      <div className="group">
        <button onClick={() => void run([newDocument()])}>New</button>
        <button onClick={() => void onOpen()}>Open</button>
        <button onClick={() => void onSave()}>Save</button>
        <button onClick={() => void onSaveAs()}>Save As</button>
      </div>
      <div className="group">
        <button onClick={() => void run([createBox([0.1, 0.1, 0.1])])}>
          + Box
        </button>
        <button onClick={() => void run([createCylinder(0.03, 0.1)])}>
          + Cylinder
        </button>
        <button onClick={() => void run([createSphere(0.05)])}>+ Sphere</button>
        <button disabled={!selected} onClick={() => void onDelete()}>
          Delete
        </button>
      </div>
      <div className="group">
        <select
          title="Mesh import unit"
          value={unit}
          onChange={(e) => setUnit(e.target.value as StlUnit)}
        >
          {STL_UNITS.map((u) => (
            <option key={u} value={u}>
              {u}
            </option>
          ))}
        </select>
        <button onClick={() => void onImportMesh()}>Import Mesh</button>
        <button onClick={() => void onImportUrdf()}>Import URDF</button>
        <button
          disabled={!snapshot || snapshot.parts.length === 0}
          onClick={() => void onExportUrdf()}
        >
          Export URDF
        </button>
      </div>
      <div className="group">
        <button
          disabled={!snapshot?.history.can_undo}
          onClick={() => void run([undo()])}
        >
          Undo
        </button>
        <button
          disabled={!snapshot?.history.can_redo}
          onClick={() => void run([redo()])}
        >
          Redo
        </button>
      </div>
    </header>
  );
}
