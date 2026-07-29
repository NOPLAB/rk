// Quick-access bar: the handful of commands that stay reachable whatever the
// ribbon is showing, plus the document title.

import { redo, undo } from "../engine/commands";
import type { AppApi } from "../ui/appApi";
import { fileActions } from "../ui/fileActions";
import { Icon, type IconName } from "./icons";

export function TitleBar({ api }: { api: AppApi }) {
  const files = fileActions(api);
  const history = api.snapshot?.history;
  const name = api.snapshot?.project_name ?? "…";
  const dirty = api.snapshot?.modified ? " *" : "";

  return (
    <header className="titlebar">
      <span className="brand">RK</span>
      <div className="qa">
        <QaBtn icon="new" title="New document" onClick={() => void files.onNew()} />
        <QaBtn icon="open" title="Open… (Ctrl+O)" onClick={() => void files.onOpen()} />
        <QaBtn icon="save" title="Save (Ctrl+S)" onClick={() => void files.onSave()} />
        <span className="qa-sep" />
        <QaBtn
          icon="undo"
          title={
            history?.undo_description
              ? `Undo ${history.undo_description} (Ctrl+Z)`
              : "Undo (Ctrl+Z)"
          }
          disabled={!history?.can_undo}
          onClick={() => void api.run([undo()])}
        />
        <QaBtn
          icon="redo"
          title="Redo (Ctrl+Y)"
          disabled={!history?.can_redo}
          onClick={() => void api.run([redo()])}
        />
      </div>
      <div className="doc-title">
        {name}
        {dirty} — RK
      </div>
      <div className="qa">
        <QaBtn
          icon="home"
          title="Home view"
          onClick={() => api.viewport()?.homeView()}
        />
        <QaBtn
          icon="fit"
          title="Zoom to fit"
          onClick={() => api.viewport()?.fitCamera()}
        />
      </div>
    </header>
  );
}

function QaBtn({
  icon,
  title,
  disabled,
  onClick,
}: {
  icon: IconName;
  title: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className="qa-btn"
      title={title}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon name={icon} size={17} />
    </button>
  );
}
