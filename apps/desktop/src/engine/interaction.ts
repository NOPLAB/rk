// Helpers for continuous edits (gizmo drags, sliders).
//
// The engine is a single mutex behind IPC, so a burst of pointer events must
// not queue up one round trip each: intermediate values are dropped in favor
// of the newest, which is what the user is looking at anyway.

import {
  applyInteractive,
  endInteraction,
  type ApplyOutcome,
  type Command,
  type EngineEvent,
} from "./api";

/**
 * UUID v4: interaction session IDs (one drag = one undo step) and
 * client-minted sketch entity IDs, which reference each other by ID.
 */
export function newUuid(): string {
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

/**
 * Apply a batch as a single undo step.
 *
 * Interaction sessions were built for gizmo drags, but the same mechanism
 * fits a command pair that is one action to the user — adding a sketch
 * constraint and re-solving. A failure rolls the whole session back.
 */
export async function applyAtomic(commands: Command[]): Promise<ApplyOutcome> {
  const session = newUuid();
  const events: EngineEvent[] = [];
  for (let i = 0; i < commands.length; i++) {
    const outcome = await applyInteractive(session, commands[i]);
    events.push(...outcome.events);
    if (outcome.error) {
      const undone = await endInteraction(session, true);
      return {
        applied: 0,
        events: [...events, ...undone.events],
        error: { index: i, message: outcome.error.message },
      };
    }
  }
  await endInteraction(session, false);
  return { applied: commands.length, events, error: null };
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
