// Ribbon building blocks: a group (bordered column with a caption under it),
// large buttons for the headline command, and stacked small buttons for the
// rest — the layout Inventor uses for every panel on its ribbon.

import { useRef, useState, type ReactNode } from "react";
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

export interface SplitItem extends ButtonProps {
  key: string;
}

/**
 * A button that stands for a family of related commands: pressing it runs the
 * one on its face, and the caret drops down the rest. Given a single item it
 * degrades to a plain button, so a family can grow without the caller caring.
 *
 * The list is `position: fixed` because the ribbon clips its own overflow —
 * anchored inside the scrolling row it would be cut off at the group's edge.
 */
export function RibSplit({
  big,
  face,
  items,
}: {
  big?: boolean;
  /** The variant the button itself runs */
  face: ButtonProps;
  items: SplitItem[];
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  const Button = big ? RibBig : RibSmall;
  if (items.length < 2) return <Button {...face} />;

  const rect = ref.current?.getBoundingClientRect();
  return (
    <div className={big ? "rb-split big" : "rb-split"} ref={ref}>
      <Button {...face} />
      <button
        className="rb-caret"
        title="More"
        aria-label="More"
        onClick={() => setOpen((v) => !v)}
      >
        <Icon name="chevron" size={10} />
      </button>
      {open && rect && (
        <>
          <div className="menu-backdrop" onClick={() => setOpen(false)} />
          <div
            className="rb-flyout"
            style={{ left: rect.left, top: rect.bottom + 2 }}
          >
            {items.map(({ key, icon, label, hint, active, disabled, onClick }) => (
              <button
                key={key}
                className={active ? "active" : ""}
                title={hint ?? label}
                disabled={disabled}
                onClick={() => {
                  setOpen(false);
                  onClick();
                }}
              >
                <Icon name={icon} size={16} />
                <span>{label}</span>
              </button>
            ))}
          </div>
        </>
      )}
    </div>
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
