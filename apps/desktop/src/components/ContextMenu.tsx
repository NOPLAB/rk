// The one right-click menu.
//
// Every surface that has a context menu — the 3D view, the browser tree, the
// panel tabs — builds a list of entries (see `ui/menus.ts`) and hands it to
// this component. It owns nothing but placement and dismissal.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { Icon, type IconName } from "./icons";

export type MenuEntry =
  | { kind: "sep" }
  | { kind: "header"; label: string }
  | {
      kind?: "item";
      icon?: IconName;
      label: string;
      hint?: string;
      disabled?: boolean;
      /** Draws a tick, for entries that toggle something */
      checked?: boolean;
      /** Destructive, shown in red */
      danger?: boolean;
      onClick: () => void;
    };

export interface MenuRequest {
  x: number;
  y: number;
  entries: MenuEntry[];
}

/** A separator at either end, or two in a row, reads as a mistake */
function tidy(entries: MenuEntry[]): MenuEntry[] {
  const out: MenuEntry[] = [];
  for (const entry of entries) {
    const isSep = "kind" in entry && entry.kind === "sep";
    if (isSep && (out.length === 0 || out[out.length - 1].kind === "sep")) {
      continue;
    }
    out.push(entry);
  }
  while (out.length > 0 && out[out.length - 1].kind === "sep") out.pop();
  return out;
}

export function ContextMenu({
  menu,
  onClose,
}: {
  menu: MenuRequest;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: menu.x, top: menu.y });
  const entries = tidy(menu.entries);

  // Open away from the edge it would otherwise run off
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos({
      left: Math.max(4, Math.min(menu.x, window.innerWidth - width - 4)),
      top: Math.max(4, Math.min(menu.y, window.innerHeight - height - 4)),
    });
  }, [menu]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  if (entries.length === 0) return null;

  return (
    <>
      <div
        className="menu-backdrop"
        onPointerDown={onClose}
        onContextMenu={(e) => {
          e.preventDefault();
          onClose();
        }}
      />
      <div className="context-menu" ref={ref} style={pos}>
        {entries.map((entry, i) => {
          if (entry.kind === "sep") return <div className="sep" key={i} />;
          if (entry.kind === "header") {
            return (
              <div className="menu-header" key={i}>
                {entry.label}
              </div>
            );
          }
          return (
            <button
              key={i}
              className={entry.danger ? "danger" : ""}
              title={entry.hint ?? entry.label}
              disabled={entry.disabled}
              onClick={() => {
                onClose();
                entry.onClick();
              }}
            >
              <span className="tick">{entry.checked ? "✓" : ""}</span>
              {entry.icon ? (
                <Icon name={entry.icon} size={14} />
              ) : (
                <span className="no-icon" />
              )}
              <span className="label">{entry.label}</span>
            </button>
          );
        })}
      </div>
    </>
  );
}
