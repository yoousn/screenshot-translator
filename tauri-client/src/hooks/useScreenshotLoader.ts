import { useEffect, useState, useRef } from "react";
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
  rectRef: React.MutableRefObject<Rect>;
  hasSelectedRef: React.MutableRefObject<boolean>;
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
type WebViewSharedBufferEvent = {
  getBuffer: () => ArrayBuffer;
  additionalData?: Record<string, unknown>;
};
type WebViewSharedBufferHost = {
  addEventListener: (type: "sharedbufferreceived", handler: (event: WebViewSharedBufferEvent) => void) => void;
  removeEventListener: (type: "sharedbufferreceived", handler: (event: WebViewSharedBufferEvent) => void) => void;
  releaseBuffer?: (buffer: ArrayBuffer) => void;
};
type PendingSharedBuffer = { buffer: ArrayBuffer; receivedAt: number };
type SharedBufferReceiver = {
  promise: Promise<ArrayBuffer | undefined>;
  cancel: () => void;
  release: (buffer: ArrayBuffer) => void;
  source: "pending" | "waiter";
};

const getScreenshotImageWidth = (image: ScreenshotImageSource) => image instanceof HTMLImageElement ? image.naturalWidth : image.width;
const getScreenshotImageHeight = (image: ScreenshotImageSource) => image instanceof HTMLImageElement ? image.naturalHeight : image.height;

export function useScreenshotLoader({
  screenshotModeRef,
  configRef,
  setConfig,
  loadWindowRects,
  clearWindowRects,
  clearScrollCaptureState,
  clearRecordingState,
  resetAnnotations,
  rectRef,
  hasSelectedRef,
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
  const sharedBufferHostRef = useRef<WebViewSharedBufferHost | null>(null);
  const pendingSharedBuffersRef = useRef<Map<string, PendingSharedBuffer>>(new Map());
  const sharedBufferWaitersRef = useRef<Map<string, (buffer: ArrayBuffer | undefined) => void>>(new Map());

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
    const preserveShellSelection = preserveVisibleShell && (
      hasSelectedRef.current || rectRef.current.w > 0 || rectRef.current.h > 0
    );
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

    if (!preserveShellSelection) {
      setCurrentRect(EMPTY_RECT, true);
      setSelection(false);
    }
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
    await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
    resetScreenshotState();
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

  const recoverPreShowDrag = async (sessionKey: string | number, sessionId: number) => {
    let loggedStart = false;
    const wait = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));
    const clampToViewport = (value: number, max: number) => Math.max(0, Math.min(max, value));
    const applyPreShowRect = (preCapture: any, finalize: boolean) => {
      const maxW = Math.max(1, window.innerWidth);
      const maxH = Math.max(1, window.innerHeight);
      const startX = clampToViewport(Math.round(Number(preCapture.x) || 0), maxW - 1);
      const startY = clampToViewport(Math.round(Number(preCapture.y) || 0), maxH - 1);
      const currentX = clampToViewport(Math.round(Number(preCapture.currentX) || startX), maxW - 1);
      const currentY = clampToViewport(Math.round(Number(preCapture.currentY) || startY), maxH - 1);
      const next = {
        x: Math.min(startX, currentX),
        y: Math.min(startY, currentY),
        w: Math.abs(currentX - startX),
        h: Math.abs(currentY - startY),
      };
      setCurrentRect(next, true);
      draw(next.x, next.y, next.w, next.h);
      const valid = next.w > 5 && next.h > 5;
      setSelection(finalize && valid);
      return { next, valid };
    };

    for (let index = 0; index < 48; index += 1) {
      if (sessionId !== captureIdRef.current) return;
      if (!overlayVisibleRef.current) return;
      if (!loggedStart && (hasSelectedRef.current || rectRef.current.w > 5 || rectRef.current.h > 5)) return;
      const pointerState = await invoke<any>("get_screenshot_pointer_state", { label: getCurrentWindow().label, sessionId: String(sessionKey) }).catch(() => null);
      const nativeOverlay = pointerState?.nativeOverlay;
      if (nativeOverlay?.cancelled === true && String(nativeOverlay.sessionId) === String(sessionKey)) {
        logScreenshotBaseline(
          sessionKey,
          "native_overlay_cancel_received",
          performance.now() - frontendSessionStartedAtRef.current,
          `phase=${String(nativeOverlay.phase || "cancelled")} event_seq=${Number(nativeOverlay.eventSeq) || 0}`
        );
        await cancelScreenshot("native-overlay-cancelled");
        return;
      }
      const preCapture = pointerState?.nativeOverlay?.available === true ? pointerState.nativeOverlay : pointerState?.preCapture;
      if (!preCapture || preCapture.available !== true || String(preCapture.sessionId) !== String(sessionKey)) return;
      const source = String(preCapture.source || "pre-capture");
      const phase = String(preCapture.phase || "unknown");
      const eventSeq = Number(preCapture.eventSeq) || 0;
      const dragDistance = Number(preCapture.dragDistance) || 0;
      const leftDown = preCapture.leftDown === true;
      const completed = preCapture.completed === true;
      if (dragDistance < 3 && leftDown) {
        await wait(16);
        continue;
      }
      if (dragDistance < 3 && !completed) return;
      const { next, valid } = applyPreShowRect(preCapture, !leftDown);
      if (!loggedStart) {
        loggedStart = true;
        logScreenshotBaseline(
          sessionKey,
          "pre_show_drag_recovered",
          performance.now() - frontendSessionStartedAtRef.current,
          `source=${source} phase=${phase} event_seq=${eventSeq} left_down=${leftDown} completed=${completed} drag=${Math.round(dragDistance)} rect=${Math.round(next.x)},${Math.round(next.y)},${Math.round(next.w)},${Math.round(next.h)}`
        );
      }
      if (!leftDown) {
        logScreenshotBaseline(
          sessionKey,
          "pre_show_drag_finalized",
          performance.now() - frontendSessionStartedAtRef.current,
          `source=${source} phase=${phase} event_seq=${eventSeq} valid=${valid} drag=${Math.round(dragDistance)}`
        );
        return;
      }
      await wait(16);
    }
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
    const preservedRect = rectRef.current;
    if (!hasSelectedRef.current && preservedRect.w <= 0 && preservedRect.h <= 0) {
      setCurrentRect(EMPTY_RECT, true);
      setSelection(false);
      draw(0, 0, 0, 0);
      return;
    }
    draw(preservedRect.x, preservedRect.y, preservedRect.w, preservedRect.h);
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
    logScreenshotBaseline(remoteSessionId || sessionId, "image_ready", performance.now() - frontendSessionStartedAtRef.current, `bytes=${bytes || 0}`);

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
      window.setTimeout(() => {
        if (sessionId !== captureIdRef.current || imageRef.current === null || maskedCanvasRef.current === null) {
          logScreenshotBaseline(remoteSessionId || sessionId, "first_paint_guard_blocked", performance.now() - frontendSessionStartedAtRef.current);
          return;
        }
        logScreenshotBaseline(remoteSessionId || sessionId, "first_paint", performance.now() - frontendSessionStartedAtRef.current, "gate=post-paint-task");
        void (async () => {
          if (sessionId !== captureIdRef.current) return;
          if (!wasNativeOverlayVisible) {
            try {
              logScreenshotBaseline(remoteSessionId || sessionId, "overlay_ready_to_show_called", performance.now() - frontendSessionStartedAtRef.current);
              await invoke("overlay_ready_to_show", { label: getCurrentWindow().label, sessionId: String(remoteSessionId || sessionId) });
              nativeOverlayVisibleRef.current = true;
              logScreenshotBaseline(remoteSessionId || sessionId, "overlay_ready_to_show_returned", performance.now() - frontendSessionStartedAtRef.current);
              void recoverPreShowDrag(remoteSessionId || sessionId, sessionId);
              window.setTimeout(() => {
                logNativeScreenshotDiagnostics(remoteSessionId || sessionId, "native_diagnostics_status", performance.now() - frontendSessionStartedAtRef.current);
              }, 120);
            } catch (error: any) {
              throw new Error(error?.message || String(error));
            }
          } else {
            logScreenshotBaseline(remoteSessionId || sessionId, "overlay_already_visible", performance.now() - frontendSessionStartedAtRef.current);
            void recoverPreShowDrag(remoteSessionId || sessionId, sessionId);
            window.setTimeout(() => {
              logNativeScreenshotDiagnostics(remoteSessionId || sessionId, "native_diagnostics_status", performance.now() - frontendSessionStartedAtRef.current);
            }, 120);
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
                draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
              }).catch(() => {});
            }
          }, 48);
        })().catch((error) => {
          if (sessionId !== captureIdRef.current) return;
          cancelScreenshot(error?.message || "Screenshot overlay failed");
        });
      }, 0);
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

  const getWebViewSharedBufferHost = (): WebViewSharedBufferHost | null => {
    const host = (window as any)?.chrome?.webview;
    if (!host || typeof host.addEventListener !== "function" || typeof host.removeEventListener !== "function") {
      return null;
    }
    return host as WebViewSharedBufferHost;
  };

  const getSharedBufferSessionId = (event: WebViewSharedBufferEvent) => {
    const data = event.additionalData || {};
    const rawSessionId = data.session_id ?? data.sessionId;
    return rawSessionId == null ? "" : String(rawSessionId);
  };

  const getSharedBufferTransferType = (event: WebViewSharedBufferEvent) => {
    const data = event.additionalData || {};
    const rawTransferType = data.transfer_type ?? data.transferType;
    return rawTransferType == null ? "" : String(rawTransferType);
  };

  const releaseWebViewSharedBuffer = (buffer: ArrayBuffer) => {
    try {
      (sharedBufferHostRef.current || getWebViewSharedBufferHost())?.releaseBuffer?.(buffer);
    } catch {
      // SharedBuffer release is best-effort; the fallback path remains valid.
    }
  };

  const prunePendingSharedBuffers = (preserveSessionId?: string) => {
    const entries = Array.from(pendingSharedBuffersRef.current.entries());
    const staleBefore = performance.now() - 5000;
    for (const [sessionId, pending] of entries) {
      if (preserveSessionId && sessionId === preserveSessionId) continue;
      if (pending.receivedAt < staleBefore || entries.length > 6) {
        pendingSharedBuffersRef.current.delete(sessionId);
        releaseWebViewSharedBuffer(pending.buffer);
      }
    }
  };

  const clearPendingSharedBuffers = () => {
    for (const pending of pendingSharedBuffersRef.current.values()) {
      releaseWebViewSharedBuffer(pending.buffer);
    }
    pendingSharedBuffersRef.current.clear();
    for (const resolve of sharedBufferWaitersRef.current.values()) {
      resolve(undefined);
    }
    sharedBufferWaitersRef.current.clear();
  };

  useEffect(() => {
    const host = getWebViewSharedBufferHost();
    sharedBufferHostRef.current = host;
    if (!host) return;

    const handleSharedBufferReceived = (event: WebViewSharedBufferEvent) => {
      if (getSharedBufferTransferType(event) !== "screenshot") return;
      const sessionId = getSharedBufferSessionId(event);
      if (!sessionId) return;

      let buffer: ArrayBuffer | undefined;
      try {
        buffer = event.getBuffer();
      } catch {
        logScreenshotBaseline(sessionId, "shared_buffer_direct_get_failed", performance.now() - (frontendSessionStartedAtRef.current || performance.now()));
        return;
      }

      const waiter = sharedBufferWaitersRef.current.get(sessionId);
      if (waiter) {
        sharedBufferWaitersRef.current.delete(sessionId);
        waiter(buffer);
        return;
      }

      const previous = pendingSharedBuffersRef.current.get(sessionId);
      if (previous) {
        releaseWebViewSharedBuffer(previous.buffer);
      }
      pendingSharedBuffersRef.current.set(sessionId, { buffer, receivedAt: performance.now() });
      logScreenshotBaseline(sessionId, "shared_buffer_direct_pending", 0, `bytes=${buffer.byteLength}`);
      prunePendingSharedBuffers(sessionId);
    };

    host.addEventListener("sharedbufferreceived", handleSharedBufferReceived);
    return () => {
      host.removeEventListener("sharedbufferreceived", handleSharedBufferReceived);
      clearPendingSharedBuffers();
      sharedBufferHostRef.current = null;
    };
  }, []);

  const createScreenshotSharedBufferReceiver = (expectedSessionId: string | number, timeoutMs = 3000): SharedBufferReceiver | null => {
    const host = getWebViewSharedBufferHost();
    if (!host) {
      return null;
    }

    const sessionId = String(expectedSessionId);
    const pending = pendingSharedBuffersRef.current.get(sessionId);
    if (pending) {
      pendingSharedBuffersRef.current.delete(sessionId);
      return {
        promise: Promise.resolve(pending.buffer),
        cancel: () => {},
        release: releaseWebViewSharedBuffer,
        source: "pending",
      };
    }

    let settled = false;
    let timeoutId: number | null = null;
    const cleanup = () => {
      if (timeoutId !== null) {
        window.clearTimeout(timeoutId);
        timeoutId = null;
      }
      sharedBufferWaitersRef.current.delete(sessionId);
    };

    let resolveReceiver: (buffer: ArrayBuffer | undefined) => void = () => {};
    const promise = new Promise<ArrayBuffer | undefined>((resolve) => {
      resolveReceiver = resolve;
      const finish = (buffer?: ArrayBuffer) => {
        if (settled) return;
        settled = true;
        cleanup();
        resolve(buffer);
      };
      sharedBufferWaitersRef.current.set(sessionId, finish);
      timeoutId = window.setTimeout(() => finish(undefined), timeoutMs);
    });

    return {
      promise,
      cancel: () => {
        if (settled) return;
        settled = true;
        cleanup();
        resolveReceiver(undefined);
      },
      release: releaseWebViewSharedBuffer,
      source: "waiter",
    };
  };

  const getSharedBufferImageInfo = (buffer: ArrayBuffer, fallbackWidth: number, fallbackHeight: number) => {
    if (buffer.byteLength >= 8) {
      const imageBytes = buffer.byteLength - 8;
      const dataView = new DataView(buffer, imageBytes, 8);
      const width = dataView.getUint32(0, true);
      const height = dataView.getUint32(4, true);
      const expected = width * height * 4;
      if (width > 0 && height > 0 && expected > 0 && expected <= imageBytes) {
        return { width, height, imageBytes };
      }
    }
    return { width: fallbackWidth, height: fallbackHeight, imageBytes: fallbackWidth * fallbackHeight * 4 };
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

  const tryLoadFullscreenFromSharedBuffer = async (
    width: number,
    height: number,
    sessionId: number,
    remoteSessionId?: string | number,
  ) => {
    if (remoteSessionId == null) {
      logScreenshotBaseline(sessionId, "shared_buffer_skipped", performance.now() - frontendSessionStartedAtRef.current, "reason=missing_remote_session_id");
      return false;
    }

    const directReceiver = createScreenshotSharedBufferReceiver(remoteSessionId, 24);
    if (!directReceiver) {
      logScreenshotBaseline(remoteSessionId, "shared_buffer_skipped", performance.now() - frontendSessionStartedAtRef.current, "reason=window_chrome_webview_unavailable");
      return false;
    }

    const directBuffer = await directReceiver.promise;
    if (sessionId !== captureIdRef.current) return false;
    if (directBuffer) {
      try {
        const info = getSharedBufferImageInfo(directBuffer, width, height);
        logScreenshotBaseline(remoteSessionId, "shared_buffer_received", performance.now() - frontendSessionStartedAtRef.current, `source=direct bytes=${directBuffer.byteLength} image_bytes=${info.imageBytes} size=${info.width}x${info.height}`);
        return loadImageFromRgbaBytes(directBuffer, info.width, info.height, sessionId, info.imageBytes, remoteSessionId);
      } finally {
        directReceiver.release(directBuffer);
      }
    }
    directReceiver.cancel();
    logScreenshotBaseline(remoteSessionId, "shared_buffer_direct_wait_miss", performance.now() - frontendSessionStartedAtRef.current);
    if (sessionId !== captureIdRef.current) return false;

    const receiver = createScreenshotSharedBufferReceiver(remoteSessionId);
    if (!receiver) {
      logScreenshotBaseline(remoteSessionId, "shared_buffer_skipped", performance.now() - frontendSessionStartedAtRef.current, "reason=window_chrome_webview_unavailable_after_direct_wait");
      return false;
    }

    if (receiver.source !== "pending") {
      try {
        const postStartedAt = performance.now();
        const postResult = await invoke<any>("post_fullscreen_rgba_shared_buffer", { sessionId: String(remoteSessionId) });
        logScreenshotBaseline(
          remoteSessionId,
          "shared_buffer_post_returned",
          performance.now() - frontendSessionStartedAtRef.current,
          `post_ms=${Math.round(performance.now() - postStartedAt)} posted=${postResult?.posted === true} bytes=${postResult?.bytes || 0}`
        );
        if (postResult?.posted !== true) {
          receiver.cancel();
          return false;
        }
      } catch (error: any) {
        receiver.cancel();
        logScreenshotBaseline(remoteSessionId, "shared_buffer_invoke_failed", performance.now() - frontendSessionStartedAtRef.current, `reason=${error?.message || String(error)}`);
        return false;
      }
    }

    const buffer = await receiver.promise;
    if (sessionId !== captureIdRef.current) {
      if (buffer) receiver.release(buffer);
      return false;
    }
    if (!buffer) {
      logScreenshotBaseline(remoteSessionId, "shared_buffer_receive_timeout", performance.now() - frontendSessionStartedAtRef.current);
      return false;
    }

    try {
      const info = getSharedBufferImageInfo(buffer, width, height);
      logScreenshotBaseline(remoteSessionId, "shared_buffer_received", performance.now() - frontendSessionStartedAtRef.current, `source=${receiver.source === "pending" ? "late-direct" : "requested"} bytes=${buffer.byteLength} image_bytes=${info.imageBytes} size=${info.width}x${info.height}`);
      return loadImageFromRgbaBytes(buffer, info.width, info.height, sessionId, info.imageBytes, remoteSessionId);
    } finally {
      receiver.release(buffer);
    }
  };

  const loadFullscreenFromRgba = async (width: number, height: number, mode = "normal", remoteSessionId?: string | number, bytes?: number, physicalBounds?: ScreenshotPhysicalBounds | null) => {
    const sessionId = startNewCaptureSession(mode, remoteSessionId, overlayVisibleRef.current, physicalBounds);
    if (await tryLoadFullscreenFromSharedBuffer(width, height, sessionId, remoteSessionId)) return;
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
    captureIdRef.current += 1;
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
    maskedCanvasRef.current = null;
    analysisImageDataRef.current = null;
    displayedSessionIdRef.current = null;
    displayedPhysicalBoundsRef.current = null;
    clearPendingSharedBuffers();
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
