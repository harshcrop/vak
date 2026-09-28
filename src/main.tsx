import React from "react";
import ReactDOM from "react-dom/client";
import "./styles.css";
import { MainWindow } from "./windows/MainWindow";
import { Hud } from "./windows/Hud";
import { Panel } from "./windows/Panel";

if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window && (window as any).__TAURI_INTERNALS__?.invoke)) {
  (await import("./lib/mock")).installMock();
}

const which = new URLSearchParams(location.search).get("w");
const Root = which === "hud" ? Hud : which === "panel" ? Panel : MainWindow;

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
