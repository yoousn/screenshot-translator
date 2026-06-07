import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type RecordingOptionsPayload = {
  fps: number;
  resolution: string;
  audio_mode: string;
  mic_device: string | null;
  system_audio_device: string | null;
  output_dir: string | null;
  region_x: number;
  region_y: number;
  region_w: number;
  region_h: number;
};

export type RecordingWindowPayload = {
  options: RecordingOptionsPayload;
  countdownSeconds: number;
  autoStart?: boolean;
  borderLabels: string[];
  noticeRect?: RecordingBorderRect;
  restoreMainWindow?: boolean;
};

export type RecordingBorderRect = { x: number; y: number; w: number; h: number };

const RECORDING_CONTROL_PREFIX = "recording_control";
const RECORDING_NOTICE_LABEL = "recording_notice";
const RECORDING_SESSION_STORAGE_PREFIX = "ysn-recording-session:";

const sleep = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

const getExistingWindows = async () => WebviewWindow.getAll().catch(() => []);

export const getWindowByLabelIfExists = async (label: string) => {
  const windows = await getExistingWindows();
  return windows.find((win) => win.label === label) || null;
};

const waitForWindowGone = async (label: string, timeoutMs = 1200) => {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() <= deadline) {
    const existing = await getWindowByLabelIfExists(label);
    if (!existing) return true;
    await sleep(40);
  }
  return !(await getWindowByLabelIfExists(label));
};

export const closeWindowIfExists = async (label: string, timeoutMs = 1200) => {
  const win = await getWindowByLabelIfExists(label);
  if (!win) return;
  await win.hide().catch(() => {});
  await win.close().catch(() => {});
  const closed = await waitForWindowGone(label, timeoutMs);
  if (!closed) throw new Error(`${label} did not close in time`);
};

const setWindowCaptureExcludedIfExists = async (label: string, excluded: boolean) => {
  await invoke("set_window_capture_excluded", { label, excluded }).catch(() => {});
};

const getWindowsByPrefix = async (prefix: string) => {
  const windows = await WebviewWindow.getAll().catch(() => []);
  return windows.filter((win) => win.label === prefix || win.label.startsWith(`${prefix}_`));
};

const closeWindowsByPrefix = async (prefix: string, timeoutMs = 800) => {
  const windows = await getWindowsByPrefix(prefix);
  await Promise.all(windows.map(async (win) => {
    await win.hide().catch(() => {});
    return withTimeout(win.close().catch(() => {}), timeoutMs).catch(() => null);
  }));
};

const setWindowCaptureExcludedByPrefix = async (prefix: string, excluded: boolean) => {
  const windows = await getWindowsByPrefix(prefix);
  await Promise.all(windows.map((win) => setWindowCaptureExcludedIfExists(win.label, excluded)));
};

const withTimeout = async <T,>(task: Promise<T>, ms: number): Promise<T | null> => {
  let timeoutId: number | undefined;
  try {
    return await Promise.race([
      task,
      new Promise<null>((resolve) => {
        timeoutId = window.setTimeout(() => resolve(null), ms);
      }),
    ]);
  } finally {
    if (timeoutId !== undefined) window.clearTimeout(timeoutId);
  }
};

const waitForWindowCreated = async (win: WebviewWindow, label: string, timeoutMs = 1600) => new Promise<void>((resolve, reject) => {
  let settled = false;
  const unlisteners: Array<() => void> = [];
  const settle = (callback: () => void) => {
    if (settled) return;
    settled = true;
    window.clearTimeout(timeoutId);
    unlisteners.forEach((unlisten) => unlisten());
    callback();
  };
  const timeoutId = window.setTimeout(() => {
    settle(() => reject(new Error(`${label} was not created in time`)));
  }, timeoutMs);

  win.once("tauri://created", () => {
    settle(resolve);
  }).then((unlisten) => {
    if (settled) unlisten();
    else unlisteners.push(unlisten);
  }).catch((error) => {
    settle(() => reject(error));
  });

  win.once<{ error?: string }>("tauri://error", (event) => {
    settle(() => reject(new Error(event.payload?.error || `${label} failed to create`)));
  }).then((unlisten) => {
    if (settled) unlisten();
    else unlisteners.push(unlisten);
  }).catch((error) => {
    settle(() => reject(error));
  });
});

const writeRecordingSessionPayload = (key: string, payload: RecordingWindowPayload) => {
  try {
    window.localStorage.setItem(key, JSON.stringify(payload));
  } catch {
  }
};

const removeRecordingSessionPayload = (key: string) => {
  try {
    window.localStorage.removeItem(key);
  } catch {
  }
};

type CloseRecordingBorderWindowsOptions = {
  source?: string;
  hideMain?: boolean;
};

export const closeRecordingBorderWindows = async (
  _labels: string[] = [],
  options: CloseRecordingBorderWindowsOptions = {}
) => {
  const source = options.source ?? "closeRecordingBorderWindows";
  const hideMain = options.hideMain ?? true;
  console.log("[window-trace] action=closeRecordingBorderWindows start");
  await Promise.all([
    setWindowCaptureExcludedIfExists("main", false),
    setWindowCaptureExcludedIfExists("screenshot", false),
    setWindowCaptureExcludedByPrefix(RECORDING_CONTROL_PREFIX, false),
    setWindowCaptureExcludedIfExists(RECORDING_NOTICE_LABEL, false),
    (async () => {
      console.log(`[window-trace] invoke force_close_recording_controls source=${source} hideMain=${hideMain}`);
      await withTimeout(invoke("force_close_recording_controls", { source, hideMain }).catch(() => {}), 700);
    })(),
    withTimeout(invoke("hide_recording_overlay").catch(() => {}), 500),
    withTimeout(closeWindowIfExists("recording_overlay", 500).catch(() => {}), 500),
  ]);
  await closeWindowIfExists(RECORDING_NOTICE_LABEL, 1000).catch(() => {});
  await closeWindowsByPrefix(RECORDING_CONTROL_PREFIX, 800);
};

export const openRecordingWindows = async (payload: Omit<RecordingWindowPayload, "borderLabels">, selection: RecordingBorderRect) => {
  console.log("[window-trace] action=openRecordingWindows start");
  await closeRecordingBorderWindows([]);
  await Promise.all([
    setWindowCaptureExcludedIfExists("main", true),
    setWindowCaptureExcludedIfExists("screenshot", true),
  ]);
  const factor = await getCurrentWindow().scaleFactor().catch(() => window.devicePixelRatio || 1);
  const overlayRect = {
    x: Math.round(selection.x / factor),
    y: Math.round(selection.y / factor),
    w: Math.max(160, Math.round(selection.w / factor)),
    h: Math.max(100, Math.round(selection.h / factor)),
  };
  const controlSize = { w: 620, h: 96 };
  const screenInfo = window.screen as Screen & { availLeft?: number; availTop?: number };
  const screenLeft = screenInfo.availLeft || 0;
  const screenTop = screenInfo.availTop || 0;
  const screenRight = screenLeft + screenInfo.availWidth;
  const screenBottom = screenTop + screenInfo.availHeight;
  const centeredX = overlayRect.x + Math.round((overlayRect.w - controlSize.w) / 2);
  const controlX = Math.min(Math.max(screenLeft + 8, centeredX), Math.max(screenLeft + 8, screenRight - controlSize.w - 8));
  const belowY = overlayRect.y + overlayRect.h + 10;
  const aboveY = overlayRect.y - controlSize.h - 10;
  const preferredY = belowY + controlSize.h <= screenBottom - 8 ? belowY : aboveY;
  const controlY = Math.min(Math.max(screenTop + 8, preferredY), Math.max(screenTop + 8, screenBottom - controlSize.h - 8));

  const fullPayload: RecordingWindowPayload = { ...payload, borderLabels: ["recording_overlay"], noticeRect: overlayRect, restoreMainWindow: false };
  const controlLabel = `${RECORDING_CONTROL_PREFIX}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  const sessionStorageKey = `${RECORDING_SESSION_STORAGE_PREFIX}${controlLabel}`;
  console.log(`[window-trace] controlLabel=${controlLabel} sessionStorageKey=${sessionStorageKey}`);
  writeRecordingSessionPayload(sessionStorageKey, fullPayload);
  let sent = false;
  const unlistenReady = await listen("recording-overlay-ready", () => {
    console.log("[window-trace] recording-overlay-ready received");
    sent = true;
    console.log("[window-trace] emitting recording-overlay-session");
    emit("recording-overlay-session", fullPayload).catch(() => {});
  });

  const control = new WebviewWindow(controlLabel, {
    url: `index.html?recordingSessionKey=${encodeURIComponent(sessionStorageKey)}`,
    title: "Recording Control",
    decorations: false,
    transparent: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    resizable: false,
    minimizable: false,
    maximizable: false,
    shadow: false,
    width: controlSize.w,
    height: controlSize.h,
    x: controlX,
    y: controlY,
  });

  try {
    await waitForWindowCreated(control, controlLabel);
    await invoke("dump_all_windows_state", { source: "openRecordingWindows-after-create" }).catch(() => {});
    await invoke("set_window_capture_excluded", { label: controlLabel, excluded: true }).catch(() => {});
    await invoke("show_recording_overlay", {
      x: Math.round(selection.x),
      y: Math.round(selection.y),
      w: Math.round(selection.w),
      h: Math.round(selection.h),
    });
    window.setTimeout(() => {
      if (!sent) {
        console.log("[window-trace] emitting recording-overlay-session (fallback)");
        emit("recording-overlay-session", fullPayload).catch(() => {});
      }
    }, 600);
    control.once("tauri://destroyed", () => {
      unlistenReady();
      removeRecordingSessionPayload(sessionStorageKey);
    }).catch(() => {});
    console.log("[window-trace] action=openRecordingWindows successful, returning (does not call screenshot.hide itself)");
  } catch (error) {
    unlistenReady();
    removeRecordingSessionPayload(sessionStorageKey);
    await closeRecordingBorderWindows([]).catch(() => {});
    throw error;
  }
};
