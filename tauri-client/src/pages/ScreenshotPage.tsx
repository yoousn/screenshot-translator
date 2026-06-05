import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { Button, Space, message } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ScreenshotToolbar from "../components/screenshot/ScreenshotToolbar";
import TextAnnotationEditor from "../components/screenshot/TextAnnotationEditor";
import TranslationLoadingOverlay from "../components/screenshot/TranslationLoadingOverlay";
import type { Config } from "../types/config";
import type { Annotation, AnnotationTool, EditingTextDraft, Point, Rect } from "../types/screenshot";
import { useScreenshotOcr } from "../hooks/useScreenshotOcr";
import { useScreenshotAnnotation, DEFAULT_ANNOTATION_COLOR, DEFAULT_ANNOTATION_TOOL, DEFAULT_ANNOTATION_SIZES } from "../hooks/useScreenshotAnnotation";
import { clamp, hitAnnotationDetailed, isDraggableAnnotation, makeLineAnnotation, moveAnnotation, normalizedRectFromPoints, resizeAnnotation, type AnnotationResizeHandle } from "../utils/annotationGeometry";
import { cropSelectionFromLoadedImage, getPhysicalSelection, renderEditedSelectionBase64 } from "../utils/screenshotImage";
import { getActionToolbarStyle, FLOATING_PANEL_MARGIN, FLOATING_PANEL_GAP } from "../utils/screenshotLayout";
import { getHandleAt, isPointInSelection } from "../utils/selectionGeometry";
import { openPinWindow } from "../utils/pinWindows";
import { getDetectionCandidatesAt } from "../utils/detectionCandidates";
import { prewarmTranslationServices } from "../utils/localOcrTranslate";
import { renderScreenshotCanvas } from "../utils/renderScreenshotCanvas";
import RecordingTargetPicker from "../components/recording/RecordingTargetPicker";
import { useScreenshotWindowRects } from "../hooks/useScreenshotWindowRects";
import { useScreenshotRecording } from "../hooks/useScreenshotRecording";
import { useScrollCapture } from "../hooks/useScrollCapture";
import { useScreenshotLoader } from "../hooks/useScreenshotLoader";

const ACTION_TOOLBAR_FALLBACK_SIZE = { width: 680, height: 86 };
const RECORDING_TOOLBAR_FALLBACK_SIZE = { width: 980, height: 96 };
const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };
const RECORDING_BORDER_COLOR = "#ef4444";
const RECORDING_READY_BORDER_COLOR = "#2563eb";
const SCROLL_CAPTURE_BORDER_COLOR = "#f97316";
const MIN_AUTO_ACTION_DRAG_PX = 8;
const MIN_SELECTION_CONFIRM_AGE_MS = 120;

type ScreenshotUpdatedPayload = string | {
  kind?: "file" | "base64";
  path?: string;
  base64?: string;
  bytes?: number;
  mode?: string;
};

type TextSourceElement = {
  text?: string;
  x?: number;
  y?: number;
  w?: number;
  h?: number;
};

type TextSourceSnapshot = {
  status?: string;
  capturedAt?: string;
  screen?: { x?: number; y?: number; w?: number; h?: number };
  timings?: { totalMs?: number };
  elements?: TextSourceElement[];
};

export default function ScreenshotPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const mouseTrackerRef = useRef<HTMLDivElement>(null);
  const actionToolbarRef = useRef<HTMLDivElement>(null);
  const activePointerIdRef = useRef<number | null>(null);
  
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
  const textSourceSnapshotPromiseRef = useRef<Promise<TextSourceSnapshot | null> | null>(null);
  const lastMouseRef = useRef({ x: 0, y: 0 });
  const pendingDetectionRef = useRef<Rect | null>(null);
  const annotationDragSnapshotRef = useRef<Annotation[] | null>(null);
  const isEditingRef = useRef(false);
  const isDrawingAnnotationRef = useRef(false);
  const isDraggingAnnotationRef = useRef(false);
  const isResizingAnnotationRef = useRef(false);
  const annotationResizeHandleRef = useRef<AnnotationResizeHandle | null>(null);
  const annotationStartRef = useRef({ x: 0, y: 0 });
  const annotationDragStartRef = useRef({ x: 0, y: 0 });

  const hasSelectedRef = useRef(false);
  const rectRef = useRef<Rect>(EMPTY_RECT);
  const configRef = useRef<Config>({});
  const isSelectingRef = useRef(false);
  const isDraggingRef = useRef(false);
  const isResizingRef = useRef<string | null>(null);
  const mouseDownRef = useRef({ x: 0, y: 0 });
  const startPosRef = useRef({ x: 0, y: 0 });
  const dragStartRef = useRef({ x: 0, y: 0 });
  const resizeStartRectRef = useRef<Rect>(EMPTY_RECT);
  const screenshotModeRef = useRef("normal");
  const drawRef = useRef(draw);

  hasSelectedRef.current = hasSelected;
  rectRef.current = rect;
  configRef.current = config;
  isEditingRef.current = isEditing;
  isSelectingRef.current = isSelecting;
  screenshotModeRef.current = screenshotMode;
  drawRef.current = draw;

  const interactionStateRef = useRef({
    get hasSelected() { return hasSelectedRef.current; },
    get isSelecting() { return isSelectingRef.current; },
    get isDragging() { return isDraggingRef.current; },
    get isResizing() { return isResizingRef.current !== null; },
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

  // 1. useScreenshotAnnotation
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

  // 2. useScreenshotWindowRects
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
    analysisImageDataRef: { get current() { return analysisImageDataRef.current; } } as any, // dynamic access
    interactionStateRef,
    triggerRender,
  });

  // 3. useScreenshotRecording
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

  // 4. useScrollCapture
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

  // 5. useScreenshotLoader
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
    draw,
    textSourceSnapshotPromiseRef,
    pendingConfirmTimerRef,
  });

  // 6. useScreenshotOcr
  const {
    isOCRing,
    isTranslating,
    translatePairs,
    translatedResult,
    prewarmLocalOcrWorker,
    handleOCR,
    handleTranslate,
    handleShowTranslateResult,
    isOCRingRef,
    isTranslatingRef,
    setTranslatedResult,
    setTranslatePairs,
  } = useScreenshotOcr({
    config,
    rectRef,
    captureRegionBase64: () => captureRegionBase64(),
    resetScreenshotState: () => resetScreenshotState(),
    draw,
    translatedImgRef,
    getTextSourceBlocksForCurrentSelection: (...args: any[]) => getTextSourceBlocksForCurrentSelection(...args),
  });

  const handleTranslateRef = useRef(handleTranslate);
  handleTranslateRef.current = handleTranslate;
  const handlePinRef = useRef<(() => any) | null>(null);

  const selectAnnotationTool = (tool: AnnotationTool) => {
    const toolSize = annotationSizesRef.current[tool] ?? DEFAULT_ANNOTATION_SIZES[tool];
    annotationToolRef.current = tool;
    annotationSizeRef.current = toolSize;
    setAnnotationTool(tool);
    setAnnotationSizeState(toolSize);
  };

  const setCurrentAnnotationSize = (size: number) => {
    const safeSize = Math.max(1, Number(size) || 1);
    annotationSizesRef.current = { ...annotationSizesRef.current, [annotationToolRef.current]: safeSize };
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
    renderNeededRef.current = true;
  };

  const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
  const sleep = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

  const readTextSourceSnapshot = async (timeoutMs = 80): Promise<TextSourceSnapshot | null> => {
    const deadline = performance.now() + timeoutMs;
    let latest: TextSourceSnapshot | null = null;
    while (performance.now() <= deadline) {
      try {
        latest = await invoke<TextSourceSnapshot>("get_text_source_snapshot");
        if (latest?.status && latest.status !== "pending") return latest;
      } catch {
        return latest;
      }
      await sleep(12);
    }
    return latest;
  };

  const primeTextSourceSnapshot = (reason: string, timeoutMs = 120) => {
    const promise = readTextSourceSnapshot(timeoutMs).then((snapshot) => {
      if (snapshot?.status === "success") {
        console.info("[Text Source Snapshot]", reason, {
          elements: snapshot.elements?.length || 0,
          timings: snapshot.timings,
        });
      }
      return snapshot;
    });
    textSourceSnapshotPromiseRef.current = promise;
    return promise;
  };

  const buildTextSourceBlocksForSelection = (snapshot: TextSourceSnapshot | null, selection: Rect) => {
    if (!snapshot || snapshot.status !== "success" || !snapshot.screen || !snapshot.elements?.length) {
      return { blocks: [], maxElementCoverage: 0, maxSelectionCoverage: 0, matchedRawCount: 0, rejectedRawCount: 0, rejectedAggregateCount: 0 };
    }
    let physicalSelection: Rect;
    try {
      physicalSelection = getPhysicalSelection({
        canvas: canvasRef.current,
        image: imageRef.current as any,
        rect: selection,
      });
    } catch {
      return { blocks: [], maxElementCoverage: 0, maxSelectionCoverage: 0, matchedRawCount: 0, rejectedRawCount: 0, rejectedAggregateCount: 0 };
    }
    const result = (window as any).buildTextSourceBlocksForPhysicalSelection 
      ? (window as any).buildTextSourceBlocksForPhysicalSelection(snapshot.elements, snapshot.screen, physicalSelection)
      : { blocks: [], maxElementCoverage: 0, maxSelectionCoverage: 0, matchedRawCount: 0, rejectedRawCount: 0, rejectedAggregateCount: 0 };
    return result;
  };

  const getTextSourceBlocksForCurrentSelection = async (timeoutMs = 80) => {
    const started = performance.now();
    const snapshot = await Promise.race([
      textSourceSnapshotPromiseRef.current || readTextSourceSnapshot(timeoutMs),
      sleep(timeoutMs).then(() => null),
    ]);
    const textSourceSelection = buildTextSourceBlocksForSelection(snapshot, rectRef.current);
    const blocks = textSourceSelection.blocks;
    const charCount = blocks.reduce((sum: number, block: any) => sum + block.text.length, 0);
    const usable = blocks.length > 0 && charCount >= 2 && textSourceSelection.maxElementCoverage >= 0.55;
    return {
      usable,
      blocks: usable ? blocks : [],
      elapsedMs: Math.round(performance.now() - started),
      status: snapshot?.status || "empty",
      rawCount: snapshot?.elements?.length || 0,
      matchedRawCount: textSourceSelection.matchedRawCount,
      rejectedRawCount: textSourceSelection.rejectedRawCount,
      rejectedAggregateCount: textSourceSelection.rejectedAggregateCount,
      maxElementCoverage: Number(textSourceSelection.maxElementCoverage.toFixed(3)),
      maxSelectionCoverage: Number(textSourceSelection.maxSelectionCoverage.toFixed(3)),
    };
  };

  const focusScreenshotCanvas = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.focus({ preventScroll: true });
  };

  const focusScreenshotWindow = () => {
    const focusCanvas = () => {
      focusScreenshotCanvas();
      requestAnimationFrame(focusScreenshotCanvas);
    };
    getCurrentWindow().setFocus().then(focusCanvas).catch(focusCanvas);
    focusCanvas();
  };

  const releaseCanvasPointer = (canvas: HTMLCanvasElement, pointerId = activePointerIdRef.current) => {
    if (pointerId === null) return;
    try {
      if (canvas.hasPointerCapture(pointerId)) {
        canvas.releasePointerCapture(pointerId);
      }
    } catch {}
    if (activePointerIdRef.current === pointerId) {
      activePointerIdRef.current = null;
    }
  };

  const startPlainSelectionAt = (cx: number, cy: number) => {
    if (!overlayVisibleRef.current) return false;
    if (hasSelectedRef.current || isSelectingRef.current || isDraggingRef.current || isResizingRef.current) return false;
    if (isEditingRef.current || recordingPickerModeRef.current || scrollCaptureModeRef.current !== "idle") return false;
    pendingDetectionRef.current = null;
    mouseDownRef.current = { x: cx, y: cy };
    startPosRef.current = { x: cx, y: cy };
    selectionStartedAtRef.current = performance.now();
    selectionDragDistanceRef.current = 0;
    setIsSelecting(true);
    setHoverCandidate(null);
    setCurrentRect({ x: cx, y: cy, w: 0, h: 0 }, true);
    setSelection(false);
    renderNeededRef.current = true;
    return true;
  };

  const getDetectionRectAt = (mx: number, my: number) => {
    const candidates = getDetectionCandidatesAt(
      mx,
      my,
      windowRectsRef.current,
      analysisImageDataRef.current,
      configRef.current.enableVisualDetection === true,
      configRef.current.visualDetectionSensitivity || 3
    );
    return candidates[hoverCandidateIndexRef.current % Math.max(1, candidates.length)] || null;
  };

  const selectDetectedRect = (candidate: Rect) => {
    const canvas = canvasRef.current;
    const maxW = canvas?.width || window.innerWidth;
    const maxH = canvas?.height || window.innerHeight;
    const next = {
      x: clamp(Math.round(candidate.x), 0, maxW - 1),
      y: clamp(Math.round(candidate.y), 0, maxH - 1),
      w: Math.max(1, Math.min(Math.round(candidate.w), maxW - Math.round(candidate.x))),
      h: Math.max(1, Math.min(Math.round(candidate.h), maxH - Math.round(candidate.y))),
      kind: candidate.kind,
    };
    setCurrentRect(next, true);
    setSelection(true);
    setHoverCandidate(null);
    setTranslatedResult(null);
    translatedImgRef.current = null;
    setTranslatePairs(null);
    renderNeededRef.current = true;
    focusScreenshotWindow();
  };

  const openTextEditor = (point: Point, targetIndex: number | null, value = "") => {
    const selection = rectRef.current;
    const width = 180;
    const height = 34;
    const x = clamp(point.x - width / 2, selection.x + 8, selection.x + selection.w - width - 8);
    const y = clamp(point.y - height / 2, selection.y + 8, selection.y + selection.h - height - 8);
    setEditingTextDraft({ x, y, value, targetIndex });
  };

  const handleMouseDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!overlayVisibleRef.current) return;
    focusScreenshotWindow();
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
      activePointerIdRef.current = e.pointerId;
    } catch {
      activePointerIdRef.current = null;
    }
    if (e.button === 2) {
      e.preventDefault();
      if (hasSelectedRef.current) {
        setCurrentRect(EMPTY_RECT, true);
        setSelection(false);
        setTranslatedResult(null);
        translatedImgRef.current = null;
        setTranslatePairs(null);
        renderNeededRef.current = true;
      } else {
        cancelScreenshot();
      }
      return;
    }

    const cx = e.clientX;
    const cy = e.clientY;
    mouseDownRef.current = { x: cx, y: cy };
    if (hasSelectedRef.current && isPointInSelection(rectRef.current, true, cx, cy)) {
      if (scrollCaptureModeRef.current === "ready") {
        e.preventDefault();
        startManualScrollCapture();
        return;
      }
      if (scrollCaptureModeRef.current === "capturing") {
        e.preventDefault();
        finishManualScrollCapture();
        return;
      }
    }
    if (isEditingRef.current && isPointInSelection(rectRef.current, hasSelectedRef.current, cx, cy)) {
      const hitInfo = hitAnnotationDetailed(annotationsRef.current, { x: cx, y: cy }, annotationSizeRef.current);
      if (hitInfo) {
        const hit = annotationsRef.current[hitInfo.index];
        selectedAnnotationIndexRef.current = hitInfo.index;
        setSelectedAnnotationIndex(hitInfo.index);
        if (hit.type === "text") {
          openTextEditor({ x: hit.rect.x + hit.rect.w / 2, y: hit.rect.y + hit.rect.h / 2 }, hitInfo.index, hit.text || "");
          return;
        }
        if (hitInfo.action === "resize" && hitInfo.handle && (hit.type === "rect" || hit.type === "circle")) {
          isResizingAnnotationRef.current = true;
          annotationResizeHandleRef.current = hitInfo.handle;
          annotationDragStartRef.current = { x: cx, y: cy };
          annotationDragSnapshotRef.current = annotationsRef.current;
          return;
        }
        if (hitInfo.action === "move" && isDraggableAnnotation(hit)) {
          isDraggingAnnotationRef.current = true;
          annotationDragStartRef.current = { x: cx, y: cy };
          annotationDragSnapshotRef.current = annotationsRef.current;
          return;
        }
      } else {
        selectedAnnotationIndexRef.current = null;
        setSelectedAnnotationIndex(null);
      }
      if (annotationToolRef.current === "text") {
        openTextEditor({ x: cx, y: cy }, null);
        return;
      }
      isDrawingAnnotationRef.current = true;
      annotationStartRef.current = { x: cx, y: cy };
      setAnnotationDraft(
        annotationToolRef.current === "brush" || annotationToolRef.current === "mosaic"
          ? { type: annotationToolRef.current, rect: { x: cx, y: cy, w: 0, h: 0 }, points: [{ x: cx, y: cy }], color: annotationColorRef.current, size: annotationSizeRef.current }
          : { type: annotationToolRef.current, rect: { x: cx, y: cy, w: 0, h: 0 }, color: annotationColorRef.current, size: annotationSizeRef.current }
      );
      return;
    }
    const handleInfo = getHandleAt(rectRef.current, hasSelectedRef.current, cx, cy, true);
    if (handleInfo) {
      if (handleInfo.handle === "move") {
        isDraggingRef.current = true;
        dragStartRef.current = { x: cx, y: cy };
      } else {
        isResizingRef.current = handleInfo.handle;
        dragStartRef.current = { x: cx, y: cy };
        resizeStartRectRef.current = { ...rectRef.current };
      }
      return;
    }

    const detected = getDetectionRectAt(cx, cy);
    if (detected) {
      pendingDetectionRef.current = detected;
      startPosRef.current = { x: cx, y: cy };
      return;
    }

    startPlainSelectionAt(cx, cy);
  };

  const handleMouseMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (!overlayVisibleRef.current) return;
    const cx = e.clientX;
    const cy = e.clientY;
    if (
      (e.buttons & 1) === 1
      && activePointerIdRef.current !== null
      && !hasSelectedRef.current
      && !isSelectingRef.current
      && !isDraggingRef.current
      && !isResizingRef.current
    ) {
      startPlainSelectionAt(cx, cy);
    }
    lastMouseRef.current = { x: cx, y: cy };
    if (mouseTrackerRef.current) {
      mouseTrackerRef.current.style.left = `${cx + 16}px`;
      mouseTrackerRef.current.style.top = `${cy + 20}px`;
      mouseTrackerRef.current.textContent = `${cx}, ${cy}`;
    }

    if (isDraggingAnnotationRef.current && selectedAnnotationIndexRef.current !== null) {
      const current = annotationsRef.current[selectedAnnotationIndexRef.current];
      if (current && isDraggableAnnotation(current)) {
        const dx = cx - annotationDragStartRef.current.x;
        const dy = cy - annotationDragStartRef.current.y;
        annotationDragStartRef.current = { x: cx, y: cy };
        const next = annotationsRef.current.map((annotation, index) => index === selectedAnnotationIndexRef.current ? moveAnnotation(annotation, dx, dy, rectRef.current) : annotation);
        annotationsRef.current = next;
        setAnnotations(next);
        renderNeededRef.current = true;
      }
      return;
    }

    if (isResizingAnnotationRef.current && selectedAnnotationIndexRef.current !== null && annotationResizeHandleRef.current) {
      const dx = cx - annotationDragStartRef.current.x;
      const dy = cy - annotationDragStartRef.current.y;
      annotationDragStartRef.current = { x: cx, y: cy };
      const handle = annotationResizeHandleRef.current;
      const next = annotationsRef.current.map((annotation, index) => index === selectedAnnotationIndexRef.current ? resizeAnnotation(annotation, handle, dx, dy, rectRef.current) : annotation);
      annotationsRef.current = next;
      setAnnotations(next);
      renderNeededRef.current = true;
      return;
    }

    if (isDrawingAnnotationRef.current) {
      if (annotationToolRef.current === "brush" || annotationToolRef.current === "mosaic") {
        const current = draftAnnotationRef.current;
        const nextPoints = [...(current?.points || []), { x: clamp(cx, rectRef.current.x, rectRef.current.x + rectRef.current.w), y: clamp(cy, rectRef.current.y, rectRef.current.y + rectRef.current.h) }];
        const xs = nextPoints.map((p) => p.x);
        const ys = nextPoints.map((p) => p.y);
        setAnnotationDraft({ type: annotationToolRef.current, rect: { x: Math.min(...xs), y: Math.min(...ys), w: Math.max(...xs) - Math.min(...xs), h: Math.max(...ys) - Math.min(...ys) }, points: nextPoints, color: annotationColorRef.current, size: annotationSizeRef.current });
      } else if (annotationToolRef.current === "arrow") {
        setAnnotationDraft(makeLineAnnotation("arrow", annotationStartRef.current, { x: cx, y: cy }, rectRef.current, annotationColorRef.current, annotationSizeRef.current));
      } else {
        setAnnotationDraft({
          type: annotationToolRef.current,
          rect: normalizedRectFromPoints(annotationStartRef.current, { x: cx, y: cy }, rectRef.current),
          color: annotationColorRef.current,
          size: annotationSizeRef.current,
        });
      }
      renderNeededRef.current = true;
      return;
    }

    if (pendingDetectionRef.current) {
      const moved = Math.hypot(cx - mouseDownRef.current.x, cy - mouseDownRef.current.y);
      if (moved > 4) {
        selectionDragDistanceRef.current = moved;
        pendingDetectionRef.current = null;
        setHoverCandidate(null);
        setIsSelecting(true);
        setSelection(false);
        const next = { x: Math.min(startPosRef.current.x, cx), y: Math.min(startPosRef.current.y, cy), w: Math.abs(startPosRef.current.x - cx), h: Math.abs(startPosRef.current.y - cy) };
        setCurrentRect(next, true);
        renderNeededRef.current = true;
      }
      return;
    }

    if (isDraggingRef.current) {
      const dx = cx - dragStartRef.current.x;
      const dy = cy - dragStartRef.current.y;
      dragStartRef.current = { x: cx, y: cy };
      const canvas = canvasRef.current;
      const maxW = canvas?.width || window.innerWidth;
      const maxH = canvas?.height || window.innerHeight;
      const next = {
        x: Math.max(0, Math.min(maxW - rectRef.current.w, rectRef.current.x + dx)),
        y: Math.max(0, Math.min(maxH - rectRef.current.h, rectRef.current.y + dy)),
        w: rectRef.current.w,
        h: rectRef.current.h,
      };
      setCurrentRect(next, true);
      renderNeededRef.current = true;
      return;
    }

    if (isResizingRef.current) {
      const r = resizeStartRectRef.current;
      const dx = cx - dragStartRef.current.x;
      const dy = cy - dragStartRef.current.y;
      let x1 = r.x;
      let y1 = r.y;
      let x2 = r.x + r.w;
      let y2 = r.y + r.h;
      const handle = isResizingRef.current;
      if (handle.includes("e")) x2 = r.x + r.w + dx;
      if (handle.includes("w")) x1 = r.x + dx;
      if (handle.includes("s")) y2 = r.y + r.h + dy;
      if (handle.includes("n")) y1 = r.y + dy;
      const next = { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
      setCurrentRect(next, true);
      renderNeededRef.current = true;
      return;
    }

    if (isSelectingRef.current) {
      const snapX: number[] = [];
      const snapY: number[] = [];
      for (const wr of windowRectsRef.current) {
        snapX.push(wr.x, wr.x + wr.w);
        snapY.push(wr.y, wr.y + wr.h);
      }
      const snap = (val: number, refs: number[]) => {
        const dist = 15;
        for (const r of refs) if (Math.abs(val - r) < dist) return r;
        return val;
      };
      const snapCx = snap(cx, snapX);
      const snapCy = snap(cy, snapY);
      selectionDragDistanceRef.current = Math.max(selectionDragDistanceRef.current, Math.hypot(snapCx - startPosRef.current.x, snapCy - startPosRef.current.y));
      const next = { x: Math.min(startPosRef.current.x, snapCx), y: Math.min(startPosRef.current.y, snapCy), w: Math.abs(startPosRef.current.x - snapCx), h: Math.abs(startPosRef.current.y - snapCy) };
      setCurrentRect(next, true);
      renderNeededRef.current = true;
      return;
    }

    loadWindowRects();

    if (isEditingRef.current && isPointInSelection(rectRef.current, hasSelectedRef.current, cx, cy)) {
      const annotationHit = hitAnnotationDetailed(annotationsRef.current, { x: cx, y: cy }, annotationSizeRef.current);
      if (annotationHit) {
        e.currentTarget.style.cursor = annotationHit.cursor;
        return;
      }
    }

    const handleInfo = getHandleAt(rectRef.current, hasSelectedRef.current, cx, cy);
    if (handleInfo) {
      e.currentTarget.style.cursor = handleInfo.cursor;
      return;
    }
    const candidates = getDetectionCandidatesAt(cx, cy, windowRectsRef.current, analysisImageDataRef.current, configRef.current.enableVisualDetection === true, configRef.current.visualDetectionSensitivity || 3);
    setHoverCandidateList(candidates);
    const detected = hoverRectRef.current;
    e.currentTarget.style.cursor = detected ? "pointer" : "crosshair";
  };

  const handleMouseUp = (e?: React.PointerEvent<HTMLCanvasElement>) => {
    if (!overlayVisibleRef.current) return;
    if (e) releaseCanvasPointer(e.currentTarget, e.pointerId);
    const wasSelecting = isSelectingRef.current;
    const pendingDetection = pendingDetectionRef.current;
    pendingDetectionRef.current = null;
    if (isDrawingAnnotationRef.current) {
      isDrawingAnnotationRef.current = false;
      const draft = draftAnnotationRef.current;
      if (draft && ((draft.type === "brush" || draft.type === "mosaic") ? (draft.points?.length || 0) > 2 : draft.rect.w > 4 && draft.rect.h > 4)) commitAnnotation(draft);
      setAnnotationDraft(null);
      renderNeededRef.current = true;
      return;
    }
    if ((isDraggingAnnotationRef.current || isResizingAnnotationRef.current) && annotationDragSnapshotRef.current) {
      const snapshot = annotationDragSnapshotRef.current;
      const changed = snapshot !== annotationsRef.current;
      if (changed) {
        pushAnnotationHistory(snapshot);
      }
      annotationDragSnapshotRef.current = null;
    }
    isSelectingRef.current = false;
    setIsSelecting(false);
    isDraggingAnnotationRef.current = false;
    isResizingAnnotationRef.current = false;
    annotationResizeHandleRef.current = null;
    isDraggingRef.current = false;
    isResizingRef.current = null;
    setCurrentRect({ ...rectRef.current }, true);
    if (pendingDetection && !wasSelecting && !isDraggingRef.current && !isResizingRef.current) {
      selectDetectedRect(pendingDetection);
      return;
    }

    const valid = rectRef.current.w > 5 && rectRef.current.h > 5;
    const dragDistance = Math.max(selectionDragDistanceRef.current, Math.hypot(rectRef.current.w, rectRef.current.h));
    const explicitSelectionRelease = wasSelecting && dragDistance >= MIN_AUTO_ACTION_DRAG_PX;
    setSelection(valid);
    renderNeededRef.current = true;
    if (valid) requestAnimationFrame(focusScreenshotWindow);
    if (valid && explicitSelectionRelease && screenshotModeRef.current === "translate") {
      setTimeout(() => handleTranslate(), 0);
    }
    if (valid && explicitSelectionRelease && screenshotModeRef.current === "record") {
      setTimeout(() => enterRecordingMode("region"), 0);
    }
  };

  const handleDoubleClick = () => {
    if (!overlayVisibleRef.current) return;
    if (canConfirmCurrentSelection(220)) confirmScreenshot("copy");
  };

  const handlePointerCancel = (e: React.PointerEvent<HTMLCanvasElement>) => {
    releaseCanvasPointer(e.currentTarget, e.pointerId);
    isSelectingRef.current = false;
    setIsSelecting(false);
    isDraggingRef.current = false;
    isResizingRef.current = null;
    isDraggingAnnotationRef.current = false;
    isResizingAnnotationRef.current = false;
    annotationResizeHandleRef.current = null;
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
      selectionBorderColor: recordingStatusRef.current === "recording" ? RECORDING_BORDER_COLOR : recordingStatusRef.current === "ready" ? RECORDING_READY_BORDER_COLOR : scrollCaptureModeRef.current !== "idle" ? SCROLL_CAPTURE_BORDER_COLOR : undefined,
      selectionLabelColor: recordingStatusRef.current === "recording" ? "rgba(239, 68, 68, 0.9)" : recordingStatusRef.current === "ready" ? "rgba(37, 99, 235, 0.9)" : scrollCaptureModeRef.current !== "idle" ? "rgba(249, 115, 22, 0.9)" : undefined,
      selectionOnly: recordingStatusRef.current !== "idle",
    });
  }

  const cropCurrentSelectionFromLoadedImage = () => cropSelectionFromLoadedImage({
    canvas: canvasRef.current,
    image: imageRef.current as any,
    rect: rectRef.current,
  });

  const captureRegionBase64 = async () => {
    const { x, y, w, h } = getPhysicalSelection({
      canvas: canvasRef.current,
      image: imageRef.current as any,
      rect: rectRef.current,
    });
    try {
      const cropped = cropCurrentSelectionFromLoadedImage();
      if (cropped.base64) return cropped.base64;
    } catch (error) {
      console.warn("[ScreenshotPage] client-side crop failed, falling back to Rust crop", error);
    }
    return await invoke<string>("capture_region", { x, y, w, h });
  };

  const renderCurrentEditedSelectionBase64 = async () => renderEditedSelectionBase64({
    canvas: canvasRef.current,
    image: imageRef.current as any,
    rect: rectRef.current,
    translatedResult,
    annotations: annotationsRef.current,
    fallbackColor: annotationColorRef.current,
    fallbackSize: annotationSizeRef.current,
  });

  const getOutputBase64 = async () => (
    annotationsRef.current.length > 0 ? await renderCurrentEditedSelectionBase64() : (translatedResult || await captureRegionBase64())
  );

  const getSelectionConfirmDelayMs = (minAgeMs = MIN_SELECTION_CONFIRM_AGE_MS) => {
    if (
      !overlayVisibleRef.current
      || !hasSelectedRef.current
      || rectRef.current.w <= 5
      || rectRef.current.h <= 5
      || isSelectingRef.current
      || isDraggingRef.current
      || isResizingRef.current
      || isDrawingAnnotationRef.current
      || isDraggingAnnotationRef.current
      || isResizingAnnotationRef.current
    ) {
      return null;
    }
    const ageMs = performance.now() - selectionCompletedAtRef.current;
    return Math.max(0, minAgeMs - ageMs);
  };

  const canConfirmCurrentSelection = (minAgeMs = MIN_SELECTION_CONFIRM_AGE_MS) => (
    getSelectionConfirmDelayMs(minAgeMs) === 0
  );

  const handlePin = async () => {
    if (!hasSelected || rect.w <= 0 || rect.h <= 0) return;
    const { base64, x, y, w, h } = cropCurrentSelectionFromLoadedImage();
    if (!base64) return;

    try {
      await openPinWindow(await getOutputBase64(), { x, y, w, h });
      cancelScreenshot();
    } catch (error) {
      console.error("Failed to create pin window", error);
      message.error("钉图失败");
    }
  };
  handlePinRef.current = handlePin;

  const forceCloseScreenshots = async () => {
    message.destroy();
    const segments = [...recordingSegmentsRef.current];
    invoke("cancel_recording_process").catch(() => {});
    if (segments.length > 0) invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
    resetScreenshotState();
    await invoke("force_close_screenshots").catch(() => {});
  };

  const confirmScreenshot = async (action: "copy" | "save" | "both") => {
    const confirmDelayMs = getSelectionConfirmDelayMs();
    if (confirmDelayMs === null) return;
    if (confirmDelayMs > 0) {
      if (pendingConfirmTimerRef.current !== null) {
        window.clearTimeout(pendingConfirmTimerRef.current);
      }
      pendingConfirmTimerRef.current = window.setTimeout(() => {
        pendingConfirmTimerRef.current = null;
        confirmScreenshot(action);
      }, confirmDelayMs + 16);
      return;
    }
    if (pendingConfirmTimerRef.current !== null) {
      window.clearTimeout(pendingConfirmTimerRef.current);
      pendingConfirmTimerRef.current = null;
    }
    try {
      const base64 = await getOutputBase64();
      await emit("screenshot-captured", base64);
      if (action === "copy" || action === "both") {
        await invoke("copy_image_to_clipboard", { imageBase64: base64 });
      }
      message.destroy();
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label });
      if (action === "save") {
        try {
          await invoke<string>("save_image_to_file", { imageBase64: base64 });
        } catch (saveErr: any) {
          if (saveErr !== "用户取消了保存") {
            message.error("保存失败：" + (saveErr.message || saveErr.toString()));
          }
        }
      }
    } catch (e: any) {
      message.error("截图操作失败：" + (e.message || e.toString()));
    }
  };

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
      if (payload?.base64) {
        loadFullscreenFromBase64(payload.base64, payload.mode || screenshotModeRef.current || "normal");
        return;
      }
      loadFullscreen();
    })
      .then((unsub) => { unlistenEvent = unsub; })
      .catch(() => {});

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.altKey && e.key === "F4") {
        e.preventDefault();
        forceCloseScreenshots();
        return;
      }
      if (e.key === "Escape") {
        if (recordingPickerModeRef.current) {
          e.preventDefault();
          cancelRecordingTargetPicker();
          return;
        }
        if (recordingStatusRef.current !== "idle") {
          e.preventDefault();
          cancelRecording();
          return;
        }
        if (scrollCaptureModeRef.current !== "idle") {
          e.preventDefault();
          cancelManualScrollCapture();
          return;
        }
        forceCloseScreenshots();
        return;
      }
      if (e.key === "Tab" && hoverCandidatesRef.current.length > 1) {
        e.preventDefault();
        hoverCandidateIndexRef.current = (hoverCandidateIndexRef.current + (e.shiftKey ? -1 : 1) + hoverCandidatesRef.current.length) % hoverCandidatesRef.current.length;
        setHoverCandidate(hoverCandidatesRef.current[hoverCandidateIndexRef.current] || null);
        renderNeededRef.current = true;
        return;
      }
      if (!hasSelectedRef.current && (e.key === "Enter" || e.key === " ") && hoverRectRef.current) {
        e.preventDefault();
        selectDetectedRect(hoverRectRef.current);
        return;
      }
      if (!hasSelectedRef.current) return;
      if (editingTextDraftRef.current) commitTextDraft();
      if (!e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
        const toolByKey: Record<string, AnnotationTool> = {
          "1": "rect",
          "2": "circle",
          "3": "arrow",
          "4": "brush",
          "5": "text",
          "6": "mosaic",
          t: "text",
          T: "text",
        };
        const nextTool = toolByKey[e.key];
        if (nextTool) {
          e.preventDefault();
          setIsEditing(true);
          selectAnnotationTool(nextTool);
          return;
        }
      }
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === "d" || e.key === "D")) {
        e.preventDefault();
        if (!isOCRingRef.current && !isTranslatingRef.current && !isScrollCapturingRef.current && recordingStatusRef.current === "idle") handleOCR();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === "z" || e.key === "Z")) {
        e.preventDefault();
        undoAnnotation();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && ((e.key === "y" || e.key === "Y") || (e.shiftKey && (e.key === "z" || e.key === "Z")))) {
        e.preventDefault();
        redoAnnotation();
        return;
      }
      if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        deleteSelectedAnnotation();
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        confirmScreenshot("copy");
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "c" || e.key === "C")) {
        e.preventDefault();
        confirmScreenshot("copy");
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "s" || e.key === "S")) {
        e.preventDefault();
        if (recordingStatusRef.current !== "idle") {
          finishRecording();
          return;
        }
        if (scrollCaptureModeRef.current === "capturing") {
          finishManualScrollCapture();
          return;
        }
        confirmScreenshot("save");
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "q" || e.key === "Q")) {
        e.preventDefault();
        if (isTranslatingRef.current || isOCRingRef.current || isScrollCapturingRef.current) return;
        handleTranslateRef.current();
      }
      if (!e.ctrlKey && !e.metaKey && (e.key === "p" || e.key === "P")) {
        e.preventDefault();
        handlePinRef.current?.();
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      if (unlistenEvent) unlistenEvent();
      if (unlistenMode) unlistenMode();
      if (unlistenRecordingEnded) unlistenRecordingEnded();
      if (requestRef.current) cancelAnimationFrame(requestRef.current);
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
      setActionToolbarSize((current: { width: number, height: number }) => (
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
    <div style={{ position: "fixed", inset: 0, overflow: "hidden", cursor: hasSelected ? "default" : "crosshair" }}>
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

      <canvas ref={canvasRef} tabIndex={-1} onPointerDown={handleMouseDown} onPointerMove={handleMouseMove} onPointerUp={handleMouseUp} onPointerCancel={handlePointerCancel} onDoubleClick={handleDoubleClick} style={{ position: "absolute", top: 0, left: 0, zIndex: 10, cursor: "crosshair", outline: "none", touchAction: "none" }} />

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
