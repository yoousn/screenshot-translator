
import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import ScreenshotPage from "./pages/ScreenshotPage";
import PinPage from "./pages/PinPage";
import OcrPage from "./pages/OcrPage";
import RecordingControlPage from "./pages/RecordingControlPage";
import RecordingNoticePage from "./pages/RecordingNoticePage";
import SaveToastPage from "./pages/SaveToastPage";
import { I18nProvider } from "./i18n";
import "./index.css";

const resolveWindowLabel = () => {
  const search = window.location.search;
  if (search.includes("recordingSessionKey") || search.includes("recording_control")) {
    return "recording_control";
  }
  if (search.includes("recording_notice")) {
    return "recording_notice";
  }
  if (search.includes("save_toast")) {
    return "save_toast";
  }
  try {
    return getCurrentWindow().label;
  } catch {
    return "main";
  }
};

const label = resolveWindowLabel();

// Guard: if main window is loaded with recording parameters (routing error),
// render nothing and force hide to prevent white-screen control panel on main.
if (label === "main") {
  const search = window.location.search;
  if (search.includes("recordingSessionKey") || search.includes("recording_control")) {
    invoke("hide_main_window").catch(() => {});
    // Throw a bare string to stop execution — this is a fatal misroute.
    // The window should remain invisible and the error is caught by the browser.
    throw new Error("FATAL: recording route loaded in main window, aborting render");
  }
}

// Set transparent background BEFORE React renders for screenshot windows
if (label === "screenshot" || label.startsWith("recording_control") || label === "recording_notice" || label === "save_toast") {
  document.body.style.backgroundColor = "transparent";
  document.documentElement.style.backgroundColor = "transparent";
  document.body.classList.add("transparent-window");
  document.documentElement.classList.add("transparent-window");
}

let Component: React.ComponentType;
let needsI18nProvider = false;
if (label === "screenshot") {
  Component = ScreenshotPage;
  needsI18nProvider = true;
} else if (label.startsWith("pin_")) {
  Component = PinPage;
} else if (label.startsWith("ocr_")) {
  Component = OcrPage;
  needsI18nProvider = true;
} else if (label.startsWith("recording_control")) {
  Component = RecordingControlPage;
} else if (label === "recording_notice") {
  Component = RecordingNoticePage;
} else if (label === "save_toast") {
  Component = SaveToastPage;
} else {
  Component = App;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {needsI18nProvider ? (
      <I18nProvider>
        <Component />
      </I18nProvider>
    ) : (
      <Component />
    )}
  </React.StrictMode>,
);
