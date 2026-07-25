import { useCallback, useEffect, useRef, useState } from "react";
import {
  applyCommands,
  sceneSnapshot,
  type Command,
  type SceneSnapshot,
} from "./engine/api";
import { Viewport } from "./scene/viewport";
import { Toolbar } from "./components/Toolbar";
import { PartList } from "./components/PartList";
import { PropertiesPanel } from "./components/PropertiesPanel";

export default function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<Viewport | null>(null);
  const [snapshot, setSnapshot] = useState<SceneSnapshot | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [status, setStatus] = useState("");

  const refresh = useCallback(async () => {
    const snap = await sceneSnapshot();
    setSnapshot(snap);
    viewportRef.current?.setTransforms(snap.transforms);
    setSelected((sel) =>
      sel && !snap.parts.some((p) => p.id === sel) ? null : sel,
    );
    return snap;
  }, []);

  /** Apply a command batch, sync the 3D scene from the events, refresh UI state */
  const run = useCallback(
    async (commands: Command[]) => {
      try {
        const outcome = await applyCommands(commands);
        setStatus(outcome.error ? `Error: ${outcome.error.message}` : "");
        await viewportRef.current?.applyEvents(outcome.events);
        await refresh();
      } catch (e) {
        setStatus(`Error: ${e}`);
      }
    },
    [refresh],
  );

  const select = useCallback((partId: string | null) => {
    setSelected(partId);
    viewportRef.current?.setSelected(partId);
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const vp = new Viewport(canvas);
    viewportRef.current = vp;
    vp.onPick = (id) => {
      setSelected(id);
      vp.setSelected(id);
    };

    const ro = new ResizeObserver(() =>
      vp.resize(container.clientWidth, container.clientHeight),
    );
    ro.observe(container);
    vp.resize(container.clientWidth, container.clientHeight);

    void (async () => {
      const snap = await sceneSnapshot();
      setSnapshot(snap);
      await vp.rebuildFromSnapshot(snap);
    })();

    return () => {
      ro.disconnect();
      vp.dispose();
      viewportRef.current = null;
    };
  }, []);

  const selectedPart =
    (selected && snapshot?.parts.find((p) => p.id === selected)) || null;
  const title = snapshot
    ? `${snapshot.project_name}${snapshot.modified ? " *" : ""}`
    : "…";

  return (
    <div className="app">
      <Toolbar
        snapshot={snapshot}
        selected={selected}
        run={run}
        onDeselect={() => select(null)}
      />
      <div className="main">
        <aside className="panel left">
          <div className="panel-title">Parts — {title}</div>
          <PartList
            parts={snapshot?.parts ?? []}
            bodyCount={snapshot?.body_ids.length ?? 0}
            selected={selected}
            onSelect={select}
          />
        </aside>
        <div className="viewport" ref={containerRef}>
          <canvas ref={canvasRef} />
        </div>
        <aside className="panel right">
          <div className="panel-title">Properties</div>
          <PropertiesPanel part={selectedPart} run={run} />
        </aside>
      </div>
      <footer className="status">
        <span className={status.startsWith("Error") ? "error" : ""}>
          {status || snapshot?.doc_path || "unsaved project"}
        </span>
        <span className="spacer" />
        <span>
          {snapshot?.history.can_undo
            ? `undo: ${snapshot.history.undo_description ?? ""}`
            : ""}
        </span>
      </footer>
    </div>
  );
}
