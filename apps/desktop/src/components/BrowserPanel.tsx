// Model browser — RK's answer to Inventor's browser tree.
//
// One tree over the whole document: origin planes, parts, the assembly's
// link/joint chain, sketches, and the feature history ending in the "End of
// Part" marker that shows where the rollback bar sits. Every row carries the
// same right-click menu the object has anywhere else in the app.

import { useState, type ReactNode } from "react";
import type {
  FeatureGroupInfo,
  FeatureInfo,
  JointInfo,
  LinkInfo,
  PartInfo,
} from "../engine/api";
import {
  deleteFeature,
  deletePart,
  deleteSketch,
  rollbackTo,
  setFeatureGroupCollapsed,
  setFeatureSuppressed,
} from "../engine/commands";
import type { AppApi } from "../ui/appApi";
import {
  featureGroupMenu,
  featureMenu,
  jointMenu,
  originPlaneMenu,
  partMenu,
  sketchMenu,
} from "../ui/menus";
import { createSketchOnOrigin } from "../ui/sketchActions";
import { Icon, type IconName } from "./icons";

const PLANES = ["XY", "XZ", "YZ"] as const;
/** Guard against a malformed joint graph turning the tree into a loop */
const MAX_DEPTH = 12;

export function BrowserPanel({ api }: { api: AppApi }) {
  // Origin starts collapsed, everything else open — the way a fresh
  // Inventor part document looks
  const [collapsed, setCollapsed] = useState<Set<string>>(
    () => new Set(["origin"]),
  );
  // Ctrl-click builds up a set of features, which is what Group acts on
  const [picked, setPicked] = useState<string[]>([]);
  const snapshot = api.snapshot;

  const isOpen = (key: string) => !collapsed.has(key);
  const toggle = (key: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  if (!snapshot) {
    return (
      <aside className="browser">
        <div className="empty">Loading…</div>
      </aside>
    );
  }

  const {
    parts,
    links,
    joints,
    sketches,
    features,
    feature_groups,
    rollback_position,
  } = snapshot;

  const partName = (partId: string | null, linkId: string) =>
    (partId && parts.find((p) => p.id === partId)?.name) ||
    links.find((l) => l.id === linkId)?.name ||
    "link";

  // The assembly is a link tree: a joint hangs under its parent link and
  // carries the child link with it
  const renderLink = (link: LinkInfo, depth: number): ReactNode[] => {
    if (depth > MAX_DEPTH) return [];
    const key = `link:${link.id}`;
    const outgoing = joints.filter((j) => j.parent_link === link.id);
    const part = link.part_id
      ? (parts.find((p) => p.id === link.part_id) ?? null)
      : null;
    const rows: ReactNode[] = [
      <TreeRow
        key={key}
        depth={depth}
        icon="link"
        label={partName(link.part_id, link.id)}
        tag={link.collisions.length > 0 ? `${link.collisions.length}c` : ""}
        hasChildren={outgoing.length > 0}
        open={isOpen(key)}
        selected={!!link.part_id && link.part_id === api.selected}
        onToggle={() => toggle(key)}
        onClick={() => link.part_id && api.select(link.part_id)}
        onMenu={part ? (x, y) => api.openMenu(x, y, partMenu(api, part)) : undefined}
      />,
    ];
    if (!isOpen(key)) return rows;
    for (const joint of outgoing) {
      rows.push(...renderJoint(joint, depth + 1));
    }
    return rows;
  };

  const renderJoint = (joint: JointInfo, depth: number): ReactNode[] => {
    const child = links.find((l) => l.id === joint.child_link);
    const rows: ReactNode[] = [
      <TreeRow
        key={`joint:${joint.id}`}
        depth={depth}
        icon="joint"
        label={joint.name}
        tag={joint.joint_type}
        onClick={() => joint.child_part && api.select(joint.child_part)}
        onMenu={(x, y) => api.openMenu(x, y, jointMenu(api, joint))}
      />,
    ];
    if (child) rows.push(...renderLink(child, depth + 1));
    return rows;
  };

  const rootLinks = links.filter(
    (l) => !joints.some((j) => j.child_link === l.id),
  );
  // Parts that never joined the assembly still belong in the tree
  const looseParts = parts.filter(
    (p) => !links.some((l) => l.part_id === p.id),
  );

  // ---- feature history, with groups folded in --------------------------

  const orderOf = new Map(features.map((f, i) => [f.id, i]));
  const rolledBack = (id: string) =>
    rollback_position !== null && (orderOf.get(id) ?? 0) >= rollback_position;
  /** The group each feature belongs to, keyed by the member drawn first */
  const groupAt = new Map<string, FeatureGroupInfo>();
  const grouped = new Set<string>();
  for (const group of feature_groups) {
    const first = features.find((f) => group.members.includes(f.id));
    if (first) groupAt.set(first.id, group);
    group.members.forEach((id) => grouped.add(id));
  }

  const clickFeature = (feature: FeatureInfo, additive: boolean) =>
    setPicked((prev) => {
      if (!additive) return prev.length === 1 && prev[0] === feature.id ? [] : [feature.id];
      return prev.includes(feature.id)
        ? prev.filter((id) => id !== feature.id)
        : [...prev, feature.id];
    });

  const featureRow = (feature: FeatureInfo, depth: number) => (
    <TreeRow
      key={feature.id}
      depth={depth}
      icon="feature"
      label={feature.name}
      tag={feature.kind}
      selected={picked.includes(feature.id)}
      struck={feature.suppressed || rolledBack(feature.id)}
      hint="Click to pick — Ctrl adds, then right-click to group"
      onClick={(e) => clickFeature(feature, e.ctrlKey || e.metaKey || e.shiftKey)}
      onMenu={(x, y) => api.openMenu(x, y, featureMenu(api, feature, picked))}
      actions={
        <>
          <RowButton
            icon="suppress"
            title={feature.suppressed ? "Unsuppress" : "Suppress"}
            onClick={() =>
              void api.run([
                setFeatureSuppressed(feature.id, !feature.suppressed),
              ])
            }
          />
          <RowButton
            icon="rollback"
            title="Roll back to just after this feature"
            onClick={() => void api.run([rollbackTo(feature.id)])}
          />
          <RowButton
            icon="close"
            title="Delete feature"
            onClick={() => void api.run([deleteFeature(feature.id)])}
          />
        </>
      }
    />
  );

  const featureRows: ReactNode[] = [];
  for (const feature of features) {
    const group = groupAt.get(feature.id);
    if (group) {
      featureRows.push(
        <TreeRow
          key={`group:${group.id}`}
          depth={2}
          icon="folder"
          label={group.name}
          tag={`${group.members.length}`}
          hasChildren
          open={!group.collapsed}
          hint="A bundle of timeline features — the model is unaffected"
          onToggle={() =>
            void api.run([setFeatureGroupCollapsed(group.id, !group.collapsed)])
          }
          onMenu={(x, y) => api.openMenu(x, y, featureGroupMenu(api, group))}
        />,
      );
      if (!group.collapsed) {
        for (const id of group.members) {
          const member = features.find((f) => f.id === id);
          if (member) featureRows.push(featureRow(member, 3));
        }
      }
      continue;
    }
    if (grouped.has(feature.id)) continue; // drawn under its group already
    featureRows.push(featureRow(feature, 2));
  }

  return (
    <aside className="browser">
      <div className="tree">
        <TreeRow
          depth={0}
          icon="new"
          label={snapshot.project_name || "Untitled"}
          tag={snapshot.modified ? "*" : ""}
          hasChildren
          open
          onClick={() => api.select(null)}
        />

        <TreeRow
          depth={1}
          icon="folder"
          label="Origin"
          hasChildren
          open={isOpen("origin")}
          onToggle={() => toggle("origin")}
        />
        {isOpen("origin") &&
          PLANES.map((plane) => (
            <TreeRow
              key={plane}
              depth={2}
              icon="plane"
              label={`${plane} Plane`}
              hint="Start a sketch on this plane"
              onClick={() => void createSketchOnOrigin(api, plane)}
              onMenu={(x, y) => api.openMenu(x, y, originPlaneMenu(api, plane))}
              // Hovering the row lights the matching quad in the 3D view
              onHover={(over) =>
                api.viewport()?.setPlaneHighlight(over ? plane : null)
              }
            />
          ))}

        <TreeRow
          depth={1}
          icon="folder"
          label="Parts"
          tag={parts.length ? String(parts.length) : ""}
          hasChildren={parts.length > 0}
          open={isOpen("parts")}
          onToggle={() => toggle("parts")}
        />
        {isOpen("parts") && parts.length === 0 && (
          <div className="empty" style={{ paddingLeft: 30 }}>
            No parts
          </div>
        )}
        {isOpen("parts") &&
          parts.map((part) => (
            <PartRow key={part.id} api={api} part={part} depth={2} />
          ))}

        <TreeRow
          depth={1}
          icon="folder"
          label="Assembly"
          tag={joints.length ? `${joints.length}j` : ""}
          hasChildren={links.length > 0}
          open={isOpen("assembly")}
          onToggle={() => toggle("assembly")}
        />
        {isOpen("assembly") && links.length === 0 && (
          <div className="empty" style={{ paddingLeft: 30 }}>
            Nothing connected
          </div>
        )}
        {isOpen("assembly") &&
          rootLinks.flatMap((link) => renderLink(link, 2))}
        {isOpen("assembly") &&
          links.length > 0 &&
          looseParts.map((part) => (
            <TreeRow
              key={`loose:${part.id}`}
              depth={2}
              icon="part"
              label={part.name}
              tag="free"
              dim
              selected={part.id === api.selected}
              onClick={() => api.select(part.id)}
              onMenu={(x, y) => api.openMenu(x, y, partMenu(api, part))}
            />
          ))}

        <TreeRow
          depth={1}
          icon="folder"
          label="Sketches"
          tag={sketches.length ? String(sketches.length) : ""}
          hasChildren={sketches.length > 0}
          open={isOpen("sketches")}
          onToggle={() => toggle("sketches")}
        />
        {isOpen("sketches") && sketches.length === 0 && (
          <div className="empty" style={{ paddingLeft: 30 }}>
            No sketches
          </div>
        )}
        {isOpen("sketches") &&
          sketches.map((sketch) => {
            const editing = sketch.id === api.activeSketch?.id;
            return (
              <TreeRow
                key={sketch.id}
                depth={2}
                icon="sketch"
                label={sketch.name}
                tag={`${sketch.entity_count}e ${sketch.profile_count}p`}
                hint="Click to edit this sketch"
                selected={editing}
                onClick={() => api.activateSketch(editing ? null : sketch.id)}
                onMenu={(x, y) => api.openMenu(x, y, sketchMenu(api, sketch))}
                actions={
                  <RowButton
                    icon="close"
                    title="Delete sketch"
                    onClick={() => {
                      if (editing) api.activateSketch(null);
                      void api.run([deleteSketch(sketch.id)]);
                    }}
                  />
                }
              />
            );
          })}

        <TreeRow
          depth={1}
          icon="folder"
          label="Features"
          tag={features.length ? String(features.length) : ""}
          hasChildren={features.length > 0}
          open={isOpen("features")}
          onToggle={() => toggle("features")}
        />
        {isOpen("features") && features.length === 0 && (
          <div className="empty" style={{ paddingLeft: 30 }}>
            No features
          </div>
        )}
        {isOpen("features") && featureRows}
        {isOpen("features") && (
          <TreeRow
            depth={2}
            icon="endOfPart"
            label="End of Part"
            dim
            hint={
              rollback_position === null
                ? "Every feature is built"
                : "Click to rebuild the rolled-back features"
            }
            onClick={() =>
              rollback_position !== null && void api.run([rollbackTo(null)])
            }
          />
        )}
      </div>
    </aside>
  );
}

function PartRow({
  api,
  part,
  depth,
}: {
  api: AppApi;
  part: PartInfo;
  depth: number;
}) {
  const rgb = part.color
    .slice(0, 3)
    .map((c) => Math.round(c * 255))
    .join(",");
  return (
    <TreeRow
      depth={depth}
      icon="part"
      label={part.name}
      selected={part.id === api.selected}
      swatch={`rgb(${rgb})`}
      onClick={() => api.select(part.id === api.selected ? null : part.id)}
      onMenu={(x, y) => api.openMenu(x, y, partMenu(api, part))}
      actions={
        <RowButton
          icon="close"
          title="Delete part"
          onClick={() => {
            if (part.id === api.selected) api.select(null);
            void api.run([deletePart(part.id)]);
          }}
        />
      }
    />
  );
}

interface RowProps {
  depth: number;
  icon: IconName;
  label: string;
  tag?: string;
  hint?: string;
  hasChildren?: boolean;
  open?: boolean;
  selected?: boolean;
  dim?: boolean;
  struck?: boolean;
  swatch?: string;
  onToggle?: () => void;
  onClick?: (e: React.MouseEvent) => void;
  /** Right-click, at the pointer */
  onMenu?: (x: number, y: number) => void;
  /** Pointer entered (true) or left (false) the row */
  onHover?: (over: boolean) => void;
  actions?: ReactNode;
}

function TreeRow({
  depth,
  icon,
  label,
  tag,
  hint,
  hasChildren,
  open,
  selected,
  dim,
  struck,
  swatch,
  onToggle,
  onClick,
  onMenu,
  onHover,
  actions,
}: RowProps) {
  const classes = [
    "tree-row",
    selected ? "selected" : "",
    dim ? "dim" : "",
    struck ? "struck" : "",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <div
      className={classes}
      style={{ paddingLeft: 4 + depth * 12 }}
      title={hint ?? label}
      onClick={onClick}
      onContextMenu={
        onMenu
          ? (e) => {
              e.preventDefault();
              e.stopPropagation();
              onMenu(e.clientX, e.clientY);
            }
          : undefined
      }
      onMouseEnter={onHover ? () => onHover(true) : undefined}
      onMouseLeave={onHover ? () => onHover(false) : undefined}
    >
      <button
        className={`twist${open ? " open" : ""}${hasChildren ? "" : " leaf"}`}
        tabIndex={-1}
        onClick={(e) => {
          e.stopPropagation();
          onToggle?.();
        }}
      >
        <Icon name="chevron" size={11} />
      </button>
      {swatch && <span className="swatch" style={{ background: swatch }} />}
      <Icon name={icon} size={14} />
      <span className="label">{label}</span>
      {tag && <span className="tag">{tag}</span>}
      {actions}
    </div>
  );
}

function RowButton({
  icon,
  title,
  onClick,
}: {
  icon: IconName;
  title: string;
  onClick: () => void;
}) {
  return (
    <button
      className="row-btn"
      title={title}
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
    >
      <Icon name={icon} size={12} />
    </button>
  );
}
