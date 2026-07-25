import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as THREE from "three";
import {
  applyCommands,
  applyInteractive,
  endInteraction,
  sceneSnapshot,
  type Command,
  type EngineEvent,
  type SceneSnapshot,
} from "./engine/api";
import { deleteSketchEntities, setPartTransform } from "./engine/commands";
import { createCoalescer, newUuid } from "./engine/interaction";
import { Viewport, type GizmoMode } from "./scene/viewport";
import { SketchDrawing, type SketchTool } from "./scene/sketchTools";
import { Toolbar } from "./components/Toolbar";
import { PartList } from "./components/PartList";
import { PropertiesPanel } from "./components/PropertiesPanel";
import { JointPanel } from "./components/JointPanel";
import { SketchPanel } from "./components/SketchPanel";
import { FeaturePanel } from "./components/FeaturePanel";

export default function App() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<Viewport | null>(null);
  const snapshotRef = useRef<SceneSnapshot | null>(null);
  const drawingRef = useRef(new SketchDrawing());
  const [snapshot, setSnapshot] = useState<SceneSnapshot | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [gizmoMode, setGizmoMode] = useState<GizmoMode>("none");
  const [sketchId, setSketchId] = useState<string | null>(null);
  const [sketchTool, setSketchTool] = useState<SketchTool>("select");
  const [sketchSelection, setSketchSelection] = useState<string | null>(null);
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
    async (commands: Command[]): Promise<EngineEvent[]> => {
      try {
        const outcome = await applyCommands(commands);
        setStatus(outcome.error ? `Error: ${outcome.error.message}` : "");
        await viewportRef.current?.applyEvents(outcome.events);
        await refresh();
        return outcome.events;
      } catch (e) {
        setStatus(`Error: ${e}`);
        return [];
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
      drag = { session: newUuid(), parentInv };
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

    // Sketch drawing: the tool state machine turns clicks into whole shapes,
    // so a rectangle reaches the engine as one command and one undo step
    vp.onSketchClick = (hit) => {
      const drawing = drawingRef.current;
      if (drawing.activeTool === "select") {
        setSketchSelection(hit.entityId);
        vp.setSketchSelection(hit.entityId);
        return;
      }
      const commands = drawing.click(hit);
      vp.setSketchPreview(drawing.preview(hit));
      if (commands.length > 0) void run(commands);
    };
    vp.onSketchMove = (hit) => {
      vp.setSketchPreview(drawingRef.current.preview(hit));
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
  }, [refresh, run]);

  useEffect(() => {
    viewportRef.current?.setGizmoMode(gizmoMode);
  }, [gizmoMode]);

  const activeSketch = useMemo(
    () => snapshot?.sketches.find((s) => s.id === sketchId) ?? null,
    [snapshot, sketchId],
  );

  // Entering/leaving sketch mode. Deleting or undoing the sketch away drops
  // `activeSketch` to null, which takes the viewport out of sketch mode too.
  useEffect(() => {
    void viewportRef.current?.setSketch(activeSketch);
    drawingRef.current.setSketch(activeSketch?.id ?? null);
    if (!activeSketch) {
      setSketchSelection(null);
      setSketchTool("select");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeSketch?.id]);

  useEffect(() => {
    drawingRef.current.setTool(sketchTool);
    viewportRef.current?.setSketchPreview(null);
  }, [sketchTool]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement | null)?.tagName ?? "";
      if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
      const vp = viewportRef.current;

      if (e.key === "Escape") {
        if (!sketchId) {
          vp?.cancelDrag();
        } else if (drawingRef.current.busy) {
          drawingRef.current.cancel();
          vp?.setSketchPreview(null);
        } else {
          setSketchTool("select");
        }
        return;
      }
      if (e.ctrlKey || e.metaKey || e.altKey) return;

      if (sketchId) {
        if (
          (e.key === "Delete" || e.key === "Backspace") &&
          sketchSelection !== null
        ) {
          setSketchSelection(null);
          vp?.setSketchSelection(null);
          void run([deleteSketchEntities(sketchId, [sketchSelection])]);
        }
        return; // the gizmo shortcuts below are meaningless while sketching
      }

      const key = e.key.toLowerCase();
      if (key === "q") setGizmoMode("none");
      else if (key === "w") setGizmoMode("translate");
      else if (key === "e") setGizmoMode("rotate");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [run, sketchId, sketchSelection]);

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
          <div className="panel-title">Sketches</div>
          <SketchPanel
            snapshot={snapshot}
            active={activeSketch}
            tool={sketchTool}
            onTool={setSketchTool}
            onActivate={setSketchId}
            onAlign={() => viewportRef.current?.alignToSketch()}
            run={run}
          />
          <div className="panel-title">Features</div>
          <FeaturePanel snapshot={snapshot} sketch={activeSketch} run={run} />
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
          {activeSketch
            ? `sketch: ${activeSketch.name} (${sketchTool})`
            : snapshot?.history.can_undo
              ? `undo: ${snapshot.history.undo_description ?? ""}`
              : ""}
        </span>
      </footer>
    </div>
  );
}
