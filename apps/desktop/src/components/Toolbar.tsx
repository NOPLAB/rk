import { open, save } from "@tauri-apps/plugin-dialog";
import type { Command, SceneSnapshot } from "../engine/api";
import {
  createBox,
  createCylinder,
  createSphere,
  deletePart,
  loadDocument,
  newDocument,
  redo,
  saveDocument,
  undo,
} from "../engine/commands";

const RK_FILTER = [{ name: "RK Project", extensions: ["rk"] }];

interface Props {
  snapshot: SceneSnapshot | null;
  selected: string | null;
  run: (commands: Command[]) => Promise<void>;
  onDeselect: () => void;
}

export function Toolbar({ snapshot, selected, run, onDeselect }: Props) {
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
