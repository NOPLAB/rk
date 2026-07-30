// One line of text, for the Rename entries on the context menus.
//
// A small dialog rather than `window.prompt`, which the webview either blocks
// or renders in a style that has nothing to do with the rest of the app.

import { useEffect, useRef, useState } from "react";
import type { TextPrompt } from "../ui/appApi";

export function TextPromptDialog({
  prompt,
  onClose,
}: {
  prompt: TextPrompt;
  onClose: () => void;
}) {
  const [value, setValue] = useState(prompt.value);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setValue(prompt.value);
    input.current?.focus();
    input.current?.select();
  }, [prompt]);

  const accept = () => {
    const trimmed = value.trim();
    onClose();
    // An empty name would leave the row in the browser with nothing to click
    if (trimmed) prompt.onAccept(trimmed);
  };

  return (
    <>
      <div className="menu-backdrop" onPointerDown={onClose} />
      <div className="text-prompt">
        <div className="tp-title">{prompt.title}</div>
        <input
          ref={input}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            e.stopPropagation();
            if (e.key === "Enter") accept();
            else if (e.key === "Escape") onClose();
          }}
        />
        <div className="tp-buttons">
          <button onClick={onClose}>Cancel</button>
          <button className="primary" onClick={accept}>
            OK
          </button>
        </div>
      </div>
    </>
  );
}
