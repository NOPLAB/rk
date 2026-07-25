import { useCallback, useEffect, useRef, useState } from "react";
import * as THREE from "three";
import {
  applyCommands,
  applyInteractive,
  endInteraction,
  sceneSnapshot,
  type Command,
  type SceneSnapshot,
} from "./engine/api";
import { setPartTransform } from "./engine/commands";
import { createCoalescer, newSessionId } from "./engine/interaction";
import { Viewport, type GizmoMode } from "./scene/viewport";
import { Toolbar } from "./components/Toolbar";
import { PartList } from "./components/PartList";
import { PropertiesPanel } from "./components/PropertiesPanel";
import { JointPanel } from "./components/JointPanel";

export default function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<Viewport | null>(null);
  const snapshotRef = useRef<SceneSnapshot | null>(null);
  const [snapshot, setSnapshot] = useState<SceneSnapshot | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [gizmoMode, setGizmoMode] = useState<GizmoMode>("none");
  const [status, setStatus] = useState("");

  const refresh = useCallback(async () => {
    const snap = await sceneSnapshot();
    snapshotRef.current = snap;
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

    // Gizmo drag: every move goes through one interaction session, so the
    // whole drag is a single undo step. Intermediate moves are coalesced —
    // the pointer produces them far faster than the IPC round trip.
    const drags = createCoalescer();
    let drag: { session: string; parentInv: THREE.Matrix4 } | null = null;

    vp.onTransformStart = (partId) => {
      const part = snapshotRef.current?.parts.find((p) => p.id === partId);
      const parentInv = new THREE.Matrix4();
      if (part) parentInv.fromArray(part.parent_transform).invert();
      drag = { session: newSessionId(), parentInv };
    };

    vp.onTransform = (partId, world) => {
      const active = drag;
      if (!active) return;
      // The gizmo works in world space; parts store their origin in the
      // frame of the link that owns them
      const origin = active.parentInv
        .clone()
        .multiply(new THREE.Matrix4().fromArray(world));
      drags.push(async () => {
        const outcome = await applyInteractive(
          active.session,
          setPartTransform(partId, origin.toArray()),
        );
        if (outcome.error) setStatus(`Error: ${outcome.error.message}`);
        await vp.applyEvents(outcome.events);
      });
    };

    vp.onTransformEnd = (canceled) => {
      const active = drag;
      drag = null;
      if (!active) return;
      void drags.finish(async () => {
        const outcome = await endInteraction(active.session, canceled);
        await vp.applyEvents(outcome.events);
        await refresh();
      });
    };

    const ro = new ResizeObserver(() =>
      vp.resize(container.clientWidth, container.clientHeight),
    );
    ro.observe(container);
    vp.resize(container.clientWidth, container.clientHeight);

    void (async () => {
      const snap = await sceneSnapshot();
      snapshotRef.current = snap;
      setSnapshot(snap);
      await vp.rebuildFromSnapshot(snap);
    })();

    return () => {
      ro.disconnect();
      vp.dispose();
      viewportRef.current = null;
    };
  }, [refresh]);

  useEffect(() => {
    viewportRef.current?.setGizmoMode(gizmoMode);
  }, [gizmoMode]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement | null)?.tagName ?? "";
      if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
      if (e.key === "Escape") {
        viewportRef.current?.cancelDrag();
        return;
      }
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      const key = e.key.toLowerCase();
      if (key === "q") setGizmoMode("none");
      else if (key === "w") setGizmoMode("translate");
      else if (key === "e") setGizmoMode("rotate");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
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
        gizmoMode={gizmoMode}
        onGizmoMode={setGizmoMode}
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
          <div className="panel-title">Joints</div>
          <JointPanel snapshot={snapshot} run={run} />
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
