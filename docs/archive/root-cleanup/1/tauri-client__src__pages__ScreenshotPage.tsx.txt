import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { join, tempDir } from "@tauri-apps/api/path";
import { Space, Button, message } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ScreenshotToolbar from "../components/screenshot/ScreenshotToolbar";
import TextAnnotationEditor from "../components/screenshot/TextAnnotationEditor";
import TranslationLoadingOverlay from "../components/screenshot/TranslationLoadingOverlay";
import type { Config } from "../types/config";
import type { Annotation, AnnotationTool, Rect, ScreenshotUpdatedPayload } from "../types/screenshot";
import { useScreenshotOcr } from "../hooks/useScreenshotOcr";
import { useScreenshotAnnotation, DEFAULT_ANNOTATION_COLOR, DEFAULT_ANNOTATION_TOOL, DEFAULT_ANNOTATION_SIZES } from "../hooks/useScreenshotAnnotation";
import { getActionToolbarStyle, FLOATING_PANEL_MARGIN, FLOATING_PANEL_GAP } from "../utils/screenshotLayout";
import { renderScreenshotCanvas } from "../utils/renderScreenshotCanvas";
import RecordingTargetPicker from "../components/recording/RecordingTargetPicker";
import { useScreenshotWindowRects } from "../hooks/useScreenshotWindowRects";
import { useScreenshotRecording } from "../hooks/useScreenshotRecording";
import { useScrollCapture } from "../hooks/useScrollCapture";
import { useScreenshotLoader } from "../hooks/useScreenshotLoader";
import { useScreenshotTextSource } from "../hooks/useScreenshotTextSource";
import { useScreenshotActions } from "../hooks/useScreenshotActions";
import { useScreenshotInteraction } from "../hooks/useScreenshotInteraction";
import { useScreenshotMagnifier } from "../hooks/useScreenshotMagnifier";
import { prewarmTranslationServices } from "../utils/localOcrTranslate";
import { getLogicalCanvasSize } from "../utils/screenshotViewport";

const ACTION_TOOLBAR_FALLBACK_SIZE = { width: 680, height: 86 };
const RECORDING_TOOLBAR_FALLBACK_SIZE = { width: 980, height: 96 };
const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };
const RECORDING_BORDER_COLOR = "#ef4444";
const RECORDING_BORDER_BLUE = "#2563eb";
const RECORDING_BORDER_RED = "#ef4444";
const RECORDING_BORDER_YELLOW = "#f59e0b";
const SCROLL_CAPTURE_BORDER_COLOR = "#f97316";
const WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_ENABLED = import.meta.env.VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT === "1";
const WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_REAL_CLIPBOARD_ENABLED = import.meta.env.VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_REAL_CLIPBOARD === "1";
const WGC_SELECTED_OUTPUT_AUTO_ACCEPTANCE_SMOKE_ENABLED = import.meta.env.VITE_YSN_WGC_SELECTED_OUTPUT_AUTO_ACCEPTANCE_SMOKE === "1";

type SelectedImageBridgeAction = "copy" | "save" | "ocr" | "translate";

const waitForShellPaint = () => new Promise<void>((resolve) => {
  let settled = false;
  const finish = () => {
    if (settled) return;
    settled = true;
    resolve();
  };
  const timeoutId = window.setTimeout(finish, 24);
  window.requestAnimationFrame(() => {
    window.clearTimeout(timeoutId);
    finish();
  });
});

export default function ScreenshotPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const mouseTrackerRef = useRef<HTMLDivElement>(null);
  const actionToolbarRef = useRef<HTMLDivElement>(null);
  const actionToolbarSizeRef = useRef(ACTION_TOOLBAR_FALLBACK_SIZE);
  const liveToolbarFrameRef = useRef<number | null>(null);
  const liveToolbarRectRef = useRef<Rect | null>(null);
  const lastMouseRef = useRef({ x: 0, y: 0 });
  const autoAcceptanceSmokeStartedRef = useRef(false);
  const lastScreenshotPayloadSignatureRef = useRef<string | null>(null);
  const lastScreenshotShellSignatureRef = useRef<string | null>(null);
  
  const [isSelecting, setIsSelecting] = useState(false);
  const [rect, setRect] = useState<Rect>(EMPTY_RECT);
  const [actionToolbarSize, setActionToolbarSize] = useState(ACTION_TOOLBAR_FALLBACK_SIZE);
  const [hasSelected, setHasSelected] = useState(false);
  const [screenshotMode, setScreenshotMode] = useState("normal");
  const [config, setConfig] = useState<Config>({});
  const [isEditing, setIsEditing] = useState(false);

  const renderNeededRef = useRef(false);
  const requestRef = useRef<number | null>(null);
  const renderFramePendingRef = useRef(false);
  const selectionStartedAtRef = useRef(0);
  const selectionCompletedAtRef = useRef(0);
  const selectionDragDistanceRef = useRef(0);
  const pendingConfirmTimerRef = useRef<number | null>(null);
  
  const isOCRingRef = useRef(false);
  const isTranslatingRef = useRef(false);

  const hasSelectedRef = useRef(false);
  const rectRef = useRef<Rect>(EMPTY_RECT);
  const configRef = useRef<Config>({});
  const isSelectingRef = useRef(false);
  const isEditingRef = useRef(false);
  const screenshotModeRef = useRef("normal");
  const frameInteractiveRef = useRef(false);
  const imageReadyRef = useRef(false);
  const drawRef = useRef(draw);
  const magnifierCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const magnifierHexRef = useRef<string>("");
  const magnifierVisibleRef = useRef(false);
  const [magnifier, setMagnifier] = useState<{ clientX: number; clientY: number; cx: number; cy: number; hex: string } | null>(null);

  // Break circular dependency
  const captureRegionBase64Ref = useRef<(action?: SelectedImageBridgeAction) => Promise<string>>(() => Promise.resolve(""));

  useEffect(() => {
    hasSelectedRef.current = hasSelected;
  }, [hasSelected]);

  useEffect(() => {
    rectRef.current = rect;
  }, [rect]);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  useEffect(() => {
    isEditingRef.current = isEditing;
  }, [isEditing]);

  useEffect(() => {
    isSelectingRef.current = isSelecting;
  }, [isSelecting]);

  useEffect(() => {
    screenshotModeRef.current = screenshotMode;
  }, [screenshotMode]);

  useEffect(() => {
    drawRef.current = draw;
  }, [draw]);

  useEffect(() => {
    actionToolbarSizeRef.current = actionToolbarSize;
  }, [actionToolbarSize]);


  const interactionStateRef = useRef({
    get hasSelected() { return hasSelectedRef.current; },
    get isSelecting() { return isSelectingRef.current; },
  });

  const scheduleRenderFrame = () => {
    if (renderFramePendingRef.current) return;
    renderFramePendingRef.current = true;
    requestRef.current = requestAnimationFrame(() => {
      requestRef.current = null;
      renderFramePendingRef.current = false;
      if (!renderNeededRef.current) return;
      renderNeededRef.current = false;
      drawRef.current(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
      if (renderNeededRef.current) scheduleRenderFrame();
    });
  };

  const triggerRender = () => {
    renderNeededRef.current = true;
    scheduleRenderFrame();
  };

  const setCurrentRect = (next: Rect, syncState = false) => {
    rectRef.current = next;
    if (syncState) setRect(next);
  };

  const setSelection = (selected: boolean) => {
    if (selected && !hasSelectedRef.current) {
      selectionCompletedAtRef.current = performance.now();
    }
    if (!selected) {
      selectionCompletedAtRef.current = 0;
      selectionDragDistanceRef.current = 0;
    }
    hasSelectedRef.current = selected;
    setHasSelected(selected);
  };

  const buildAutomationAcceptanceRect = (): Rect | null => {
    const canvas = canvasRef.current;
    if (!canvas || canvas.width <= 12 || canvas.height <= 12) return null;
    const width = Math.max(96, Math.min(640, Math.floor(canvas.width * 0.32)));
    const height = Math.max(72, Math.min(360, Math.floor(canvas.height * 0.28)));
    const x = Math.max(4, Math.min(canvas.width - width - 4, Math.floor(canvas.width * 0.18)));
    const y = Math.max(4, Math.min(canvas.height - height - 4, Math.floor(canvas.height * 0.18)));
    const w = Math.max(0, Math.min(width, canvas.width - x));
    const h = Math.max(0, Math.min(height, canvas.height - y));
    if (w <= 5 || h <= 5) return null;
    return { x, y, w, h };
  };

  const ensureAutomationAcceptanceSelection = () => {
    if (hasSelectedRef.current && rectRef.current.w > 5 && rectRef.current.h > 5) {
      return { ok: true, synthesized: false, rect: rectRef.current };
    }
    const next = buildAutomationAcceptanceRect();
    if (!next) return { ok: false, synthesized: false, rect: null };
    selectionDragDistanceRef.current = Math.hypot(next.w, next.h);
    setCurrentRect(next, true);
    setSelection(true);
    drawRef.current(next.x, next.y, next.w, next.h);
    syncActionToolbarPosition(next);
    return { ok: true, synthesized: true, rect: next };
  };

  const buildWgcAcceptanceReportPngPath = async () => {
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
    return await join(await tempDir(), `ysn-wgc-alt-a-acceptance-${timestamp}.png`);
  };

  // 1. useScreenshotTextSource
  const {
    textSourceSnapshotPromiseRef,
    primeTextSourceSnapshot,
    getTextSourceBlocksForCurrentSelection,
  } = useScreenshotTextSource({
    canvasRef,
    imageRef: { get current() { return imageRef.current; } } as any,
    rectRef,
  });

  // 2. useScreenshotAnnotation
  const {
    annotationTool, setAnnotationTool,
    annotationColor, setAnnotationColor,
    annotationSize, setAnnotationSize: setAnnotationSizeState,
    selectedAnnotationIndex, setSelectedAnnotationIndex,
    editingTextDraft, setEditingTextDraft,
    annotations, setAnnotations,
    annotationHistory, setAnnotationHistory,
    redoAnnotations, setRedoAnnotations,
    draftAnnotation, setAnnotationDraft,

    annotationToolRef, annotationColorRef, annotationSizeRef, annotationSizesRef,
    selectedAnnotationIndexRef, annotationsRef, annotationHistoryRef, redoAnnotationsRef,
    draftAnnotationRef, editingTextDraftRef,

    pushAnnotationHistory, undoAnnotation, redoAnnotation, commitAnnotation,
    cancelTextDraft, commitTextDraft, deleteSelectedAnnotation, resetAnnotations,

    markerShape, setMarkerShape, markerShapeRef,
  } = useScreenshotAnnotation(() => {
    triggerRender();
  });

  // 3. useScreenshotWindowRects
  const {
    windowRects,
    hoverRect,
    hoverCandidates,
    windowRectsRef,
    hoverRectRef,
    hoverCandidatesRef,
    hoverCandidateIndexRef,
    setHoverCandidate,
    setHoverCandidateList,
    loadWindowRects,
    clearWindowRects,
  } = useScreenshotWindowRects({
    configRef,
    lastMouseRef,
    analysisImageDataRef: { get current() { return analysisImageDataRef.current; } } as any,
    interactionStateRef: {
      get current() {
        return {
          hasSelected: hasSelectedRef.current,
          isSelecting: isSelectingRef.current,
          isDragging: false,
          isResizing: false,
        };
      }
    } as any,
    triggerRender,
  });

  // 4. useScreenshotRecording
  const {
    recordingStatus,
    recordingPickerMode,
    recordingFps,
    recordingResolution,
    recordingAudioMode,
    recordingMode,
    recordingTargets,
    selectedWindowTargetId,
    selectedDisplayTargetId,
    recordingInfo,
    isRecordingBusy,
    recordingStartedAt,
    recordingElapsedMs,
    recordingStatusRef,
    recordingPickerModeRef,
    recordingModeRef,
    isRecordingBusyRef,
    recordingStartedAtRef,
    recordingSegmentsRef,
    setRecordingFps,
    setRecordingResolution,
    setRecordingAudioMode,
    setRecordingStatus,
    setRecordingPickerMode,
    setRecordingMode,
    enterRecordingMode,
    cancelRecordingTargetPicker,
    confirmRecordingTargetPicker,
    selectRecordingTarget,
    startRecording,
    finishRecording,
    cancelRecording,
    clearRecordingState,
    formatAudioDeviceLabel,
    getRecordingDevices,
    loadRecordingPrerequisites,
  } = useScreenshotRecording({
    rectRef,
    canvasRef,
    imageRef: { get current() { return imageRef.current; } } as any,
    screenshotModeRef,
    triggerRender,
    setCurrentRect,
    setSelection,
    setHoverCandidate,
    resetScreenshotState: () => resetScreenshotState(),
  });

  // 5. useScrollCapture
  const {
    isScrollCapturing,
    scrollCaptureMode,
    scrollPreviewBase64,
    isScrollCapturingRef,
    scrollCaptureModeRef,
    scrollFramesRef,
    scrollTimerRef,
    handleScrollCapture,
    startManualScrollCapture,
    finishManualScrollCapture,
    cancelManualScrollCapture,
    clearScrollCaptureState,
  } = useScrollCapture({
    rectRef,
    canvasRef,
    imageRef: { get current() { return imageRef.current; } } as any,
    triggerRender,
    resetScreenshotState: () => resetScreenshotState(),
  });

  // 6. useScreenshotLoader
  const {
    screenshotState,
    overlayVisible,
    dbgStatus,
    imageRef,
    translatedImgRef,
    maskedCanvasRef,
    analysisImageDataRef,
    overlayVisibleRef,
    nativeOverlayVisibleRef,
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
    resetScreenshotState,
    cancelScreenshot,
  } = useScreenshotLoader({
    screenshotModeRef,
    configRef,
    setConfig,
    loadWindowRects,
    clearWindowRects,
    clearScrollCaptureState,
    clearRecordingState,
    resetAnnotations,
    resetInteractionState: () => resetInteractionState(),
    rectRef,
    hasSelectedRef,
    setCurrentRect,
    setSelection,
    resetOcrState: () => resetOcrState(),
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
    prewarmLocalOcrWorker: (reason) => prewarmLocalOcrWorker(reason),
    draw: (...args) => drawRef.current(...args),
    textSourceSnapshotPromiseRef,
    pendingConfirmTimerRef,
  });

  const canInteractWithOverlay = overlayVisible;
  frameInteractiveRef.current = canInteractWithOverlay;
  imageReadyRef.current = Boolean(imageRef.current);

  // 7. useScreenshotOcr (Initialized before actions to avoid temporal dead zone)
  const {
    isOCRing,
    isTranslating,
    translatePairs,
    translatedResult,
    prewarmLocalOcrWorker,
    handleOCR,
    handleTranslate,
    handleShowTranslateResult,
    isOCRingRef: ocrIsOCRingRef,
    isTranslatingRef: ocrIsTranslatingRef,
    resetOcrState,
  } = useScreenshotOcr({
    config,
    rectRef,
    captureRegionBase64: (action) => captureRegionBase64Ref.current(action),
    resetScreenshotState,
    draw: (...args) => drawRef.current(...args),
    translatedImgRef,
    getTextSourceBlocksForCurrentSelection,
  });

  // 8. useScreenshotActions
  const {
    cropCurrentSelectionFromLoadedImage,
    captureRegionBase64,
    renderCurrentEditedSelectionBase64,
    getOutputBase64,
    runGuardedWgcExplicitSelectionDiagnostic,
    getSelectionConfirmDelayMs,
    canConfirmCurrentSelection,
    handlePin,
    forceCloseScreenshots,
    confirmScreenshot,
  } = useScreenshotActions({
    canvasRef,
    imageRef: imageRef as any,
    displayedSessionIdRef,
    displayedPhysicalBoundsRef,
    rectRef,
    rect,
    hasSelected,
    translatedResult,
    annotationsRef,
    annotationColorRef,
    annotationSizeRef,
    overlayVisibleRef,
    selectionCompletedAtRef,
    pendingConfirmTimerRef,
    recordingSegmentsRef,
    interactionStateRef: {
      get current() {
        return {
          hasSelected: hasSelectedRef.current,
          isSelecting: isSelectingRef.current,
          isDragging: false,
          isResizing: false,
        };
      }
    } as any,
    annotationStateRef: {
      get current() {
        return annotationStateRef.current;
      }
    } as any,
    resetScreenshotState,
    cancelScreenshot,
  });

  const handleWgcExplicitSelectionDiagnostic = async () => {
    const key = "wgc-explicit-selection-diagnostic";
    const reportEnabled = WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_ENABLED || WGC_SELECTED_OUTPUT_AUTO_ACCEPTANCE_SMOKE_ENABLED;
    if (reportEnabled) {
      const selection = ensureAutomationAcceptanceSelection();
      if (!selection.ok) {
        message.error({ content: "WGC 选区验收未运行：无法建立有效自动化选区", key, duration: 3 });
        return;
      }
    }
    message.loading({ content: "WGC 选区诊断运行中...", key, duration: 0 });
    const savePath = reportEnabled ? await buildWgcAcceptanceReportPngPath() : undefined;
    if (reportEnabled) {
      invoke("log_screenshot_perf", {
        message: `[wgc-acceptance] start auto=${WGC_SELECTED_OUTPUT_AUTO_ACCEPTANCE_SMOKE_ENABLED} realClipboard=${WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_REAL_CLIPBOARD_ENABLED} file=${savePath || ""} rect=${JSON.stringify(rectRef.current)}`,
      }).catch(() => {});
    }
    const response = await runGuardedWgcExplicitSelectionDiagnostic(reportEnabled ? {
      allowFakeClipboardSink: !WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_REAL_CLIPBOARD_ENABLED,
      allowRealClipboard: WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_REAL_CLIPBOARD_ENABLED,
      includeSelectedPngBase64: true,
      allowFileWrite: true,
      savePath,
    } : undefined);
    if (!response) {
      if (reportEnabled) {
        invoke("log_screenshot_perf", {
          message: `[wgc-acceptance] ok=false reason=no-response file=${savePath || ""}`,
        }).catch(() => {});
      }
      message.error({ content: "WGC 选区诊断未运行：缺少有效选区或物理屏幕范围", key, duration: 3 });
      return;
    }
    if (response.ok) {
      const selectedFile = response.selectedFile as { path?: unknown } | null | undefined;
      const savedPath = typeof selectedFile?.path === "string" ? selectedFile.path : savePath;
      if (reportEnabled) {
        console.info("[ScreenshotPage] WGC selected-output acceptance report", response);
        invoke("log_screenshot_perf", {
          message: `[wgc-acceptance] ok=true file=${savedPath || ""} realClipboard=${response.realClipboardVerified === true} width=${(response.selectedFile as any)?.pngWidth || ""} height=${(response.selectedFile as any)?.pngHeight || ""}`,
        }).catch(() => {});
      }
      message.success({ content: reportEnabled ? `WGC 选区验收通过：${savedPath || "PNG 已生成"}` : "WGC 选区诊断通过", key, duration: 5 });
      return;
    }
    const reason = typeof response.error === "string" ? response.error : response.stage || "未通过";
    if (reportEnabled) {
      const selectedFile = response.selectedFile as { path?: unknown; ok?: unknown; error?: unknown } | null | undefined;
      invoke("log_screenshot_perf", {
        message: `[wgc-acceptance] ok=false reason=${reason} file=${savePath || ""} selectedFileOk=${selectedFile?.ok === true} selectedFileError=${typeof selectedFile?.error === "string" ? selectedFile.error : ""}`,
      }).catch(() => {});
    }
    message.warning({ content: `WGC 选区诊断：${reason}`, key, duration: 4 });
  };
  // Set the Ref to complete the circle
  captureRegionBase64Ref.current = captureRegionBase64;

  // F1+F2: PixPin-style magnifier + HEX color picker
  const { getPixelHex, drawMagnifier } = useScreenshotMagnifier(imageRef as any, canvasRef);

  const updateMagnifier = (clientX: number, clientY: number) => {
    if (
      configRef.current.enableMagnifier === false ||
      !frameInteractiveRef.current ||
      !imageReadyRef.current ||
      hasSelectedRef.current ||
      isEditingRef.current
    ) {
      if (magnifierVisibleRef.current) setMagnifier(null);
      magnifierHexRef.current = "";
      return;
    }
    const cv = canvasRef.current;
    if (!cv) {
      if (magnifierVisibleRef.current) setMagnifier(null);
      return;
    }
    const r = cv.getBoundingClientRect();
    const cx = clientX - r.left;
    const cy = clientY - r.top;
    const info = getPixelHex(cx, cy);
    if (!info) {
      if (magnifierVisibleRef.current) setMagnifier(null);
      magnifierHexRef.current = "";
      return;
    }
    magnifierHexRef.current = info.hex;
    setMagnifier({ clientX, clientY, cx, cy, hex: info.hex });
  };

  useEffect(() => {
    magnifierVisibleRef.current = !!magnifier;
    if (magnifier && magnifierCanvasRef.current) {
      drawMagnifier(magnifierCanvasRef.current, magnifier.cx, magnifier.cy, 8, 120);
    }
  }, [magnifier, drawMagnifier]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!overlayVisibleRef.current || !frameInteractiveRef.current) return;
      if ((e.key === "c" || e.key === "C") && magnifierVisibleRef.current && magnifierHexRef.current) {
        e.preventDefault();
        e.stopPropagation();
        const hex = magnifierHexRef.current;
        navigator.clipboard?.writeText(hex).then(() => message.success("已复制颜色 " + hex)).catch(() => {});
        return;
      }
      if (!hasSelectedRef.current || isEditingRef.current) return;
      // 精确选区输入框开关：关闭时禁用方向键微调选区。
      if (configRef.current.enablePreciseSelection === false) return;
      const step = e.shiftKey ? 10 : 1;
      let dx = 0;
      let dy = 0;
      if (e.key === "ArrowLeft") dx = -step;
      else if (e.key === "ArrowRight") dx = step;
      else if (e.key === "ArrowUp") dy = -step;
      else if (e.key === "ArrowDown") dy = step;
      else return;
      e.preventDefault();
      const cur = rectRef.current;
      const cv = canvasRef.current;
      const maxW = cv ? cv.width : Number.MAX_SAFE_INTEGER;
      const maxH = cv ? cv.height : Number.MAX_SAFE_INTEGER;
      let next: Rect;
      if (e.altKey) {
        next = { x: cur.x, y: cur.y, w: Math.max(1, cur.w + dx), h: Math.max(1, cur.h + dy) };
      } else {
        const nx = Math.min(Math.max(0, cur.x + dx), Math.max(0, maxW - cur.w));
        const ny = Math.min(Math.max(0, cur.y + dy), Math.max(0, maxH - cur.h));
        next = { x: nx, y: ny, w: cur.w, h: cur.h };
      }
      setCurrentRect(next, true);
      drawRef.current(next.x, next.y, next.w, next.h);
      syncActionToolbarPosition(next);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const syncActionToolbarPosition = (nextRect: Rect) => {
    if (!actionToolbarRef.current || !overlayVisibleRef.current || !hasSelectedRef.current) return;
    liveToolbarRectRef.current = nextRect;
    if (liveToolbarFrameRef.current !== null) return;
    liveToolbarFrameRef.current = requestAnimationFrame(() => {
      liveToolbarFrameRef.current = null;
      const toolbar = actionToolbarRef.current;
      const liveRect = liveToolbarRectRef.current;
      if (!toolbar || !liveRect || !overlayVisibleRef.current || !hasSelectedRef.current) return;

      const bounds = toolbar.getBoundingClientRect();
      const measuredSize = {
        width: Math.ceil(bounds.width) || actionToolbarSizeRef.current.width,
        height: Math.ceil(bounds.height) || actionToolbarSizeRef.current.height,
      };
      const fallbackSize = recordingStatusRef.current !== "idle" || recordingPickerModeRef.current || scrollCaptureModeRef.current !== "idle"
        ? RECORDING_TOOLBAR_FALLBACK_SIZE
        : ACTION_TOOLBAR_FALLBACK_SIZE;
      const style = getActionToolbarStyle({
        rect: liveRect,
        toolbarSize: measuredSize,
        fallbackSize,
        viewportWidth: window.innerWidth,
        viewportHeight: window.innerHeight,
        margin: FLOATING_PANEL_MARGIN,
        gap: FLOATING_PANEL_GAP,
      });
      const nextTop = typeof style.top === "number" ? style.top : Number.parseFloat(String(style.top));
      const nextLeft = typeof style.left === "number" ? style.left : Number.parseFloat(String(style.left));
      if (Number.isFinite(nextTop)) toolbar.style.top = `${nextTop}px`;
      if (Number.isFinite(nextLeft)) toolbar.style.left = `${nextLeft}px`;
    });
  };

  // 9. useScreenshotInteraction
  const {
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
    handlePointerCancel,
    handleDoubleClick,
    focusScreenshotWindow,
    resetInteractionState,
    annotationStateRef,
  } = useScreenshotInteraction({
    canvasRef,
    mouseTrackerRef,
    rectRef,
    rect,
    setCurrentRect,
    setSelection,
    hasSelected,
    hasSelectedRef,
    overlayVisibleRef,
    frameInteractiveRef,
    imageReadyRef,
    activeSessionIdRef: displayedSessionIdRef,
    displayedPhysicalBoundsRef,
    isSelecting,
    setIsSelecting,
    isSelectingRef,
    isEditing,
    setIsEditing,
    isEditingRef,
    screenshotMode,
    screenshotModeRef,
    configRef,

    editingTextDraftRef,
    setEditingTextDraft,
    commitTextDraft,
    cancelTextDraft,
    commitAnnotation,
    setAnnotationDraft,
    draftAnnotationRef,
    annotationsRef,
    setAnnotations,
    annotationHistory,
    redoAnnotations,
    selectedAnnotationIndexRef,
    setSelectedAnnotationIndex,
    pushAnnotationHistory,
    undoAnnotation,
    redoAnnotation,
    deleteSelectedAnnotation,
    selectAnnotationTool: (tool) => {
      const toolSize = annotationSizesRef.current[tool] ?? DEFAULT_ANNOTATION_SIZES[tool];
      annotationToolRef.current = tool;
      annotationSizeRef.current = toolSize;
      setAnnotationTool(tool);
      setAnnotationSizeState(toolSize);
    },
    annotationToolRef,
    annotationColorRef,
    annotationSizeRef,
    annotationSizesRef,
    markerShapeRef,
    selectMoveTool: () => {
      setIsEditing(false);
      setAnnotationTool(null);
      selectedAnnotationIndexRef.current = null;
      setSelectedAnnotationIndex(null);
      setEditingTextDraft(null);
      setAnnotationDraft(null);
      triggerRender();
    },

    windowRectsRef,
    hoverRectRef,
    hoverCandidatesRef,
    hoverCandidateIndexRef,
    setHoverCandidate,
    setHoverCandidateList,
    loadWindowRects,
    clearWindowRects,

    recordingStatus,
    recordingStatusRef,
    recordingPickerModeRef,
    scrollCaptureModeRef,
    startManualScrollCapture,
    finishManualScrollCapture,
    cancelManualScrollCapture,
    enterRecordingMode,
    cancelRecordingTargetPicker,
    cancelRecording,
    finishRecording,

    handleOCR,
    handleTranslate,
    confirmScreenshot,
    cancelScreenshot,
    handlePin,
    forceCloseScreenshots,
    runWgcExplicitSelectionDiagnostic: handleWgcExplicitSelectionDiagnostic,
    lastMouseRef,

    selectionStartedAtRef,
    selectionCompletedAtRef,
    selectionDragDistanceRef,
    isOCRingRef,
    isTranslatingRef,
    isScrollCapturingRef,
    analysisImageDataRef,
    pendingConfirmTimerRef,

    draw: (...args) => drawRef.current(...args),
    syncToolbarPosition: syncActionToolbarPosition,
  });

  isOCRingRef.current = isOCRing;
  isTranslatingRef.current = isTranslating;

  const selectAnnotationTool = (tool: AnnotationTool) => {
    const toolSize = annotationSizesRef.current[tool] ?? DEFAULT_ANNOTATION_SIZES[tool];
    annotationToolRef.current = tool;
    annotationSizeRef.current = toolSize;
    setAnnotationTool(tool);
    setAnnotationSizeState(toolSize);
  };

  const setCurrentAnnotationSize = (size: number) => {
    const safeSize = Math.max(1, Number(size) || 1);
    annotationSizesRef.current = { ...annotationSizesRef.current, [annotationToolRef.current || DEFAULT_ANNOTATION_TOOL]: safeSize };
    annotationSizeRef.current = safeSize;
    setAnnotationSizeState(safeSize);
  };

  const selectMoveTool = () => {
    setIsEditing(false);
    setAnnotationTool(null);
    selectedAnnotationIndexRef.current = null;
    setSelectedAnnotationIndex(null);
    setEditingTextDraft(null);
    setAnnotationDraft(null);
    triggerRender();
  };

  function draw(rx: number, ry: number, rw: number, rh: number, translatedImg?: HTMLImageElement | HTMLCanvasElement | null) {
    renderScreenshotCanvas({
      canvas: canvasRef.current,
      image: imageRef.current as any,
      maskedCanvas: maskedCanvasRef.current,
      hoverRect: hoverRectRef.current,
      hoverCandidatesCount: hoverCandidatesRef.current.length,
      hoverCandidateIndex: hoverCandidateIndexRef.current,
      hasSelected: hasSelectedRef.current,
      selection: { x: rx, y: ry, w: rw, h: rh },
      translatedImg: translatedImgRef.current as any,
      overrideTranslatedImg: translatedImg as any,
      annotations: annotationsRef.current,
      draftAnnotation: draftAnnotationRef.current,
      selectedAnnotationIndex: selectedAnnotationIndexRef.current,
      detectionBorderWidth: configRef.current.detectionBorderWidth || 2,
      selectionBorderColor: recordingStatusRef.current === "recording" ? RECORDING_BORDER_COLOR : recordingStatusRef.current === "ready" ? RECORDING_BORDER_BLUE : scrollCaptureModeRef.current !== "idle" ? SCROLL_CAPTURE_BORDER_COLOR : undefined,
      selectionLabelColor: recordingStatusRef.current === "recording" ? "rgba(239, 68, 68, 0.9)" : recordingStatusRef.current === "ready" ? "rgba(37, 99, 235, 0.9)" : scrollCaptureModeRef.current !== "idle" ? "rgba(249, 115, 22, 0.9)" : undefined,
      selectionOnly: recordingStatusRef.current !== "idle",
    });
  }

  useEffect(() => {
    invoke("log_screenshot_perf", { message: "[baseline] session=screenshot-page phase=mounted elapsed_ms=0" }).catch(() => {});
    loadConfig();
    document.body.style.setProperty("margin", "0", "important");
    document.body.style.setProperty("overflow", "hidden", "important");
    document.body.style.setProperty("background", "transparent", "important");
    document.documentElement.style.setProperty("background", "transparent", "important");
    window.setTimeout(() => loadWindowRects(), 120);

    let unlistenMode: (() => void) | null = null;
    let unlistenShell: (() => void) | null = null;
    let unlistenEvent: (() => void) | null = null;
    let unlistenSessionCancelled: (() => void) | null = null;
    let unlistenRecordingEnded: (() => void) | null = null;

    const getPayloadSignature = (payload: ScreenshotUpdatedPayload | null | undefined) => {
      if (!payload || typeof payload === "string") return null;
      if (!payload.sessionId) return null;
      return [
        payload.sessionId,
        payload.kind || "object",
        payload.width || 0,
        payload.height || 0,
        payload.bytes || 0,
        payload.path || "",
        payload.base64 ? payload.base64.length : 0,
      ].join("|");
    };

    const recoverShellPreCaptureDrag = async (sessionId: string, shellReceivedAt: number) => {
      let loggedStart = false;
      const wait = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));
      const clampToViewport = (value: number, max: number) => Math.max(0, Math.min(max, value));
      const localSelectionStarted = () => (
        isSelectingRef.current
        || hasSelectedRef.current
        || rectRef.current.w > 0
        || rectRef.current.h > 0
      );
      const applyPreCapture = (preCapture: any, finalize: boolean) => {
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
        const dragDistance = Number(preCapture.dragDistance) || Math.hypot(next.w, next.h);
        selectionDragDistanceRef.current = Math.max(selectionDragDistanceRef.current, dragDistance);
        setCurrentRect(next, true);
        drawRef.current(next.x, next.y, next.w, next.h);
        const valid = next.w > 5 && next.h > 5;
        isSelectingRef.current = !finalize;
        setIsSelecting(!finalize);
        setSelection(finalize && valid);
        return { next, valid, dragDistance };
      };

      for (let index = 0; index < 48; index += 1) {
        if (displayedSessionIdRef.current !== sessionId || !overlayVisibleRef.current) return;
        if (!loggedStart && localSelectionStarted()) {
          invoke("log_screenshot_perf", {
            message: `[baseline] session=${sessionId} phase=shell_pre_capture_skipped_local_selection elapsed_ms=${Math.round(performance.now() - shellReceivedAt)} selecting=${isSelectingRef.current} rect=${Math.round(rectRef.current.w)}x${Math.round(rectRef.current.h)} selected=${hasSelectedRef.current}`,
          }).catch(() => {});
          return;
        }
        const pointerState = await invoke<any>("get_screenshot_pointer_state", {
          label: getCurrentWindow().label,
          sessionId,
        }).catch(() => null);
        const nativeOverlay = pointerState?.nativeOverlay;
        if (nativeOverlay?.cancelled === true && String(nativeOverlay.sessionId) === sessionId) {
          invoke("log_screenshot_perf", {
            message: `[baseline] session=${sessionId} phase=shell_native_overlay_cancel_received elapsed_ms=${Math.round(performance.now() - shellReceivedAt)} phase_value=${String(nativeOverlay.phase || "cancelled")} event_seq=${Number(nativeOverlay.eventSeq) || 0}`,
          }).catch(() => {});
          cancelScreenshot("native-overlay-cancelled");
          return;
        }
        const preCapture = pointerState?.nativeOverlay?.available === true ? pointerState.nativeOverlay : pointerState?.preCapture;
        if (!preCapture || preCapture.available !== true || String(preCapture.sessionId) !== sessionId) return;
        const source = String(preCapture.source || "pre-capture");
        const phase = String(preCapture.phase || "unknown");
        const eventSeq = Number(preCapture.eventSeq) || 0;
        const dragDistance = Number(preCapture.dragDistance) || 0;
        const leftDown = preCapture.leftDown === true;
        const completed = preCapture.completed === true;
        if (dragDistance < 3 && leftDown) {
          await wait(12);
          continue;
        }
        if (dragDistance < 3 && !completed) return;
        const { next, valid } = applyPreCapture(preCapture, !leftDown);
        if (!loggedStart) {
          loggedStart = true;
          invoke("log_screenshot_perf", {
            message: `[baseline] session=${sessionId} phase=shell_pre_capture_drag_recovered elapsed_ms=${Math.round(performance.now() - shellReceivedAt)} source=${source} phase_value=${phase} event_seq=${eventSeq} left_down=${leftDown} completed=${completed} valid=${valid} drag=${Math.round(dragDistance)} rect=${Math.round(next.x)},${Math.round(next.y)},${Math.round(next.w)},${Math.round(next.h)}`,
          }).catch(() => {});
        }
        if (!leftDown) {
          invoke("log_screenshot_perf", {
            message: `[baseline] session=${sessionId} phase=shell_pre_capture_drag_finalized elapsed_ms=${Math.round(performance.now() - shellReceivedAt)} source=${source} phase_value=${phase} event_seq=${eventSeq} valid=${valid} drag=${Math.round(dragDistance)}`,
          }).catch(() => {});
          return;
        }
        await wait(12);
      }
    };

    const handleScreenshotPayload = (payload: ScreenshotUpdatedPayload | null | undefined, source: string) => {
      const signature = getPayloadSignature(payload);
      if (signature && signature === lastScreenshotPayloadSignatureRef.current) {
        invoke("log_screenshot_perf", { message: `[baseline] session=${(payload as any)?.sessionId || "unknown"} phase=payload_duplicate_skipped elapsed_ms=0 source=${source}` }).catch(() => {});
        return;
      }
      if (signature) lastScreenshotPayloadSignatureRef.current = signature;
      primeTextSourceSnapshot(source, 160);
      if (typeof payload === "string") {
        if (payload) loadFullscreenFromBase64(payload, screenshotModeRef.current || "normal");
        else loadFullscreen();
        return;
      }
      if (payload?.kind === "file" && payload.path) {
        loadFullscreenFromFile(payload.path, payload.bytes, payload.mode || screenshotModeRef.current || "normal", payload.sessionId, payload.physicalBounds);
        return;
      }
      if (payload?.kind === "rgba" && payload.width && payload.height) {
        loadFullscreenFromRgba(payload.width, payload.height, payload.mode || screenshotModeRef.current || "normal", payload.sessionId, payload.bytes, payload.physicalBounds);
        return;
      }
      if (payload?.kind === "memory") {
        loadFullscreen(payload.mode || screenshotModeRef.current || "normal", payload.sessionId, payload.bytes, false, payload.physicalBounds);
        return;
      }
      if (payload?.base64) {
        loadFullscreenFromBase64(payload.base64, payload.mode || screenshotModeRef.current || "normal", payload.sessionId, payload.physicalBounds);
        return;
      }
      loadFullscreen();
    };

    listen<string>("screenshot-mode", (event) => {
      const nextMode = event.payload || "normal";
      setScreenshotMode(nextMode);
      if (nextMode === "translate") {
        prewarmLocalOcrWorker("translate-hotkey");
        prewarmTranslationServices(configRef.current, { reason: "translate-hotkey" })
          .catch((error) => console.warn("[Translation Service Prewarm] failed", error));
      }
      if (nextMode === "record") {
        setRecordingMode("region");
      }
    })
      .then((unsub) => { unlistenMode = unsub; })
      .catch(() => {});

    listen("recording-ended", () => {
      invoke("cancel_screenshot", { label: getCurrentWindow().label })
        .finally(() => {
          clearRecordingState();
          resetScreenshotState();
        });
    })
      .then((unsub) => { unlistenRecordingEnded = unsub; })
      .catch(() => {});

    const handleScreenshotShellPayload = (rawPayload: any, source = "screenshot-shell") => {
      const payload = rawPayload || {};
      const nextMode = payload.mode || screenshotModeRef.current || "normal";
      const sessionId = String(payload.sessionId || "shell");
      const shellSignature = JSON.stringify({
        sessionId,
        mode: nextMode,
        nativeVisible: payload.nativeVisible === true,
        showOnShellReady: payload.showOnShellReady === true,
        screen: payload.screen || null,
      });
      if (lastScreenshotShellSignatureRef.current === shellSignature) {
        invoke("log_screenshot_perf", { message: `[baseline] session=${sessionId} phase=shell_payload_duplicate_skipped elapsed_ms=0 source=${source}` }).catch(() => {});
        return;
      }
      if (imageRef.current && displayedSessionIdRef.current === sessionId) {
        lastScreenshotShellSignatureRef.current = shellSignature;
        invoke("log_screenshot_perf", { message: `[baseline] session=${sessionId} phase=shell_payload_skipped_image_ready elapsed_ms=0 source=${source}` }).catch(() => {});
        return;
      }
      lastScreenshotShellSignatureRef.current = shellSignature;
      const shellReceivedAt = performance.now();
      const nativeVisible = payload.nativeVisible === true;
      const showOnShellReady = payload.showOnShellReady === true;
      const shouldPresentShell = nativeVisible || showOnShellReady;
      screenshotModeRef.current = nextMode;
      nativeOverlayVisibleRef.current = shouldPresentShell;
      displayedSessionIdRef.current = sessionId;
      displayedPhysicalBoundsRef.current = payload.screen ? {
        x: Number(payload.screen.x) || 0,
        y: Number(payload.screen.y) || 0,
        width: Number(payload.screen.width) || 0,
        height: Number(payload.screen.height) || 0,
      } : null;
      setScreenshotMode(nextMode);
      autoAcceptanceSmokeStartedRef.current = false;
      resetOcrState();
      resetInteractionState();
      setCurrentRect(EMPTY_RECT, true);
      setSelection(false);
      hoverRectRef.current = null;
      hoverCandidatesRef.current = [];
      hoverCandidateIndexRef.current = 0;
      imageRef.current = null;
      maskedCanvasRef.current = null;
      analysisImageDataRef.current = null;
      const shellCanvas = canvasRef.current;
      const shellCtx = shellCanvas?.getContext("2d");
      if (shellCanvas && shellCtx) {
        const { width, height } = getLogicalCanvasSize(displayedPhysicalBoundsRef.current);
        shellCanvas.width = width;
        shellCanvas.height = height;
        shellCanvas.style.width = `${width}px`;
        shellCanvas.style.height = `${height}px`;
        shellCtx.clearRect(0, 0, shellCanvas.width, shellCanvas.height);
      }
      setHoverCandidate(null);
      setHoverCandidateList([]);
      overlayVisibleRef.current = shouldPresentShell;
      setOverlayVisible(shouldPresentShell);
      setScreenshotState("initializing");
      setDbgStatus({ imageLoaded: false, imageWidth: payload.screen?.width || 0, imageHeight: payload.screen?.height || 0, screenshotBytes: 0, errorMsg: "" });
      invoke("log_screenshot_perf", { message: `[baseline] session=${sessionId} phase=shell_event_received elapsed_ms=0 source=${source} mode=${nextMode} native_visible=${nativeVisible} show_on_shell_ready=${showOnShellReady}` }).catch(() => {});
      if (showOnShellReady && !nativeVisible) {
        void (async () => {
          try {
            await waitForShellPaint();
            if (displayedSessionIdRef.current !== sessionId) return;
            const calledAt = Math.round(performance.now() - shellReceivedAt);
            invoke("log_screenshot_perf", { message: `[baseline] session=${sessionId} phase=shell_ready_to_show_called elapsed_ms=${calledAt}` }).catch(() => {});
            await invoke("overlay_ready_to_show", { label: getCurrentWindow().label, sessionId });
            if (displayedSessionIdRef.current !== sessionId) return;
            nativeOverlayVisibleRef.current = true;
            const returnedAt = Math.round(performance.now() - shellReceivedAt);
            invoke("log_screenshot_perf", { message: `[baseline] session=${sessionId} phase=shell_ready_to_show_returned elapsed_ms=${returnedAt}` }).catch(() => {});
            const shellCanvas = canvasRef.current;
            if (shellCanvas) {
              shellCanvas.focus({ preventScroll: true });
              requestAnimationFrame(() => shellCanvas.focus({ preventScroll: true }));
            }
            void recoverShellPreCaptureDrag(sessionId, shellReceivedAt);
          } catch (error: any) {
            if (displayedSessionIdRef.current !== sessionId) return;
            nativeOverlayVisibleRef.current = false;
            cancelScreenshot(error?.message || "Screenshot shell failed to show");
          }
        })();
      }
    };

    listen<any>("screenshot-shell", (event) => {
      handleScreenshotShellPayload(event.payload, "screenshot-shell");
    })
      .then((unsub) => {
        unlistenShell = unsub;
        invoke<any | null>("get_latest_screenshot_shell_payload")
          .then((payload) => {
            if (payload) handleScreenshotShellPayload(payload, "screenshot-shell-pending-payload");
          })
          .catch(() => {});
      })
      .catch(() => {});

    listen<any>("screenshot-session-cancelled", (event) => {
      const payload = event.payload || {};
      const cancelledSessionId = payload.sessionId == null ? "" : String(payload.sessionId);
      if (cancelledSessionId && displayedSessionIdRef.current && displayedSessionIdRef.current !== cancelledSessionId) {
        invoke("log_screenshot_perf", { message: `[baseline] session=${cancelledSessionId} phase=session_cancelled_skipped elapsed_ms=0 reason=${payload.reason || "unknown"} displayed=${displayedSessionIdRef.current}` }).catch(() => {});
        return;
      }
      invoke("log_screenshot_perf", { message: `[baseline] session=${cancelledSessionId || displayedSessionIdRef.current || "unknown"} phase=session_cancelled_received elapsed_ms=0 reason=${payload.reason || "unknown"}` }).catch(() => {});
      lastScreenshotPayloadSignatureRef.current = null;
      lastScreenshotShellSignatureRef.current = null;
      resetScreenshotState();
    })
      .then((unsub) => {
        unlistenSessionCancelled = unsub;
      })
      .catch(() => {});

    listen<ScreenshotUpdatedPayload>("screenshot-updated", (event) => handleScreenshotPayload(event.payload, "screenshot-updated"))
      .then((unsub) => {
        unlistenEvent = unsub;
        invoke<ScreenshotUpdatedPayload | null>("get_latest_screenshot_payload")
          .then((payload) => {
            if (payload) handleScreenshotPayload(payload, "screenshot-pending-payload");
          })
          .catch(() => {});
      })
      .catch(() => {});

    return () => {
      if (unlistenShell) unlistenShell();
      if (unlistenEvent) unlistenEvent();
      if (unlistenSessionCancelled) unlistenSessionCancelled();
      if (unlistenMode) unlistenMode();
      if (unlistenRecordingEnded) unlistenRecordingEnded();
      if (requestRef.current) cancelAnimationFrame(requestRef.current);
      requestRef.current = null;
      renderFramePendingRef.current = false;
      if (liveToolbarFrameRef.current !== null) cancelAnimationFrame(liveToolbarFrameRef.current);
    };
  }, []);

  useEffect(() => {
    if (!WGC_SELECTED_OUTPUT_AUTO_ACCEPTANCE_SMOKE_ENABLED) return;
    if (autoAcceptanceSmokeStartedRef.current) return;
    if (!overlayVisible || screenshotState !== "ready" || !imageRef.current || !displayedPhysicalBoundsRef.current) return;
    autoAcceptanceSmokeStartedRef.current = true;
    window.setTimeout(() => {
      handleWgcExplicitSelectionDiagnostic().catch((error) => {
        autoAcceptanceSmokeStartedRef.current = false;
        console.warn("[ScreenshotPage] WGC selected-output auto acceptance smoke failed", error);
      });
    }, 120);
  }, [overlayVisible, screenshotState]);

  useEffect(() => {
    const toolbar = actionToolbarRef.current;
    if (!toolbar || !hasSelected) return;

    const updateToolbarSize = () => {
      const bounds = toolbar.getBoundingClientRect();
      const next = {
        width: Math.ceil(bounds.width) || ACTION_TOOLBAR_FALLBACK_SIZE.width,
        height: Math.ceil(bounds.height) || ACTION_TOOLBAR_FALLBACK_SIZE.height,
      };
      setActionToolbarSize((current) => (
        Math.abs(current.width - next.width) > 2 || Math.abs(current.height - next.height) > 2 ? next : current
      ));
    };

    updateToolbarSize();

    const observer = new ResizeObserver(updateToolbarSize);
    observer.observe(toolbar);
    return () => observer.disconnect();
  }, [hasSelected, recordingStatus, recordingPickerMode, scrollCaptureMode, isEditing, annotationTool, recordingMode]);

  const currentToolbarStyle = getActionToolbarStyle({ 
    rect, 
    toolbarSize: actionToolbarSize, 
    fallbackSize: recordingStatus !== "idle" || recordingPickerMode || scrollCaptureMode !== "idle" ? RECORDING_TOOLBAR_FALLBACK_SIZE : ACTION_TOOLBAR_FALLBACK_SIZE, 
    viewportWidth: window.innerWidth, 
    viewportHeight: window.innerHeight, 
    margin: FLOATING_PANEL_MARGIN, 
    gap: FLOATING_PANEL_GAP 
  });
  const currentOverlayToolbarStyle: React.CSSProperties = { ...currentToolbarStyle, padding: 0, border: "none", boxShadow: "none", background: "transparent" };
  const currentRecordingDevices = getRecordingDevices();
  const audioOptions = [
    { label: "Silent", value: "none" },
    { label: currentRecordingDevices.mic ? `Mic: ${formatAudioDeviceLabel(currentRecordingDevices.mic)}` : "Mic (not detected)", value: "mic", disabled: !currentRecordingDevices.mic },
    { label: currentRecordingDevices.system ? formatAudioDeviceLabel(currentRecordingDevices.system) : "System audio (not detected)", value: "system", disabled: !currentRecordingDevices.system },
    { label: "System audio + Mic", value: "system_mic", disabled: !currentRecordingDevices.system || !currentRecordingDevices.mic },
  ];

  return (
    <div className={`screenshot-root ${overlayVisible && screenshotState === "ready" ? "ready" : overlayVisible ? "shell" : "initializing"}`} style={{ position: "fixed", inset: 0, overflow: "hidden", cursor: canInteractWithOverlay ? (hasSelected ? "default" : "crosshair") : "wait" }}>
      {overlayVisible && !hasSelected && (
        <div ref={mouseTrackerRef} style={{ position: "absolute", top: -100, left: -100, zIndex: 9999, background: "rgba(0, 0, 0, 0.75)", color: "#fff", padding: "2px 8px", borderRadius: "4px", fontSize: "11px", fontFamily: "Consolas, Monaco, monospace", pointerEvents: "none", whiteSpace: "nowrap", lineHeight: "18px", display: "none" }}>0, 0</div>
      )}

      {magnifier && (
        <div
          style={{ position: "fixed", left: magnifier.clientX + 16, top: magnifier.clientY + 16, zIndex: 2147483647, pointerEvents: "none", background: "rgba(255,255,255,0.96)", borderRadius: 6, padding: 4, boxShadow: "0 2px 10px rgba(0,0,0,0.3)" }}
        >
          <canvas
            ref={magnifierCanvasRef}
            width={120}
            height={120}
            style={{ display: "block", width: 120, height: 120, imageRendering: "pixelated" }}
          />
          <div style={{ marginTop: 2, fontSize: 12, fontFamily: "monospace", textAlign: "center", color: "#222" }}>
            {magnifier.hex} · 按 C 复制
          </div>
        </div>
      )}

      {isTranslating && <TranslationLoadingOverlay rect={rect} />}

      {editingTextDraft && (
        <TextAnnotationEditor
          draft={editingTextDraft}
          onChange={(value) => setEditingTextDraft((draft) => draft ? { ...draft, value } : draft)}
          onCommit={commitTextDraft}
          onCancel={cancelTextDraft}
        />
      )}

      <canvas
        ref={canvasRef}
        tabIndex={-1}
        onPointerDown={handleMouseDown}
        onPointerMove={(e) => { handleMouseMove(e); updateMagnifier(e.clientX, e.clientY); }}
        onPointerUp={(e) => { handleMouseUp(e); setMagnifier(null); }}
        onPointerCancel={handlePointerCancel}
        onPointerLeave={() => { if (magnifierVisibleRef.current) setMagnifier(null); }}
        onDoubleClick={handleDoubleClick}
        style={{ position: "absolute", top: 0, left: 0, zIndex: 10, cursor: "crosshair", outline: "none", touchAction: "none", pointerEvents: overlayVisible ? "auto" : "none" }}
      />

      {overlayVisible && screenshotState === "ready" && hasSelected && !isSelecting && recordingStatus === "idle" && recordingPickerMode && (
        <div ref={actionToolbarRef} style={currentOverlayToolbarStyle} onContextMenu={(event) => event.stopPropagation()}>
          <RecordingTargetPicker
            mode={recordingPickerMode}
            targets={recordingPickerMode === "window" ? recordingTargets.windows : recordingTargets.displays}
            selectedTargetId={recordingPickerMode === "window" ? selectedWindowTargetId : selectedDisplayTargetId}
            busy={isRecordingBusy}
            onSelect={(targetId) => {
              if (recordingPickerMode) selectRecordingTarget(recordingPickerMode, targetId);
            }}
            onConfirm={confirmRecordingTargetPicker}
            onCancel={cancelRecordingTargetPicker}
          />
        </div>
      )}

      {overlayVisible && scrollCaptureMode === "capturing" && scrollPreviewBase64 && (
        <div style={{ position: "absolute", top: Math.max(12, rect.y), left: Math.min(window.innerWidth - 190, rect.x + rect.w + 12), zIndex: 19, width: 176, maxHeight: Math.min(420, window.innerHeight - 24), borderRadius: 12, overflow: "hidden", border: "1px solid rgba(226,232,240,0.95)", background: "rgba(255,255,255,0.96)", boxShadow: "0 16px 42px rgba(15,23,42,0.18)" }}>
          <div style={{ padding: "6px 8px", fontSize: 12, fontWeight: 800, color: "#0f172a", borderBottom: "1px solid #e2e8f0" }}>Scroll Preview</div>
          <img src={`data:image/png;base64,${scrollPreviewBase64}`} alt="" style={{ display: "block", width: "100%", height: "auto", maxHeight: 380, objectFit: "contain", background: "#fff" }} />
        </div>
      )}

      {overlayVisible && screenshotState === "ready" && hasSelected && !isSelecting && recordingStatus === "idle" && !recordingPickerMode && scrollCaptureMode !== "idle" && (
        <div ref={actionToolbarRef} style={currentOverlayToolbarStyle} onContextMenu={(event) => event.stopPropagation()}>
          <Space size={[8, 8]} wrap style={{ maxWidth: "100%", padding: "8px 10px", borderRadius: 16, background: "rgba(255,255,255,0.96)", border: "1px solid rgba(226,232,240,0.95)", boxShadow: "0 12px 32px rgba(15,23,42,0.18)", color: "#111827", boxSizing: "border-box" }}>
            <span style={{ color: SCROLL_CAPTURE_BORDER_COLOR, fontWeight: 800 }}>Manual Scroll Capture</span>
            <span style={{ fontSize: 12, color: "#475569" }}>Click start, scroll the target window, then finish to stitch and copy.</span>
            {scrollCaptureMode === "ready" && <Button size="small" type="primary" onClick={startManualScrollCapture}>Start</Button>}
            {scrollCaptureMode === "capturing" && <Button size="small" type="primary" onClick={finishManualScrollCapture}>Finish</Button>}
            <Button size="small" onClick={cancelManualScrollCapture}>Cancel</Button>
          </Space>
        </div>
      )}

      {overlayVisible && screenshotState === "ready" && hasSelected && !isSelecting && recordingStatus === "idle" && !recordingPickerMode && scrollCaptureMode === "idle" && (
        <ScreenshotToolbar
          containerRef={actionToolbarRef}
          style={currentToolbarStyle}
          annotationTool={annotationTool}
          annotationColor={annotationColor}
          annotationSize={annotationSize}
          isEditing={isEditing}
          isTranslating={isTranslating}
          isOCRing={isOCRing}
          isScrollCapturing={isScrollCapturing}
          canUndo={annotationHistory.length > 0}
          canRedo={redoAnnotations.length > 0}
          onSetEditing={setIsEditing}
          onSelectMove={selectMoveTool}
          onSetAnnotationTool={selectAnnotationTool}
          onSetAnnotationColor={setAnnotationColor}
          onSetAnnotationSize={setCurrentAnnotationSize}
          markerShape={markerShape}
          onSetMarkerShape={setMarkerShape}
          onTranslate={handleTranslate}
          onShowTranslateResult={handleShowTranslateResult}
          canShowTranslateResult={Boolean(translatePairs && translatePairs.length > 0)}
          onOCR={handleOCR}
          onScrollCapture={() => handleScrollCapture(hasSelected)}
          onRecording={enterRecordingMode}
          onPin={handlePin}
          onUndo={undoAnnotation}
          onRedo={redoAnnotation}
          onSave={() => confirmScreenshot("save")}
          onCancel={cancelScreenshot}
          onCopy={() => confirmScreenshot("copy")}
          buttonGap={config.toolbarButtonGap ?? 6}
        />
      )}
    </div>
  );
}
