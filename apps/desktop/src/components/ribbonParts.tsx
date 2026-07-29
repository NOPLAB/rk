// Ribbon building blocks: a group (bordered column with a caption under it),
// large buttons for the headline command, and stacked small buttons for the
// rest — the layout Inventor uses for every panel on its ribbon.

import type { ReactNode } from "react";
import { Icon, type IconName } from "./icons";

export function RibGroup({
  name,
  children,
  pinned,
}: {
  name: string;
  children: ReactNode;
  /** Stays against the right edge when the ribbon is too wide to fit —
   *  a tab's way out must never be the thing that scrolls off */
  pinned?: boolean;
}) {
  return (
    <div className={pinned ? "rb-group pinned" : "rb-group"}>
      <div className="rb-group-body">{children}</div>
      <div className="rb-group-name">{name}</div>
    </div>
  );
}

/** Vertical stack of small buttons, up to three per column */
export function RibCol({ children }: { children: ReactNode }) {
  return <div className="rb-col">{children}</div>;
}

interface ButtonProps {
  icon: IconName;
  label: string;
  hint?: string;
  disabled?: boolean;
  active?: boolean;
  onClick: () => void;
}

export function RibBig({
  icon,
  label,
  hint,
  disabled,
  active,
  onClick,
}: ButtonProps) {
  return (
    <button
      className={`rb-big${active ? " active" : ""}`}
      title={hint ?? label}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon name={icon} size={26} />
      <span>{label}</span>
    </button>
  );
}

export function RibSmall({
  icon,
  label,
  hint,
  disabled,
  active,
  onClick,
}: ButtonProps) {
  return (
    <button
      className={`rb-small${active ? " active" : ""}`}
      title={hint ?? label}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon name={icon} size={16} />
      <span>{label}</span>
    </button>
  );
}

/** Explanatory text inside a group, for whatever blocks a command */
export function RibHint({ children }: { children: ReactNode }) {
  return <div className="rb-hint">{children}</div>;
}

/** Split a list into columns of `size`, so groups fill top-to-bottom */
export function chunk<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) {
    out.push(items.slice(i, i + size));
  }
  return out;
}
