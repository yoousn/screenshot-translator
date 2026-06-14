
import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { App as AntdApp, ConfigProvider } from "antd";
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

// Keep the screenshot helper visually transparent before React renders. The
// native window is shown only after a real canvas frame is ready, so an empty
// WebView backing surface should not flash black or white.
if (label === "screenshot") {
  document.body.style.background = "transparent";
  document.documentElement.style.background = "transparent";
  document.getElementById("root")?.style.setProperty("background", "transparent", "important");
  document.body.classList.add("transparent-window");
  document.documentElement.classList.add("transparent-window");
} else if (label.startsWith("recording_control") || label === "recording_notice" || label === "save_toast") {
  document.body.style.background = "transparent";
  document.documentElement.style.background = "transparent";
  document.getElementById("root")?.style.setProperty("background", "transparent", "important");
  document.body.classList.add("transparent-window");
  document.documentElement.classList.add("transparent-window");
}

let Component: React.ComponentType;
let needsI18nProvider = false;
let needsAntdProvider = false;
if (label === "screenshot") {
  Component = ScreenshotPage;
  needsI18nProvider = true;
  needsAntdProvider = true;
} else if (label.startsWith("pin_")) {
  Component = PinPage;
} else if (label.startsWith("ocr_")) {
  Component = OcrPage;
  needsI18nProvider = true;
  needsAntdProvider = true;
} else if (label.startsWith("recording_control")) {
  Component = RecordingControlPage;
} else if (label === "recording_notice") {
  Component = RecordingNoticePage;
} else if (label === "save_toast") {
  Component = SaveToastPage;
} else {
  Component = App;
}

const renderComponent = () => {
  const content = needsI18nProvider ? (
    <I18nProvider>
      <Component />
    </I18nProvider>
  ) : (
    <Component />
  );

  if (!needsAntdProvider) return content;

  return (
    <ConfigProvider theme={{ token: { borderRadius: 12, colorPrimary: "#1677ff" } }}>
      <AntdApp>
        {content}
      </AntdApp>
    </ConfigProvider>
  );
};

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {renderComponent()}
  </React.StrictMode>,
);
