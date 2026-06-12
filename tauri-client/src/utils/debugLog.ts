import { invoke } from "@tauri-apps/api/core";

export const isDebugLoggingEnabled =
  import.meta.env.DEV || import.meta.env.VITE_YSN_DEBUG_LOGS === "1";

export const traceLog = (...args: unknown[]) => {
  if (isDebugLoggingEnabled) {
    console.log(...args);
  }
};

export const logScreenshotPerf = (message: string) => {
  if (isDebugLoggingEnabled) {
    invoke("log_screenshot_perf", { message }).catch(() => {});
  }
};
