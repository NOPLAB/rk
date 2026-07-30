import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

// A window that fails to start must say so. Without this the webview simply
// stays blank white, which looks identical whether the app crashed, the dev
// server is down, or the page never loaded at all.
function showFailure(what: string, error: unknown) {
  const root = document.getElementById("root");
  if (!root) return;
  const detail = error instanceof Error ? `${error.message}\n\n${error.stack}` : String(error);
  root.innerHTML = "";
  const box = document.createElement("pre");
  box.className = "startup-failure";
  box.textContent = `${what}\n\n${detail}`;
  root.appendChild(box);
}

window.addEventListener("error", (e) => showFailure("RK failed to start", e.error ?? e.message));
window.addEventListener("unhandledrejection", (e) =>
  showFailure("RK hit an unhandled rejection", e.reason),
);

try {
  createRoot(document.getElementById("root")!).render(<App />);
} catch (e) {
  showFailure("RK failed to start", e);
}
