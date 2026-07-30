// Panels that live in their own OS window.
//
// A torn-off panel runs the same app in a second webview; the two share
// nothing but the engine, which is in Rust. So each window keeps its own
// scene and re-pulls whenever the other one changes the document — that is
// what `rk://document-changed` is for.

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isPanelId, type PanelId } from "./layout";

const PANEL_PREFIX = "panel-";
const DOCUMENT_CHANGED = "rk://document-changed";
const PANEL_CLOSED = "rk://panel-closed";

function label(): string {
  try {
    return getCurrentWindow().label;
  } catch {
    // Outside Tauri (a plain `vite dev` browser tab) there is no window label
    return "main";
  }
}

/** The single panel this window renders, or `null` for the main window */
export function currentPanel(): PanelId | null {
  const name = label();
  if (!name.startsWith(PANEL_PREFIX)) return null;
  const panel = name.slice(PANEL_PREFIX.length);
  return isPanelId(panel) ? panel : null;
}

/**
 * Another window edited the document. The engine has already applied it —
 * this window only has to re-read, so the callback does a full re-sync.
 */
export function onDocumentChanged(run: () => void): Promise<UnlistenFn> {
  const self = label();
  return listen<{ origin: string }>(DOCUMENT_CHANGED, (event) => {
    if (event.payload.origin !== self) run();
  });
}

/** A floating panel's window was closed; the main window docks it back */
export function onPanelClosed(run: (panel: PanelId) => void): Promise<UnlistenFn> {
  return listen<string>(PANEL_CLOSED, (event) => {
    if (isPanelId(event.payload)) run(event.payload);
  });
}
