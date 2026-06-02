import { emit, listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Rect } from "../types/screenshot";
import { clamp } from "./annotationGeometry";

export type OcrResultNormalizationSummary = {
  rawCount: number;
  usefulCount: number;
  virtualLineCount: number;
  droppedCount: number;
  routeMissingScripts?: string[];
};

type OcrResultWindowOptions = {
  selection: Rect;
  text: string;
  previewBase64: string;
  margin: number;
  gap: number;
  windowSize: { width: number; height: number };
  title?: string;
  normalizationSummary?: OcrResultNormalizationSummary;
};

const getOcrWindowPosition = async (
  selection: Rect,
  margin: number,
  gap: number,
  windowSize: { width: number; height: number },
) => {
  let screenX = 0;
  let screenY = 0;
  let factor = 1;

  try {
    const win = getCurrentWindow();
    const pos = await win.outerPosition();
    factor = await win.scaleFactor();
    screenX = pos.x / factor;
    screenY = pos.y / factor;
  } catch (error) {
    console.warn("Failed to get screenshot window position", error);
  }

  const minLeft = screenX + margin;
  const minTop = screenY + margin;
  const maxLeft = Math.max(minLeft, screenX + window.innerWidth - windowSize.width - margin);
  const maxTop = Math.max(minTop, screenY + window.innerHeight - windowSize.height - margin);
  const hasSpaceRight = selection.x + selection.w + gap + windowSize.width <= window.innerWidth - margin;
  const leftCandidate = screenX + (hasSpaceRight ? selection.x + selection.w + gap : selection.x);
  const topCandidate = screenY + selection.y;

  return {
    x: clamp(leftCandidate, minLeft, maxLeft),
    y: clamp(topCandidate, minTop, maxTop),
  };
};

export const openOcrResultWindow = async ({ selection, text, previewBase64, margin, gap, windowSize, title, normalizationSummary }: OcrResultWindowOptions) => {
  const label = `ocr_${Date.now()}`;
  const payload = JSON.stringify({ text, previewBase64, title, normalizationSummary });
  const position = await getOcrWindowPosition(selection, margin, gap, windowSize);
  let isPayloadDelivered = false;
  let resolvePayload: () => void = () => {};
  const payloadReady = new Promise<void>((resolve) => {
    resolvePayload = resolve;
  });

  const sendPayload = (finish = false) => {
    if (isPayloadDelivered) return;
    emit(`ocr-result-${label}`, payload).finally(() => {
      if (finish) {
        isPayloadDelivered = true;
        resolvePayload();
      }
    });
  };

  const unlistenReady = await listen(`ocr-ready-${label}`, () => sendPayload(true));
  try {
    const win = new WebviewWindow(label, {
      url: "index.html",
      title: title || "OCR Result",
      decorations: false,
      alwaysOnTop: true,
      focus: true,
      x: position.x,
      y: position.y,
      width: windowSize.width,
      height: windowSize.height,
      minWidth: 360,
      minHeight: 260,
      resizable: true,
      skipTaskbar: true,
      preventOverflow: true,
      shadow: true,
    });

    win.once("tauri://created", () => {
      win.setFocus().catch(() => {});
      setTimeout(sendPayload, 500);
    });
    win.once("tauri://destroyed", () => {
      if (!isPayloadDelivered) {
        isPayloadDelivered = true;
        resolvePayload();
      }
    });

    const retryTimer = window.setInterval(() => sendPayload(), 300);
    const timeoutTimer = window.setTimeout(() => {
      if (!isPayloadDelivered) {
        isPayloadDelivered = true;
        resolvePayload();
      }
    }, 5000);
    await payloadReady;
    window.clearInterval(retryTimer);
    window.clearTimeout(timeoutTimer);
  } finally {
    unlistenReady();
  }
};
