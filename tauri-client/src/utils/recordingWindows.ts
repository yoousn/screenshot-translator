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
  borderLabels: string[];
};

export type RecordingBorderRect = { x: number; y: number; w: number; h: number };

const closeWindowIfExists = async (label: string) => {
  const win = await WebviewWindow.getByLabel(label).catch(() => null);
  if (!win) return;
  await win.destroy().catch(() => win.close().catch(() => {}));
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
    withTimeout(invoke("hide_recording_overlay").catch(() => {}), 250),
    withTimeout(closeWindowIfExists("recording_overlay"), 250),
    withTimeout(closeWindowIfExists("recording_control"), 250),
  ]);
};

export const openRecordingWindows = async (payload: Omit<RecordingWindowPayload, "borderLabels">, selection: RecordingBorderRect) => {
  await closeRecordingBorderWindows([]);
  const factor = await getCurrentWindow().scaleFactor().catch(() => window.devicePixelRatio || 1);
  const overlayRect = {
    x: Math.round(selection.x / factor),
    y: Math.round(selection.y / factor),
    w: Math.max(160, Math.round(selection.w / factor)),
    h: Math.max(100, Math.round(selection.h / factor)),
  };
  const controlSize = { w: 450, h: 58 };
  const screenInfo = window.screen as Screen & { availLeft?: number; availTop?: number };
  const screenLeft = screenInfo.availLeft || 0;
  const screenTop = screenInfo.availTop || 0;
  const screenRight = screenLeft + screenInfo.availWidth;
  const screenBottom = screenTop + screenInfo.availHeight;
  const controlX = Math.min(Math.max(screenLeft + 4, overlayRect.x + 8), Math.max(screenLeft + 4, screenRight - controlSize.w - 4));
  const controlY = Math.min(Math.max(screenTop + 4, overlayRect.y + 8), Math.max(screenTop + 4, screenBottom - controlSize.h - 4));

  const fullPayload: RecordingWindowPayload = { ...payload, borderLabels: ["recording_overlay"] };
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
    title: "录制控制",
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
    control.setFocus().catch(() => {});
    window.setTimeout(() => {
      if (!sent) emit("recording-overlay-session", fullPayload).catch(() => {});
    }, 600);
  });
  control.once("tauri://destroyed", () => unlistenReady());
};
