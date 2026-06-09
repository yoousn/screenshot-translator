import { useState, useRef } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";
import type { NativeScreenshotDiagnosticsStatus, Rect, ScreenshotPhysicalBounds } from "../types/screenshot";
import type { Config } from "../types/config";
import { prewarmTranslationServices } from "../utils/localOcrTranslate";

const logScreenshotPerf = (messageText: string) => {
  invoke("log_screenshot_perf", { message: messageText }).catch(() => {});
};

const logScreenshotBaseline = (sessionId: string | number, phase: string, elapsedMs: number, detail = "") => {
  invoke("log_screenshot_perf", {
    message: `[baseline] session=${sessionId} phase=${phase} elapsed_ms=${Math.round(elapsedMs)} ${detail}`,
  }).catch(() => {});
};

const logNativeScreenshotDiagnostics = (sessionId: string | number, phase: string, elapsedMs: number) => {
  invoke<NativeScreenshotDiagnosticsStatus>("get_native_screenshot_diagnostics_status")
    .then((status) => {
      const gpuStatus = status?.gpuPlan?.primaryStatus || status?.d3d11?.capability?.status || "unknown";
      const gpuFallback = status?.gpuPlan?.primaryFallback || status?.d3d11?.capability?.fallback || "unknown";
      const wgcSupported = status?.wgc?.nativeApi?.isSupported;
      const dxgiReason = status?.dxgi?.nativeApi?.reason || "unknown";
      logScreenshotBaseline(
        sessionId,
        phase,
        elapsedMs,
        `native_gpu=${gpuStatus} native_fallback=${gpuFallback} wgc_supported=${wgcSupported === true} dxgi_reason=${dxgiReason}`
      );
    })
    .catch(() => {
      logScreenshotBaseline(sessionId, phase, elapsedMs, "native_status=unavailable");
    });
};

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
type ScreenshotImageSource = HTMLImageElement | HTMLCanvasElement;

const getScreenshotImageWidth = (image: ScreenshotImageSource) => image instanceof HTMLImageElement ? image.naturalWidth : image.width;
const getScreenshotImageHeight = (image: ScreenshotImageSource) => image instanceof HTMLImageElement ? image.naturalHeight : image.height;
const waitForAnimationFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

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

  const imageRef = useRef<ScreenshotImageSource | null>(null);
  const translatedImgRef = useRef<HTMLImageElement | null>(null);
  const maskedCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const analysisImageDataRef = useRef<ImageData | null>(null);
  const timeoutRef = useRef<any>(null);
  const captureIdRef = useRef<number>(0);
  const displayedSessionIdRef = useRef<string | null>(null);
  const displayedPhysicalBoundsRef = useRef<ScreenshotPhysicalBounds | null>(null);
  const overlayVisibleRef = useRef(false);
  const nativeOverlayVisibleRef = useRef(false);
  const frontendSessionStartedAtRef = useRef<number>(0);

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

  const startNewCaptureSession = (mode = "normal", remoteSessionId?: string | number, preserveVisibleShell = false, physicalBounds?: ScreenshotPhysicalBounds | null) => {
    clearPendingConfirm();
    captureIdRef.current += 1;
    const currentId = captureIdRef.current;
    displayedSessionIdRef.current = remoteSessionId == null ? null : String(remoteSessionId);
    displayedPhysicalBoundsRef.current = physicalBounds ?? null;
    frontendSessionStartedAtRef.current = performance.now();
    logScreenshotBaseline(remoteSessionId || currentId, "frontend_session_start", 0, `mode=${mode}`);

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
    if (!preserveVisibleShell) {
      clearWindowRects();
      nativeOverlayVisibleRef.current = false;
    }

    setCurrentRect(EMPTY_RECT, true);
    setSelection(false);
    setScreenshotMode(mode);
    screenshotModeRef.current = mode;
    setScreenshotState("initializing");
    overlayVisibleRef.current = preserveVisibleShell;
    setOverlayVisible(preserveVisibleShell);
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

  const captureAnalysisImageData = (img: ScreenshotImageSource, sessionId: number, remoteSessionId?: string | number) => {
    window.setTimeout(() => {
      if (sessionId !== captureIdRef.current) return;
      const width = Math.max(1, window.innerWidth);
      const height = Math.max(1, window.innerHeight);
      const analysisCanvas = document.createElement("canvas");
      analysisCanvas.width = width;
      analysisCanvas.height = height;
      const ctx = analysisCanvas.getContext("2d", { willReadFrequently: true });
      if (!ctx) return;
      ctx.drawImage(img, 0, 0, width, height);
      try {
        analysisImageDataRef.current = ctx.getImageData(0, 0, width, height);
        logScreenshotBaseline(remoteSessionId || sessionId, "analysis_image_data_ready", performance.now() - frontendSessionStartedAtRef.current, `width=${width} height=${height}`);
      } catch {
        analysisImageDataRef.current = null;
      }
    }, 0);
  };

  const initCanvas = (img: ScreenshotImageSource, sessionId?: number, remoteSessionId?: string | number) => {
    const width = Math.max(1, window.innerWidth);
    const height = Math.max(1, window.innerHeight);

    const offscreen = document.createElement("canvas");
    offscreen.width = width;
    offscreen.height = height;
    const oCtx = offscreen.getContext("2d");
    if (oCtx) {
      oCtx.drawImage(img, 0, 0, width, height);
      oCtx.fillStyle = "rgba(0, 0, 0, 0.45)";
      oCtx.fillRect(0, 0, width, height);
    }
    maskedCanvasRef.current = offscreen;
    if (sessionId) logScreenshotBaseline(remoteSessionId || sessionId, "mask_canvas_ready", performance.now() - frontendSessionStartedAtRef.current, `width=${width} height=${height}`);
    setCurrentRect(EMPTY_RECT, true);
    setSelection(false);
    draw(0, 0, 0, 0);
  };
  const completeImageLoad = (img: ScreenshotImageSource, sessionId: number, bytes: number | undefined, remoteSessionId?: string | number) => {
    if (sessionId !== captureIdRef.current) return;
    const imageWidth = getScreenshotImageWidth(img);
    const imageHeight = getScreenshotImageHeight(img);
    imageRef.current = img;
    setDbgStatus({
      imageLoaded: true,
      imageWidth,
      imageHeight,
      screenshotBytes: bytes || 0,
      errorMsg: ""
    });
    const wasNativeOverlayVisible = nativeOverlayVisibleRef.current;
    logScreenshotPerf(`frontend image ready bytes=${bytes || 0}`);

    const canvas = document.querySelector("canvas");
    if (canvas) {
      const width = Math.max(1, window.innerWidth);
      const height = Math.max(1, window.innerHeight);
      canvas.width = width;
      canvas.height = height;
      canvas.style.width = `${width}px`;
      canvas.style.height = `${height}px`;
    }

    initCanvas(img, sessionId, remoteSessionId);
    overlayVisibleRef.current = true;
    setScreenshotState("ready");
    setOverlayVisible(true);

    requestAnimationFrame(() => {
      if (sessionId !== captureIdRef.current || imageRef.current === null || maskedCanvasRef.current === null) {
        logScreenshotBaseline(remoteSessionId || sessionId, "first_paint_guard_blocked", performance.now() - frontendSessionStartedAtRef.current);
        return;
      }
      logScreenshotBaseline(remoteSessionId || sessionId, "first_paint", performance.now() - frontendSessionStartedAtRef.current);
      logNativeScreenshotDiagnostics(remoteSessionId || sessionId, "native_diagnostics_status", performance.now() - frontendSessionStartedAtRef.current);
      void (async () => {
        if (sessionId !== captureIdRef.current) return;
        if (!wasNativeOverlayVisible) {
          try {
            logScreenshotBaseline(remoteSessionId || sessionId, "pre_show_candidate_load_start", performance.now() - frontendSessionStartedAtRef.current);
            await loadWindowRects(true);
            logScreenshotBaseline(remoteSessionId || sessionId, "pre_show_candidate_first_batch", performance.now() - frontendSessionStartedAtRef.current);
            draw(0, 0, 0, 0);
            await waitForAnimationFrame();
            logScreenshotBaseline(remoteSessionId || sessionId, "overlay_ready_to_show_called", performance.now() - frontendSessionStartedAtRef.current);
            await invoke("overlay_ready_to_show", { label: getCurrentWindow().label, sessionId: String(remoteSessionId || sessionId) });
            nativeOverlayVisibleRef.current = true;
            logScreenshotBaseline(remoteSessionId || sessionId, "overlay_ready_to_show_returned", performance.now() - frontendSessionStartedAtRef.current);
          } catch (error: any) {
            throw new Error(error?.message || String(error));
          }
        } else {
          logScreenshotBaseline(remoteSessionId || sessionId, "overlay_already_visible", performance.now() - frontendSessionStartedAtRef.current);
        }
        const focusCanvas = () => {
          const canvasEl = document.querySelector("canvas");
          if (canvasEl) canvasEl.focus({ preventScroll: true });
        };
        focusCanvas();
        window.setTimeout(focusCanvas, 60);
        captureAnalysisImageData(img, sessionId, remoteSessionId);
        window.setTimeout(() => {
          if (sessionId === captureIdRef.current) {
            logScreenshotBaseline(remoteSessionId || sessionId, "candidate_load_start", performance.now() - frontendSessionStartedAtRef.current);
            loadWindowRects(true).then(() => {
              logScreenshotBaseline(remoteSessionId || sessionId, "candidate_first_batch", performance.now() - frontendSessionStartedAtRef.current);
            }).catch(() => {});
          }
        }, 48);
      })().catch((error) => {
        if (sessionId !== captureIdRef.current) return;
        cancelScreenshot(error?.message || "Screenshot overlay failed");
      });
    });
  };

  const loadImageFromSource = (source: string, sessionId: number, bytes?: number, remoteSessionId?: string | number, revokeSource = false) => {
    if (sessionId !== captureIdRef.current) return;
    const img = new Image();
    img.crossOrigin = "anonymous";
    logScreenshotBaseline(remoteSessionId || sessionId, "file_load_start", performance.now() - frontendSessionStartedAtRef.current, `bytes=${bytes || 0}`);

    timeoutRef.current = setTimeout(() => {
      if (sessionId !== captureIdRef.current) return;
      if (imageRef.current === null) {
        cancelScreenshot("Screenshot overlay failed");
      }
    }, 1500);

    img.onload = async () => {
      if (sessionId !== captureIdRef.current) return;
      logScreenshotBaseline(remoteSessionId || sessionId, "file_load_end", performance.now() - frontendSessionStartedAtRef.current, `natural=${img.naturalWidth}x${img.naturalHeight}`);
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      if (revokeSource) {
        URL.revokeObjectURL(source);
      }
      try {
        logScreenshotBaseline(remoteSessionId || sessionId, "image_decode_start", performance.now() - frontendSessionStartedAtRef.current);
        await img.decode?.();
      } catch (e) {
        console.warn("[ScreenshotPage] img.decode failed", e);
      }
      if (sessionId !== captureIdRef.current) return;
      logScreenshotBaseline(remoteSessionId || sessionId, "image_decode_end", performance.now() - frontendSessionStartedAtRef.current);

      completeImageLoad(img, sessionId, bytes, remoteSessionId);
    };

    img.onerror = () => {
      if (sessionId !== captureIdRef.current) return;
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      if (revokeSource) {
        URL.revokeObjectURL(source);
      }
      cancelScreenshot("Screenshot overlay failed");
    };
    img.src = source;
  };

  const loadImageFromBase64 = (base64: string, sessionId: number, remoteSessionId?: string | number) => {
    if (sessionId !== captureIdRef.current) return;
    if (!base64 || base64.length < 1000) {
      cancelScreenshot("Screenshot overlay failed");
      return;
    }
    const dataUrl = "data:image/png;base64," + base64;
    loadImageFromSource(dataUrl, sessionId, Math.round(base64.length * 0.75), remoteSessionId);
  };

  const normalizeScreenshotBytes = (raw: unknown): Uint8Array | null => {
    if (raw instanceof ArrayBuffer) return new Uint8Array(raw);
    if (ArrayBuffer.isView(raw)) return new Uint8Array(raw.buffer, raw.byteOffset, raw.byteLength);
    if (Array.isArray(raw)) return new Uint8Array(raw as number[]);
    if (raw && typeof raw === "object") {
      const boxed = raw as { data?: unknown; bytes?: unknown; buffer?: unknown };
      for (const value of [boxed.data, boxed.bytes, boxed.buffer]) {
        const normalized = normalizeScreenshotBytes(value);
        if (normalized) return normalized;
      }
    }
    return null;
  };

  const describeScreenshotBytesShape = (raw: unknown) => {
    if (raw instanceof ArrayBuffer) return `ArrayBuffer byteLength=${raw.byteLength}`;
    if (ArrayBuffer.isView(raw)) return `${raw.constructor.name} byteLength=${raw.byteLength}`;
    if (Array.isArray(raw)) return `Array length=${raw.length}`;
    if (raw && typeof raw === "object") return `object keys=${Object.keys(raw as Record<string, unknown>).slice(0, 8).join(",")}`;
    return typeof raw;
  };

  const loadImageFromBytes = (raw: unknown, sessionId: number, bytes?: number, remoteSessionId?: string | number) => {
    if (sessionId !== captureIdRef.current) return false;
    const data = normalizeScreenshotBytes(raw);
    if (!data || data.byteLength < 1000) return false;
    const objectUrl = URL.createObjectURL(new Blob([data], { type: "image/png" }));
    loadImageFromSource(objectUrl, sessionId, bytes || data.byteLength, remoteSessionId, true);
    return true;
  };


  const loadImageFromRgbaBytes = (raw: unknown, width: number, height: number, sessionId: number, bytes?: number, remoteSessionId?: string | number) => {
    if (sessionId !== captureIdRef.current) return false;
    const data = normalizeScreenshotBytes(raw);
    const expectedBytes = width * height * 4;
    if (!data || width <= 0 || height <= 0 || data.byteLength < expectedBytes) {
      logScreenshotBaseline(
        remoteSessionId || sessionId,
        "rgba_rejected",
        performance.now() - frontendSessionStartedAtRef.current,
        `shape=${describeScreenshotBytesShape(raw)} normalized_bytes=${data?.byteLength || 0} expected=${expectedBytes} size=${width}x${height}`
      );
      return false;
    }
    const rgbaStartedAt = performance.now();
    const sourceCanvas = document.createElement("canvas");
    sourceCanvas.width = width;
    sourceCanvas.height = height;
    const sourceCtx = sourceCanvas.getContext("2d");
    if (!sourceCtx) return false;
    sourceCtx.putImageData(new ImageData(new Uint8ClampedArray(data.buffer, data.byteOffset, expectedBytes), width, height), 0, 0);
    logScreenshotBaseline(remoteSessionId || sessionId, "rgba_canvas_ready", performance.now() - frontendSessionStartedAtRef.current, `build_ms=${Math.round(performance.now() - rgbaStartedAt)} bytes=${bytes || data.byteLength}`);
    completeImageLoad(sourceCanvas, sessionId, bytes || data.byteLength, remoteSessionId);
    return true;
  };

  const loadFullscreenFromRgba = async (width: number, height: number, mode = "normal", remoteSessionId?: string | number, bytes?: number, physicalBounds?: ScreenshotPhysicalBounds | null) => {
    const sessionId = startNewCaptureSession(mode, remoteSessionId, overlayVisibleRef.current, physicalBounds);
    try {
      const rawStartedAt = performance.now();
      const raw = await invoke<unknown>("get_fullscreen_rgba_bytes", { sessionId: remoteSessionId == null ? null : String(remoteSessionId) });
      logScreenshotBaseline(remoteSessionId || sessionId, "rgba_fetch_end", performance.now() - frontendSessionStartedAtRef.current, `fetch_ms=${Math.round(performance.now() - rawStartedAt)} bytes=${bytes || 0} size=${width}x${height}`);
      if (loadImageFromRgbaBytes(raw, width, height, sessionId, bytes, remoteSessionId)) return;
    } catch (rawErr) {
      console.warn("[ScreenshotPage] rgba screenshot fetch failed, falling back to PNG", rawErr);
    }
    await loadFullscreen(mode, remoteSessionId, bytes, overlayVisibleRef.current, physicalBounds);
  };

  const loadFullscreen = async (mode = screenshotModeRef.current || "normal", remoteSessionId?: string | number, bytes?: number, preserveVisibleShell = false, physicalBounds?: ScreenshotPhysicalBounds | null) => {
    const sessionId = startNewCaptureSession(mode, remoteSessionId, preserveVisibleShell, physicalBounds);
    try {
      const binaryStartedAt = performance.now();
      const binary = await invoke<unknown>("get_fullscreen_image_bytes");
      logScreenshotBaseline(remoteSessionId || sessionId, "binary_fetch_end", performance.now() - frontendSessionStartedAtRef.current, `fetch_ms=${Math.round(performance.now() - binaryStartedAt)} bytes=${bytes || 0}`);
      if (loadImageFromBytes(binary, sessionId, bytes, remoteSessionId)) return;
    } catch (binaryErr) {
      console.warn("[ScreenshotPage] binary screenshot fetch failed, falling back to base64", binaryErr);
    }
    try {
      const base64StartedAt = performance.now();
      const base64 = await invoke<string>("get_fullscreen_image");
      logScreenshotBaseline(remoteSessionId || sessionId, "base64_fetch_end", performance.now() - frontendSessionStartedAtRef.current, `fetch_ms=${Math.round(performance.now() - base64StartedAt)} bytes=${bytes || 0}`);
      if (sessionId !== captureIdRef.current) return;
      if (!base64 || base64.length < 1000) {
        cancelScreenshot("Screenshot overlay failed");
        return;
      }
      loadImageFromBase64(base64, sessionId, remoteSessionId);
    } catch (err: any) {
      if (sessionId !== captureIdRef.current) return;
      cancelScreenshot(err?.message || "Screenshot overlay failed");
    }
  };

  const loadFullscreenFromBase64 = (base64: string, mode = "normal", remoteSessionId?: string | number, physicalBounds?: ScreenshotPhysicalBounds | null) => {
    const sessionId = startNewCaptureSession(mode, remoteSessionId, false, physicalBounds);
    try {
      if (!base64 || base64.length < 1000) {
        cancelScreenshot("Screenshot overlay failed");
        return;
      }
      loadImageFromBase64(base64, sessionId, remoteSessionId);
    } catch (err: any) {
      if (sessionId !== captureIdRef.current) return;
      cancelScreenshot(err?.message || "Screenshot overlay failed");
    }
  };

  const loadFullscreenFromFile = (path: string, bytes?: number, mode = "normal", remoteSessionId?: string | number, physicalBounds?: ScreenshotPhysicalBounds | null) => {
    const sessionId = startNewCaptureSession(mode, remoteSessionId, false, physicalBounds);
    try {
      if (!path) {
        loadFullscreen(mode, remoteSessionId, bytes, false, physicalBounds);
        return;
      }
      loadImageFromSource(`${convertFileSrc(path)}?t=${Date.now()}`, sessionId, bytes, remoteSessionId);
    } catch (err: any) {
      if (sessionId !== captureIdRef.current) return;
      loadFullscreen(mode, remoteSessionId, bytes, false, physicalBounds);
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
    overlayVisibleRef.current = false;
    nativeOverlayVisibleRef.current = false;
    setOverlayVisible(false);
    setDbgStatus({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });
    imageRef.current = null;
    translatedImgRef.current = null;
    analysisImageDataRef.current = null;
    displayedSessionIdRef.current = null;
    displayedPhysicalBoundsRef.current = null;
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
    nativeOverlayVisibleRef,
    timeoutRef,
    captureIdRef,
    displayedSessionIdRef,
    displayedPhysicalBoundsRef,
    setScreenshotState,
    setOverlayVisible,
    setDbgStatus,
    loadConfig,
    loadFullscreen,
    loadFullscreenFromBase64,
    loadFullscreenFromFile,
    loadFullscreenFromRgba,
    loadImageFromBase64,
    loadImageFromSource,
    initCanvas,
    resetScreenshotState,
    cancelScreenshot,
  };
}
