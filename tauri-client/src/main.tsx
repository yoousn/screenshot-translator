import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import ScreenshotPage from "./pages/ScreenshotPage";
import "./index.css";

const label = getCurrentWindow().label;

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {label === "screenshot" ? <ScreenshotPage /> : <App />}
  </React.StrictMode>,
);
