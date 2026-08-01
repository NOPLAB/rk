// Dragging a panel tab.
//
// Pointer events rather than HTML5 drag-and-drop, for one reason: with the
// pointer captured the move events keep coming after the pointer leaves the
// window, and `screenX`/`screenY` say where on the desktop it ended up. That
// is what makes "drag the tab onto the second monitor" possible at all.
//
// Nothing here goes through React. The ghost is a plain DOM node and the
// drop-target highlight is a class toggled on the dock — a pointermove at
// 120 Hz must not re-render the browser tree.

/** Where a dragged tab was let go */
export type PanelDrop =
  | { kind: "dock"; dock: string; index: number }
  /** Outside the window: the panel becomes its own OS window there */
  | { kind: "outside"; screenX: number; screenY: number }
  | null;

const DROP_CLASS = "drop-into";
/** Below this the gesture was a click on the tab, not a drag */
const THRESHOLD = 5;

function clearHighlights() {
  document
    .querySelectorAll(`.${DROP_CLASS}`)
    .forEach((el) => el.classList.remove(DROP_CLASS));
}

function outside(e: PointerEvent): boolean {
  return (
    e.clientX < 0 ||
    e.clientY < 0 ||
    e.clientX > window.innerWidth ||
    e.clientY > window.innerHeight
  );
}

/** The dock under the pointer, and where among its tabs the panel would land */
function dockUnder(e: PointerEvent): { dock: string; index: number } | null {
  const el = document
    .elementsFromPoint(e.clientX, e.clientY)
    .find((n) => n instanceof HTMLElement && n.closest("[data-dock]")) as
    | HTMLElement
    | undefined;
  const dockEl = el?.closest("[data-dock]") as HTMLElement | undefined;
  const dock = dockEl?.dataset.dock;
  if (!dock) return null;

  // Dropping on the strip inserts between tabs; anywhere else appends
  const strip = dockEl?.querySelector("[data-tabstrip]");
  const tabs = strip ? Array.from(strip.querySelectorAll("[data-panel]")) : [];
  let index = tabs.length;
  for (let i = 0; i < tabs.length; i++) {
    const rect = tabs[i].getBoundingClientRect();
    if (e.clientX < rect.left + rect.width / 2) {
      index = i;
      break;
    }
  }
  return { dock, index };
}

/**
 * Begin dragging `label`'s tab. `onDrop` receives where it landed, or `null`
 * when the gesture was a plain click or was cancelled with Escape.
 */
export function startPanelDrag(
  event: React.PointerEvent<HTMLElement>,
  label: string,
  onDrop: (drop: PanelDrop) => void,
) {
  if (event.button !== 0) return;
  const target = event.currentTarget;
  const pointerId = event.pointerId;
  const startX = event.clientX;
  const startY = event.clientY;

  let dragging = false;
  let ghost: HTMLDivElement | null = null;
  let last: PanelDrop = null;
  let cancelled = false;

  const begin = () => {
    dragging = true;
    ghost = document.createElement("div");
    ghost.className = "panel-ghost";
    ghost.textContent = label;
    document.body.appendChild(ghost);
  };

  const finish = () => {
    target.removeEventListener("pointermove", onMove);
    target.removeEventListener("pointerup", onUp);
    target.removeEventListener("pointercancel", onCancel);
    window.removeEventListener("keydown", onKey);
    try {
      target.releasePointerCapture(pointerId);
    } catch {
      // The capture is already gone when the pointer was cancelled
    }
    ghost?.remove();
    clearHighlights();
  };

  const onMove = (e: PointerEvent) => {
    if (e.pointerId !== pointerId) return;
    if (!dragging) {
      if (Math.hypot(e.clientX - startX, e.clientY - startY) < THRESHOLD) return;
      begin();
    }
    if (ghost) {
      ghost.style.transform = `translate(${e.clientX + 12}px, ${e.clientY + 12}px)`;
    }
    clearHighlights();
    if (outside(e)) {
      last = { kind: "outside", screenX: e.screenX, screenY: e.screenY };
      ghost?.classList.add("tearing");
      return;
    }
    ghost?.classList.remove("tearing");
    const hit = dockUnder(e);
    last = hit ? { kind: "dock", ...hit } : null;
    if (hit) {
      document
        .querySelector(`[data-dock="${hit.dock}"]`)
        ?.classList.add(DROP_CLASS);
    }
  };

  const onUp = (e: PointerEvent) => {
    if (e.pointerId !== pointerId) return;
    finish();
    onDrop(cancelled || !dragging ? null : last);
  };

  const onCancel = (e: PointerEvent) => {
    if (e.pointerId !== pointerId) return;
    finish();
    onDrop(null);
  };

  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return;
    cancelled = true;
    finish();
    onDrop(null);
  };

  target.setPointerCapture(pointerId);
  target.addEventListener("pointermove", onMove);
  target.addEventListener("pointerup", onUp);
  target.addEventListener("pointercancel", onCancel);
  window.addEventListener("keydown", onKey);
}
