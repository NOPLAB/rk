// Helpers for continuous edits (gizmo drags, sliders).
//
// The engine is a single mutex behind IPC, so a burst of pointer events must
// not queue up one round trip each: intermediate values are dropped in favor
// of the newest, which is what the user is looking at anyway.

/** Session ID for `apply_interactive` — one drag is one undo step */
export function newSessionId(): string {
  if (typeof crypto.randomUUID === "function") return crypto.randomUUID();
  const hex = [...crypto.getRandomValues(new Uint8Array(16))].map((b, i) => {
    const v = i === 6 ? (b & 0x0f) | 0x40 : i === 8 ? (b & 0x3f) | 0x80 : b;
    return v.toString(16).padStart(2, "0");
  });
  return [
    hex.slice(0, 4).join(""),
    hex.slice(4, 6).join(""),
    hex.slice(6, 8).join(""),
    hex.slice(8, 10).join(""),
    hex.slice(10).join(""),
  ].join("-");
}

export interface Coalescer {
  /** Run `task` when the queue drains; a newer push replaces a waiting one */
  push(task: () => Promise<void>): void;
  /** Run `task` after everything already queued — never dropped */
  finish(task: () => Promise<void>): Promise<void>;
}

export function createCoalescer(): Coalescer {
  let pending: (() => Promise<void>) | null = null;
  let draining: Promise<void> | null = null;

  const drain = async () => {
    while (pending) {
      const task = pending;
      pending = null;
      try {
        await task();
      } catch (e) {
        console.error("coalesced task failed:", e);
      }
    }
    draining = null;
  };

  return {
    push(task) {
      pending = task;
      if (!draining) draining = drain();
    },
    async finish(task) {
      if (draining) await draining;
      await task();
    },
  };
}
