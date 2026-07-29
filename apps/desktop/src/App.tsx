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
  type SketchGeometry,
} from "./engine/api";
import {
  deleteSketchEntities,
  redo,
  setPartTransform,
  undo,
  type StlUnit,
} from "./engine/commands";
import { applyAtomic, createCoalescer, newUuid } from "./engine/interaction";
import { Viewport, type GizmoMode } from "./scene/viewport";
import { SketchDrawing, type SketchTool } from "./scene/sketchTools";
import { BrowserPanel } from "./components/BrowserPanel";
import { DimensionEntry } from "./components/DimensionEntry";
import { FeatureDialog } from "./components/FeatureDialog";
import { Inspector } from "./components/Inspector";
import { NavBar } from "./components/NavBar";
import { Ribbon } from "./components/Ribbon";
import { DocTabs, StatusBar } from "./components/StatusBar";
import { TitleBar } from "./components/TitleBar";
import type { AppApi, DialogKind, PendingDimension } from "./ui/appApi";
import { fileActions } from "./ui/fileActions";

/** Click behaviour of the sketch select tool: shift accumulates, plain replaces */
function nextSelection(
  current: string[],
  entityId: string | null,
  additive: boolean,
): string[] {
  if (!entityId) return additive ? current : [];
  if (!additive) return [entityId];
  return current.includes(entityId)
    ? current.filter((id) => id !== entityId)
    : [...current, entityId];
}

/** Sketch tool shortcuts, following Inventor's single-letter keys */
const SKETCH_KEYS: Record<string, SketchTool> = {
  s: "select",
  l: "line",
  r: "rect",
  c: "circle",
};

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
  const [sketchSelection, setSketchSelection] = useState<string[]>([]);
  const [sketchGeom, setSketchGeom] = useState<SketchGeometry | null>(null);
  const [pendingDimension, setPendingDimension] =
    useState<PendingDimension | null>(null);
  const [dialog, setDialog] = useState<DialogKind | null>(null);
  const [showCollisions, setShowCollisions] = useState(false);
  const [showGrid, setShowGrid] = useState(true);
  const [showBrowser, setShowBrowser] = useState(true);
  const [showInspector, setShowInspector] = useState(true);
  const [meshUnit, setMeshUnit] = useState<StlUnit>("Millimeters");
  const [status, setStatus] = useState("");

  const refresh = useCallback(async () => {
    const snap = await sceneSnapshot();
    snapshotRef.current = snap;
    setSnapshot(snap);
    viewportRef.current?.setTransforms(snap.transforms);
    // Collision transforms already fold in the link poses, so they are
    // rebuilt from the snapshot rather than tracked through events
    viewportRef.current?.setCollisions(snap.links);
    setSelected((sel) =>
      sel && !snap.parts.some((p) => p.id === sel) ? null : sel,
    );
    return snap;
  }, []);

  /** Apply a command batch, sync the 3D scene from the events, refresh UI state */
  const run = useCallback(
    async (commands: Command[], atomic = false): Promise<EngineEvent[]> => {
      try {
        const outcome = atomic
          ? await applyAtomic(commands)
          : await applyCommands(commands);
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
    vp.onSketchClick = (hit, additive) => {
      const drawing = drawingRef.current;
      if (drawing.activeTool === "select") {
        // Constraints act on several entities, so picking accumulates
        setSketchSelection((prev) => nextSelection(prev, hit.entityId, additive));
        return;
      }
      const commands = drawing.click(hit);
      vp.setSketchPreview(drawing.preview(hit));
      if (commands.length > 0) void run(commands);
    };
    vp.onSketchMove = (hit) => {
      vp.setSketchPreview(drawingRef.current.preview(hit));
    };
    // The panels classify and measure the same geometry the viewport draws
    vp.onSketchGeometry = setSketchGeom;

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
      vp.setCollisions(snap.links);
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

  useEffect(() => {
    viewportRef.current?.setCollisionsVisible(showCollisions);
  }, [showCollisions]);

  useEffect(() => {
    viewportRef.current?.setGridVisible(showGrid);
  }, [showGrid]);

  useEffect(() => {
    viewportRef.current?.setSketchSelection(sketchSelection);
  }, [sketchSelection]);

  const activeSketch = useMemo(
    () => snapshot?.sketches.find((s) => s.id === sketchId) ?? null,
    [snapshot, sketchId],
  );

  // Entering/leaving sketch mode. Deleting or undoing the sketch away drops
  // `activeSketch` to null, which takes the viewport out of sketch mode too.
  useEffect(() => {
    void viewportRef.current?.setSketch(activeSketch);
    drawingRef.current.setSketch(activeSketch?.id ?? null);
    setSketchSelection([]);
    setPendingDimension(null);
    if (!activeSketch) {
      setSketchGeom(null);
      setSketchTool("select");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeSketch?.id]);

  useEffect(() => {
    drawingRef.current.setTool(sketchTool);
    viewportRef.current?.setSketchPreview(null);
  }, [sketchTool]);

  const selectedPart =
    (selected && snapshot?.parts.find((p) => p.id === selected)) || null;

  const api: AppApi = {
    snapshot,
    selected,
    selectedPart,
    select,
    gizmoMode,
    setGizmoMode,
    activeSketch,
    activateSketch: setSketchId,
    sketchTool,
    setSketchTool,
    sketchSelection,
    setSketchSelection,
    sketchGeometry: sketchGeom,
    hoverSketch: (ids) => viewportRef.current?.setSketchHover(ids),
    pendingDimension,
    setPendingDimension,
    dialog,
    setDialog,
    showCollisions,
    setShowCollisions,
    showGrid,
    setShowGrid,
    showBrowser,
    setShowBrowser,
    showInspector,
    setShowInspector,
    meshUnit,
    setMeshUnit,
    viewport: () => viewportRef.current,
    run,
    setStatus,
  };

  useEffect(() => {
    const files = fileActions(api);

    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement | null)?.tagName ?? "";
      if (tag === "INPUT" || tag === "SELECT" || tag === "TEXTAREA") return;
      const vp = viewportRef.current;

      if (e.ctrlKey || e.metaKey) {
        const key = e.key.toLowerCase();
        if (key === "s") {
          e.preventDefault();
          void files.onSave();
        } else if (key === "o") {
          e.preventDefault();
          void files.onOpen();
        } else if (key === "z" && !e.shiftKey) {
          e.preventDefault();
          if (snapshot?.history.can_undo) void run([undo()]);
        } else if (key === "y" || (key === "z" && e.shiftKey)) {
          e.preventDefault();
          if (snapshot?.history.can_redo) void run([redo()]);
        }
        return;
      }

      if (e.key === "Escape") {
        // Unwind whatever is most recent: a value entry, a dialog, the shape
        // being drawn, the selection, then a gizmo drag
        if (pendingDimension) setPendingDimension(null);
        else if (dialog) setDialog(null);
        else if (!sketchId) vp?.cancelDrag();
        else if (drawingRef.current.busy) {
          drawingRef.current.cancel();
          vp?.setSketchPreview(null);
        } else if (sketchSelection.length > 0) setSketchSelection([]);
        else setSketchTool("select");
        return;
      }
      if (e.altKey) return;

      if (sketchId) {
        if (
          (e.key === "Delete" || e.key === "Backspace") &&
          sketchSelection.length > 0
        ) {
          setSketchSelection([]);
          void run([deleteSketchEntities(sketchId, sketchSelection)]);
          return;
        }
        const tool = SKETCH_KEYS[e.key.toLowerCase()];
        if (tool) setSketchTool(tool);
        return; // the gizmo shortcuts below are meaningless while sketching
      }

      const key = e.key.toLowerCase();
      if (key === "q") setGizmoMode("none");
      else if (key === "w") setGizmoMode("translate");
      else if (key === "e") setGizmoMode("rotate");
    };

    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <div className="app">
      <TitleBar api={api} />
      <Ribbon api={api} />
      <div className="workspace">
        {showBrowser && <BrowserPanel api={api} />}
        <div className="viewport" ref={containerRef}>
          <canvas ref={canvasRef} />
          <NavBar api={api} />
          {dialog && <FeatureDialog api={api} kind={dialog} />}
          {pendingDimension && <DimensionEntry api={api} />}
        </div>
        {showInspector && <Inspector api={api} />}
      </div>
      <DocTabs api={api} />
      <StatusBar api={api} status={status} />
    </div>
  );
}
