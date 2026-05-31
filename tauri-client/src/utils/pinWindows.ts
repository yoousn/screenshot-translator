import { emit, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Rect } from "../types/screenshot";

type CroppedSelection = { x: number; y: number; w: number; h: number };

const emitPinImageWhenReady = async (label: string, imgData: string, delayMs: number) => {
  let sent = false;
  const unlistenReady = await listen(`pin-ready-${label}`, () => {
    sent = true;
    emit(`pin-image-${label}`, imgData).catch(() => {});
  });
  return {
    sent: () => sent,
    emitFallback: () => {
      if (!sent) {
        sent = true;
        emit(`pin-image-${label}`, imgData).catch(() => {});
      }
    },
    unlistenReady,
    delayMs,
  };
};

export const openPinWindow = async (imgData: string, cropped: CroppedSelection) => {
  const label = `pin_${Date.now()}`;
  let finalX = cropped.x;
  let finalY = cropped.y;
  let factor = 1;

  try {
    const win = getCurrentWindow();
    const pos = await win.outerPosition();
    factor = await win.scaleFactor();
    finalX += pos.x;
    finalY += pos.y;
  } catch (error) {
    console.warn("Failed to get window position", error);
  }

  const ready = await emitPinImageWhenReady(label, imgData, 1000);
  try {
    const win = new WebviewWindow(label, {
      url: "index.html",
      title: "Pin",
      transparent: true,
      decorations: false,
      alwaysOnTop: true,
      x: finalX / factor,
      y: finalY / factor,
      width: cropped.w / factor,
      height: cropped.h / factor,
      skipTaskbar: true,
    });

    win.once("tauri://created", () => {
      setTimeout(ready.emitFallback, ready.delayMs);
    });
    win.once("tauri://destroyed", () => ready.unlistenReady());
  } catch (error) {
    ready.unlistenReady();
    throw error;
  }
};

export const openPreviewWindow = async (imgData: string, rect: Rect) => {
  const label = `pin_preview_${Date.now()}`;
  const maxW = 720;
  const maxH = 520;
  const scale = Math.min(1, maxW / rect.w, maxH / rect.h);
  const ready = await emitPinImageWhenReady(label, imgData, 500);

  const win = new WebviewWindow(label, {
    url: "index.html",
    title: "截图预览",
    transparent: false,
    decorations: true,
    alwaysOnTop: true,
    width: Math.max(240, Math.round(rect.w * scale)),
    height: Math.max(180, Math.round(rect.h * scale)),
    resizable: true,
    skipTaskbar: false,
  });

  win.once("tauri://created", () => {
    setTimeout(ready.emitFallback, ready.delayMs);
  });
  win.once("tauri://destroyed", () => ready.unlistenReady());
};
