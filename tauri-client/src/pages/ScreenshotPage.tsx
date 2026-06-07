import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Space, Button } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ScreenshotToolbar from "../components/screenshot/ScreenshotToolbar";
import TextAnnotationEditor from "../components/screenshot/TextAnnotationEditor";
import TranslationLoadingOverlay from "../components/screenshot/TranslationLoadingOverlay";
import type { Config } from "../types/config";
import type { Annotation, AnnotationTool, Rect } from "../types/screenshot";
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
import { prewarmTranslationServices } from "../utils/localOcrTranslate";

const ACTION_TOOLBAR_FALLBACK_SIZE = { width: 680, height: 86 };
const RECORDING_TOOLBAR_FALLBACK_SIZE = { width: 980, height: 96 };
const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };
const RECORDING_BORDER_COLOR = "#ef4444";
const RECORDING_BORDER_BLUE = "#2563eb";
const RECORDING_BORDER_RED = "#ef4444";
const RECORDING_BORDER_YELLOW = "#f59e0b";
const SCROLL_CAPTURE_BORDER_COLOR = "#f97316";

type ScreenshotUpdatedPayload = string | {
  kind?: "file" | "base64" | "memory";
  path?: string;
  base64?: string;
  bytes?: number;
  mode?: string;
};

export default function ScreenshotPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const mouseTrackerRef = useRef<HTMLDivElement>(null);
  const actionToolbarRef = useRef<HTMLDivElement>(null);
  const actionToolbarSizeRef = useRef(ACTION_TOOLBAR_FALLBACK_SIZE);
  const liveToolbarFrameRef = useRef<number | null>(null);
  const liveToolbarRectRef = useRef<Rect | null>(null);
  const lastMouseRef = useRef({ x: 0, y: 0 });
  
  const [isSelecting, setIsSelecting] = useState(false);
  const [rect, setRect] = useState<Rect>(EMPTY_RECT);
  const [actionToolbarSize, setActionToolbarSize] = useState(ACTION_TOOLBAR_FALLBACK_SIZE);
  const [hasSelected, setHasSelected] = useState(false);
  const [screenshotMode, setScreenshotMode] = useState("normal");
  const [config, setConfig] = useState<Config>({});
  const [isEditing, setIsEditing] = useState(false);

  const renderNeededRef = useRef(false);
  const requestRef = useRef<number | null>(null);
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
  const drawRef = useRef(draw);

  // Break circular dependency
  const captureRegionBase64Ref = useRef<() => Promise<string>>(() => Promise.resolve(""));

  hasSelectedRef.current = hasSelected;
  rectRef.current = rect;
  configRef.current = config;
  isEditingRef.current = isEditing;
  isSelectingRef.current = isSelecting;
  screenshotModeRef.current = screenshotMode;
  drawRef.current = draw;
  actionToolbarSizeRef.current = actionToolbarSize;

  const interactionStateRef = useRef({
    get hasSelected() { return hasSelectedRef.current; },
    get isSelecting() { return isSelectingRef.current; },
  });

  const triggerRender = () => {
    renderNeededRef.current = true;
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
    cancelTextDraft, commitTextDraft, deleteSelectedAnnotation, resetAnnotations
  } = useScreenshotAnnotation(() => {
    renderNeededRef.current = true;
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
    loadConfig,
    loadFullscreen,
    loadFullscreenFromBase64,
    loadFullscreenFromFile,
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
    setCurrentRect,
    setSelection,
    setHasSelected,
    setTranslatedResult: (res) => setTranslatedResult(res),
    setTranslatePairs: (pairs) => setTranslatePairs(pairs),
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
    setTranslatedResult,
    setTranslatePairs,
  } = useScreenshotOcr({
    config,
    rectRef,
    captureRegionBase64: () => captureRegionBase64Ref.current(),
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
    getSelectionConfirmDelayMs,
    canConfirmCurrentSelection,
    handlePin,
    forceCloseScreenshots,
    confirmScreenshot,
  } = useScreenshotActions({
    canvasRef,
    imageRef,
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

  // Set the Ref to complete the circle
  captureRegionBase64Ref.current = captureRegionBase64;

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
    const tick = () => {
      if (renderNeededRef.current) {
        drawRef.current(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
        renderNeededRef.current = false;
      }
      requestRef.current = requestAnimationFrame(tick);
    };
    requestRef.current = requestAnimationFrame(tick);

    loadConfig();
    document.body.style.setProperty("margin", "0", "important");
    document.body.style.setProperty("overflow", "hidden", "important");
    document.body.style.setProperty("background", "transparent", "important");
    document.documentElement.style.setProperty("background", "transparent", "important");
    loadWindowRects();

    let unlistenMode: (() => void) | null = null;
    let unlistenEvent: (() => void) | null = null;
    let unlistenRecordingEnded: (() => void) | null = null;

    listen<string>("screenshot-mode", async (event) => {
      await loadConfig();
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
      clearRecordingState();
      resetScreenshotState();
      invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
    })
      .then((unsub) => { unlistenRecordingEnded = unsub; })
      .catch(() => {});

    listen<ScreenshotUpdatedPayload>("screenshot-updated", async (event) => {
      await loadConfig();
      primeTextSourceSnapshot("screenshot-updated", 160);
      const payload = event.payload;
      if (typeof payload === "string") {
        if (payload) loadFullscreenFromBase64(payload, screenshotModeRef.current || "normal");
        else loadFullscreen();
        return;
      }
      if (payload?.kind === "file" && payload.path) {
        loadFullscreenFromFile(payload.path, payload.bytes, payload.mode || screenshotModeRef.current || "normal");
        return;
      }
      if (payload?.kind === "memory") {
        loadFullscreen(payload.mode || screenshotModeRef.current || "normal");
        return;
      }
      if (payload?.base64) {
        loadFullscreenFromBase64(payload.base64, payload.mode || screenshotModeRef.current || "normal");
        return;
      }
      loadFullscreen();
    })
      .then((unsub) => { unlistenEvent = unsub; })
      .catch(() => {});

    return () => {
      if (unlistenEvent) unlistenEvent();
      if (unlistenMode) unlistenMode();
      if (unlistenRecordingEnded) unlistenRecordingEnded();
      if (requestRef.current) cancelAnimationFrame(requestRef.current);
      if (liveToolbarFrameRef.current !== null) cancelAnimationFrame(liveToolbarFrameRef.current);
    };
  }, []);

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
    { label: "静音", value: "none" },
    { label: currentRecordingDevices.mic ? `麦克风：${formatAudioDeviceLabel(currentRecordingDevices.mic)}` : "麦克风（未检测到）", value: "mic", disabled: !currentRecordingDevices.mic },
    { label: currentRecordingDevices.system ? formatAudioDeviceLabel(currentRecordingDevices.system) : "系统声音（未检测到）", value: "system", disabled: !currentRecordingDevices.system },
    { label: "系统声音 + 麦克风", value: "system_mic", disabled: !currentRecordingDevices.system || !currentRecordingDevices.mic },
  ];

  return (
    <div className={`screenshot-root ${overlayVisible && screenshotState === "ready" ? "ready" : "initializing"}`} style={{ position: "fixed", inset: 0, overflow: "hidden", cursor: hasSelected ? "default" : "crosshair" }}>
      {overlayVisible && !hasSelected && (
        <div ref={mouseTrackerRef} style={{ position: "absolute", top: -100, left: -100, zIndex: 9999, background: "rgba(0, 0, 0, 0.75)", color: "#fff", padding: "2px 8px", borderRadius: "4px", fontSize: "11px", fontFamily: "Consolas, Monaco, monospace", pointerEvents: "none", whiteSpace: "nowrap", lineHeight: "18px", display: "none" }}>0, 0</div>
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
        onPointerMove={handleMouseMove}
        onPointerUp={handleMouseUp}
        onPointerCancel={handlePointerCancel}
        onDoubleClick={handleDoubleClick}
        style={{ position: "absolute", top: 0, left: 0, zIndex: 10, cursor: "crosshair", outline: "none", touchAction: "none", pointerEvents: overlayVisible ? "auto" : "none" }}
      />

      {overlayVisible && hasSelected && !isSelecting && recordingStatus === "idle" && recordingPickerMode && (
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
          <div style={{ padding: "6px 8px", fontSize: 12, fontWeight: 800, color: "#0f172a", borderBottom: "1px solid #e2e8f0" }}>滚动预览</div>
          <img src={`data:image/png;base64,${scrollPreviewBase64}`} alt="" style={{ display: "block", width: "100%", height: "auto", maxHeight: 380, objectFit: "contain", background: "#fff" }} />
        </div>
      )}

      {overlayVisible && hasSelected && !isSelecting && recordingStatus === "idle" && !recordingPickerMode && scrollCaptureMode !== "idle" && (
        <div ref={actionToolbarRef} style={currentOverlayToolbarStyle} onContextMenu={(event) => event.stopPropagation()}>
          <Space size={[8, 8]} wrap style={{ maxWidth: "100%", padding: "8px 10px", borderRadius: 16, background: "rgba(255,255,255,0.96)", border: "1px solid rgba(226,232,240,0.95)", boxShadow: "0 12px 32px rgba(15,23,42,0.18)", color: "#111827", boxSizing: "border-box" }}>
            <span style={{ color: SCROLL_CAPTURE_BORDER_COLOR, fontWeight: 800 }}>手动滚动截图</span>
            <span style={{ fontSize: 12, color: "#475569" }}>点击开始后自己滚动目标窗口，完成后自动拼接并复制</span>
            {scrollCaptureMode === "ready" && <Button size="small" type="primary" onClick={startManualScrollCapture}>开始采集</Button>}
            {scrollCaptureMode === "capturing" && <Button size="small" type="primary" onClick={finishManualScrollCapture}>完成</Button>}
            <Button size="small" onClick={cancelManualScrollCapture}>取消</Button>
          </Space>
        </div>
      )}

      {overlayVisible && hasSelected && !isSelecting && recordingStatus === "idle" && !recordingPickerMode && scrollCaptureMode === "idle" && (
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
