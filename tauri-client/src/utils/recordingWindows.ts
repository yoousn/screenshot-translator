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

const closeWindowIfExists = async (label: string) => {
  const win = await WebviewWindow.getByLabel(label).catch(() => null);
  if (!win) return;
  await win.destroy().catch(() => win.close().catch(() => {}));
};

const setWindowCaptureExcludedIfExists = async (label: string, excluded: boolean) => {
  await invoke("set_window_capture_excluded", { label, excluded }).catch(() => {});
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

export const closeRecordingBorderWindows = async (_labels: string[] = []) => {
  await Promise.all([
    setWindowCaptureExcludedIfExists("main", false),
    setWindowCaptureExcludedIfExists("screenshot", false),
    setWindowCaptureExcludedIfExists("recording_control", false),
    setWindowCaptureExcludedIfExists("recording_notice", false),
    withTimeout(invoke("hide_recording_overlay").catch(() => {}), 250),
    withTimeout(closeWindowIfExists("recording_overlay"), 250),
    withTimeout(closeWindowIfExists("recording_control"), 250),
    withTimeout(closeWindowIfExists("recording_notice"), 250),
  ]);
};

export const openRecordingWindows = async (payload: Omit<RecordingWindowPayload, "borderLabels">, selection: RecordingBorderRect) => {
  await closeRecordingBorderWindows([]);
  const mainWindow = await WebviewWindow.getByLabel("main").catch(() => null);
  const restoreMainWindow = Boolean(await mainWindow?.isVisible().catch(() => false));
  if (restoreMainWindow) {
    await mainWindow?.hide().catch(() => {});
  }
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
  const controlSize = { w: 560, h: 58 };
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

  const fullPayload: RecordingWindowPayload = { ...payload, borderLabels: ["recording_overlay"], noticeRect: overlayRect, restoreMainWindow };
  let sent = false;
  const unlistenReady = await listen("recording-overlay-ready", () => {
    sent = true;
    emit("recording-overlay-session", fullPayload).catch(() => {});
  });

  invoke("show_recording_overlay", {
    x: Math.round(selection.x),
    y: Math.round(selection.y),
    w: Math.round(selection.w),
    h: Math.round(selection.h),
  }).catch((error) => console.warn("Failed to show recording overlay", error));

  const control = new WebviewWindow("recording_control", {
    url: "index.html",
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

  control.once("tauri://created", () => {
    invoke("set_window_capture_excluded", { label: "recording_control", excluded: true }).catch(() => {});
    window.setTimeout(() => {
      if (!sent) emit("recording-overlay-session", fullPayload).catch(() => {});
    }, 600);
  });
  control.once("tauri://destroyed", () => unlistenReady());
};
