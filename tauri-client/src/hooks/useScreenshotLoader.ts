import { useState, useRef } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";
import type { Rect } from "../types/screenshot";
import type { Config } from "../types/config";
import { prewarmTranslationServices } from "../utils/localOcrTranslate";

interface UseScreenshotLoaderProps {
  screenshotModeRef: React.MutableRefObject<string>;
  configRef: React.MutableRefObject<Config>;
  setConfig: (config: Config) => void;
  loadWindowRects: (force?: boolean) => Promise<void>;
  clearWindowRects: () => void;
  clearScrollCaptureState: () => void;
  clearRecordingState: () => void;
  resetAnnotations: () => void;
  setCurrentRect: (next: Rect, syncState?: boolean) => void;
  setSelection: (selected: boolean) => void;
  setHasSelected: (selected: boolean) => void;
  setTranslatedResult: (res: string | null) => void;
  setTranslatePairs: (pairs: any[] | null) => void;
  setIsEditing: (editing: boolean) => void;
  setAnnotationTool: (tool: any) => void;
  setAnnotationColor: (color: string) => void;
  setAnnotationSizeState: (size: number) => void;
  setAnnotations: (annotations: any[]) => void;
  setRedoAnnotations: (annotations: any[]) => void;
  setSelectedAnnotationIndex: (index: number | null) => void;
  setEditingTextDraft: (draft: any) => void;
  setAnnotationDraft: (draft: any) => void;
  setScreenshotMode: (mode: string) => void;
  prewarmLocalOcrWorker: (reason: string) => void;
  draw: (rx: number, ry: number, rw: number, rh: number) => void;
  textSourceSnapshotPromiseRef: React.MutableRefObject<Promise<any> | null>;
  pendingConfirmTimerRef: React.MutableRefObject<number | null>;
}

const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };

export function useScreenshotLoader({
  screenshotModeRef,
  configRef,
  setConfig,
  loadWindowRects,
  clearWindowRects,
  clearScrollCaptureState,
  clearRecordingState,
  resetAnnotations,
  setCurrentRect,
  setSelection,
  setHasSelected,
  setTranslatedResult,
  setTranslatePairs,
  setIsEditing,
  setAnnotationTool,
  setAnnotationColor,
  setAnnotationSizeState,
  setAnnotations,
  setRedoAnnotations,
  setSelectedAnnotationIndex,
  setEditingTextDraft,
  setAnnotationDraft,
  setScreenshotMode,
  prewarmLocalOcrWorker,
  draw,
  textSourceSnapshotPromiseRef,
  pendingConfirmTimerRef,
}: UseScreenshotLoaderProps) {
  const [screenshotState, setScreenshotState] = useState<"initializing" | "ready" | "failed">("initializing");
  const [overlayVisible, setOverlayVisible] = useState(false);
  const [dbgStatus, setDbgStatus] = useState({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });

  const imageRef = useRef<HTMLImageElement | null>(null);
  const translatedImgRef = useRef<HTMLImageElement | null>(null);
  const maskedCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const analysisImageDataRef = useRef<ImageData | null>(null);
  const timeoutRef = useRef<any>(null);
  const captureIdRef = useRef<number>(0);
  const overlayVisibleRef = useRef(false);

  const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

  const reportOverlayFailure = (reason: string) => {
    setDbgStatus({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: reason });
    message.error({ content: `Screenshot window failed to start: ${reason}`, key: "screenshot-overlay", duration: 3 });
  };

  const clearPendingConfirm = () => {
    if (pendingConfirmTimerRef.current !== null) {
      window.clearTimeout(pendingConfirmTimerRef.current);
      pendingConfirmTimerRef.current = null;
    }
  };

  const startNewCaptureSession = (mode = "normal") => {
    clearPendingConfirm();
    captureIdRef.current += 1;
    const currentId = captureIdRef.current;

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    imageRef.current = null;
    translatedImgRef.current = null;
    analysisImageDataRef.current = null;
    textSourceSnapshotPromiseRef.current = null;
    setTranslatedResult(null);
    setTranslatePairs(null);
    setIsEditing(false);
    resetAnnotations();
    setAnnotationTool(null);
    setAnnotationColor("#ff0000");
    setAnnotationSizeState(6);
    setAnnotations([]);
    setRedoAnnotations([]);
    setSelectedAnnotationIndex(null);
    setEditingTextDraft(null);
    setAnnotationDraft(null);

    clearScrollCaptureState();
    clearRecordingState();
    clearWindowRects();

    setCurrentRect(EMPTY_RECT, true);
    setSelection(false);
    setScreenshotMode(mode);
    screenshotModeRef.current = mode;
    setScreenshotState("initializing");
    overlayVisibleRef.current = false;
    setOverlayVisible(false);
    setDbgStatus({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });

    return currentId;
  };

  const loadConfig = async () => {
    try {
      const raw = await invoke<string>("get_config");
      const parsedConfig = JSON.parse(raw);
      configRef.current = parsedConfig;
      setConfig(parsedConfig);
      prewarmLocalOcrWorker("screenshot-page-load");
      prewarmTranslationServices(parsedConfig, { reason: "screenshot-page-load" })
        .catch((error) => console.warn("[Translation Service Prewarm] failed", error));
    } catch (e) {
      console.error("[ScreenshotPage] loadConfig failed:", e);
      setConfig({});
    }
  };

  const cancelScreenshot = async (reason?: string) => {
    if (reason) reportOverlayFailure(reason);
    resetScreenshotState();
    await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
  };

  const initCanvas = (img: HTMLImageElement) => {
    const width = Math.max(1, window.innerWidth);
    const height = Math.max(1, window.innerHeight);

    const offscreen = document.createElement("canvas");
    offscreen.width = width;
    offscreen.height = height;
    const oCtx = offscreen.getContext("2d");
    if (oCtx) {
      oCtx.drawImage(img, 0, 0, width, height);
      try {
        analysisImageDataRef.current = oCtx.getImageData(0, 0, width, height);
      } catch {
        analysisImageDataRef.current = null;
      }
      oCtx.fillStyle = "rgba(0, 0, 0, 0.45)";
      oCtx.fillRect(0, 0, width, height);
    }
    maskedCanvasRef.current = offscreen;
    setCurrentRect(EMPTY_RECT, true);
    setSelection(false);
    draw(0, 0, 0, 0);
  };

  const loadImageFromSource = (source: string, sessionId: number, bytes?: number) => {
    if (sessionId !== captureIdRef.current) return;
    const img = new Image();
    img.crossOrigin = "anonymous";

    timeoutRef.current = setTimeout(() => {
      if (sessionId !== captureIdRef.current) return;
      if (imageRef.current === null) {
        cancelScreenshot("Screenshot overlay failed");
      }
    }, 1500);

    img.onload = async () => {
      if (sessionId !== captureIdRef.current) return;
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      try {
        await img.decode?.();
      } catch (e) {
        console.warn("[ScreenshotPage] img.decode failed", e);
      }
      if (sessionId !== captureIdRef.current) return;

      imageRef.current = img;
      setDbgStatus({ 
        imageLoaded: true, 
        imageWidth: img.naturalWidth, 
        imageHeight: img.naturalHeight, 
        screenshotBytes: bytes || 0,
        errorMsg: "" 
      });
      setScreenshotState("ready");
      await nextFrame();
      
      const canvas = document.querySelector("canvas");
      if (canvas) {
        const width = Math.max(1, window.innerWidth);
        const height = Math.max(1, window.innerHeight);
        canvas.width = width;
        canvas.height = height;
        canvas.style.width = `${width}px`;
        canvas.style.height = `${height}px`;
      }

      initCanvas(img);

      requestAnimationFrame(() => {
        void (async () => {
          if (sessionId !== captureIdRef.current) return;
          overlayVisibleRef.current = true;
          setOverlayVisible(true);
          await nextFrame();
          await nextFrame();
          if (sessionId !== captureIdRef.current) return;
          try {
            await invoke("overlay_ready_to_show", { label: getCurrentWindow().label });
          } catch (error: any) {
            throw new Error(error?.message || String(error));
          }
          const focusWindow = () => {
            const canvasEl = document.querySelector("canvas");
            if (canvasEl) canvasEl.focus({ preventScroll: true });
            getCurrentWindow().setFocus().catch(() => {});
          };
          focusWindow();
          window.setTimeout(focusWindow, 60);
        })().catch((error) => {
          if (sessionId !== captureIdRef.current) return;
          cancelScreenshot(error?.message || "Screenshot overlay failed");
        });
      });
    };

    img.onerror = () => {
      if (sessionId !== captureIdRef.current) return;
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      cancelScreenshot("Screenshot overlay failed");
    };
    img.src = source;
  };

  const loadImageFromBase64 = (base64: string, sessionId: number) => {
    if (sessionId !== captureIdRef.current) return;
    if (!base64 || base64.length < 1000) {
      cancelScreenshot("Screenshot overlay failed");
      return;
    }
    const dataUrl = "data:image/png;base64," + base64;
    loadImageFromSource(dataUrl, sessionId, Math.round(base64.length * 0.75));
  };

  const loadFullscreen = async (mode = screenshotModeRef.current || "normal") => {
    const sessionId = startNewCaptureSession(mode);
    try {
      loadWindowRects(true);
      const base64 = await invoke<string>("get_fullscreen_image");
      if (sessionId !== captureIdRef.current) return;
      if (!base64 || base64.length < 1000) {
        cancelScreenshot("Screenshot overlay failed");
        return;
      }
      loadImageFromBase64(base64, sessionId);
    } catch (err: any) {
      if (sessionId !== captureIdRef.current) return;
      cancelScreenshot(err?.message || "Screenshot overlay failed");
    }
  };

  const loadFullscreenFromBase64 = (base64: string, mode = "normal") => {
    const sessionId = startNewCaptureSession(mode);
    try {
      if (!base64 || base64.length < 1000) {
        cancelScreenshot("Screenshot overlay failed");
        return;
      }
      loadWindowRects(true);
      loadImageFromBase64(base64, sessionId);
    } catch (err: any) {
      if (sessionId !== captureIdRef.current) return;
      cancelScreenshot(err?.message || "Screenshot overlay failed");
    }
  };

  const loadFullscreenFromFile = (path: string, bytes?: number, mode = "normal") => {
    const sessionId = startNewCaptureSession(mode);
    try {
      if (!path) {
        loadFullscreen(mode);
        return;
      }
      loadWindowRects(true);
      loadImageFromSource(`${convertFileSrc(path)}?t=${Date.now()}`, sessionId, bytes);
    } catch (err: any) {
      if (sessionId !== captureIdRef.current) return;
      loadFullscreen(mode);
    }
  };

  function resetScreenshotState() {
    clearPendingConfirm();
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setCurrentRect(EMPTY_RECT, true);
    setHasSelected(false);
    setTranslatedResult(null);
    setTranslatePairs(null);
    setIsEditing(false);
    resetAnnotations();
    setAnnotationTool(null);
    setAnnotationColor("#ff0000");
    setAnnotationSizeState(6);
    setAnnotations([]);
    setRedoAnnotations([]);
    setSelectedAnnotationIndex(null);
    setEditingTextDraft(null);
    setAnnotationDraft(null);

    clearScrollCaptureState();
    clearRecordingState();
    clearWindowRects();

    invoke("set_window_capture_excluded", { label: getCurrentWindow().label, excluded: false }).catch(() => {});
    setScreenshotMode("normal");
    screenshotModeRef.current = "normal";
    setScreenshotState("initializing");
    setOverlayVisible(false);
    setDbgStatus({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });
    imageRef.current = null;
    translatedImgRef.current = null;
    analysisImageDataRef.current = null;
  }

  return {
    screenshotState,
    overlayVisible,
    dbgStatus,
    imageRef,
    translatedImgRef,
    maskedCanvasRef,
    analysisImageDataRef,
    overlayVisibleRef,
    timeoutRef,
    captureIdRef,
    setScreenshotState,
    setOverlayVisible,
    setDbgStatus,
    loadConfig,
    loadFullscreen,
    loadFullscreenFromBase64,
    loadFullscreenFromFile,
    loadImageFromBase64,
    loadImageFromSource,
    initCanvas,
    resetScreenshotState,
    cancelScreenshot,
  };
}
