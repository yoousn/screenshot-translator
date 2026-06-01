import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import ScreenshotPage from "./pages/ScreenshotPage";
import PinPage from "./pages/PinPage";
import OcrPage from "./pages/OcrPage";
import RecordingControlPage from "./pages/RecordingControlPage";
import "./index.css";

const label = getCurrentWindow().label;

// Set transparent background BEFORE React renders for screenshot windows
if (label === "screenshot" || label === "recording_control") {
  document.body.style.backgroundColor = "transparent";
  document.documentElement.style.backgroundColor = "transparent";
  document.body.classList.add("transparent-window");
  document.documentElement.classList.add("transparent-window");
}

let Component: React.ComponentType;
if (label === "screenshot") {
  Component = ScreenshotPage;
} else if (label.startsWith("pin_")) {
  Component = PinPage;
} else if (label.startsWith("ocr_")) {
  Component = OcrPage;
} else if (label === "recording_control") {
  Component = RecordingControlPage;
} else {
  Component = App;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Component />
  </React.StrictMode>,
);
