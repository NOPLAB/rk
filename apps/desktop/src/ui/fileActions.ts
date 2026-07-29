// Document-level commands shared by the quick-access bar and the File menu.
//
// The native dialogs live here so both entry points behave identically:
// pick a path, then hand a single command to the engine.

import { open, save } from "@tauri-apps/plugin-dialog";
import {
  exportUrdf,
  importMesh,
  importUrdf,
  loadDocument,
  newDocument,
  saveDocument,
} from "../engine/commands";
import type { AppApi } from "./appApi";

const RK_FILTER = [{ name: "RK Project", extensions: ["rk"] }];
const MESH_FILTER = [{ name: "Mesh", extensions: ["stl", "obj", "dae"] }];
const URDF_FILTER = [{ name: "URDF", extensions: ["urdf", "xml"] }];

export interface FileActions {
  onNew(): Promise<void>;
  onOpen(): Promise<void>;
  onSave(): Promise<void>;
  onSaveAs(): Promise<void>;
  onImportMesh(): Promise<void>;
  onImportUrdf(): Promise<void>;
  onExportUrdf(): Promise<void>;
}

export function fileActions(api: AppApi): FileActions {
  const onSaveAs = async () => {
    const path = await save({ filters: RK_FILTER });
    if (path) await api.run([saveDocument(path)]);
  };

  return {
    onNew: async () => {
      api.select(null);
      api.activateSketch(null);
      await api.run([newDocument()]);
    },
    onOpen: async () => {
      const path = await open({ filters: RK_FILTER, multiple: false });
      if (typeof path === "string") {
        api.select(null);
        api.activateSketch(null);
        await api.run([loadDocument(path)]);
      }
    },
    onSave: async () => {
      // Saving over the current file only works once it has one
      if (api.snapshot?.doc_path) await api.run([saveDocument(null)]);
      else await onSaveAs();
    },
    onSaveAs,
    onImportMesh: async () => {
      const path = await open({ filters: MESH_FILTER, multiple: false });
      if (typeof path === "string") {
        await api.run([importMesh(path, api.meshUnit)]);
      }
    },
    onImportUrdf: async () => {
      const path = await open({ filters: URDF_FILTER, multiple: false });
      if (typeof path === "string") {
        api.select(null);
        api.activateSketch(null);
        await api.run([importUrdf(path, api.meshUnit)]);
      }
    },
    onExportUrdf: async () => {
      const path = await save({
        filters: [{ name: "URDF", extensions: ["urdf"] }],
      });
      if (!path) return;
      const name = (api.snapshot?.project_name || "robot").replace(/\s+/g, "_");
      await api.run([exportUrdf(path, name)]);
    },
  };
}
