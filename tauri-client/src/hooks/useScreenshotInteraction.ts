import React, { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Rect, Annotation, Point, AnnotationTool, MarkerShape, ScreenshotPhysicalBounds } from "../types/screenshot";
import type { Config } from "../types/config";
import { invoke } from "@tauri-apps/api/core";
import { clamp, hitAnnotationDetailed, isDraggableAnnotation, makeLineAnnotation, makeNumberAnnotation, moveAnnotation, normalizedRectFromPoints, resizeAnnotation, type AnnotationResizeHandle } from "../utils/annotationGeometry";
import { getHandleAt, isPointInSelection } from "../utils/selectionGeometry";
import { getDetectionCandidatesAt } from "../utils/detectionCandidates";
import { getPhysicalSelection } from "../utils/screenshotImage";
import { logScreenshotPerf } from "../utils/debugLog";
import { getViewportDevicePixelRatio } from "../utils/screenshotViewport";

const MIN_AUTO_ACTION_DRAG_PX = 8;
const HOVER_DETECTION_MIN_INTERVAL_MS = 50;

type PendingPointerDown = {
  x: number;
  y: number;
  pointerId: number;
  sessionId: string | null;
  createdAt: number;
};

interface UseScreenshotInteractionProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  mouseTrackerRef: React.RefObject<HTMLDivElement | null>;
  rectRef: React.RefObject<Rect>;
  rect: Rect;
  setCurrentRect: (next: Rect, syncState?: boolean) => void;
  setSelection: (selected: boolean) => void;
  hasSelected: boolean;
  hasSelectedRef: React.RefObject<boolean>;
  overlayVisibleRef: React.RefObject<boolean>;
  frameInteractiveRef: React.RefObject<boolean>;
  imageReadyRef: React.RefObject<boolean>;
  activeSessionIdRef?: React.RefObject<string | null>;
  preShowDownOriginRef?: React.MutableRefObject<{ x: number; y: number; sessionId: string | null; source?: string; eventSeq?: number } | null>;
  displayedPhysicalBoundsRef?: React.RefObject<ScreenshotPhysicalBounds | null>;
  isSelecting: boolean;
  setIsSelecting: (selected: boolean) => void;
  isSelectingRef: React.RefObject<boolean>;
  isEditing: boolean;
  setIsEditing: (editing: boolean) => void;
  isEditingRef: React.RefObject<boolean>;
  screenshotMode: string;
  screenshotModeRef: React.RefObject<string>;
  configRef: React.RefObject<Config>;

  // Annotation
  editingTextDraftRef: React.RefObject<{ x: number; y: number; value: string; targetIndex: number | null } | null>;
  setEditingTextDraft: React.Dispatch<React.SetStateAction<{ x: number; y: number; value: string; targetIndex: number | null } | null>>;
  commitTextDraft: () => void;
  cancelTextDraft: () => void;
  commitAnnotation: (anno: Annotation) => void;
  setAnnotationDraft: (anno: Annotation | null) => void;
  draftAnnotationRef: React.RefObject<Annotation | null>;
  annotationsRef: React.RefObject<Annotation[]>;
  setAnnotations: React.Dispatch<React.SetStateAction<Annotation[]>>;
  annotationHistory: Annotation[][];
  redoAnnotations: Annotation[][];
  selectedAnnotationIndexRef: React.RefObject<number | null>;
  setSelectedAnnotationIndex: React.Dispatch<React.SetStateAction<number | null>>;
  pushAnnotationHistory: (snapshot: Annotation[]) => void;
  undoAnnotation: () => void;
  redoAnnotation: () => void;
  deleteSelectedAnnotation: () => void;
  selectAnnotationTool: (tool: AnnotationTool) => void;
  annotationToolRef: React.RefObject<AnnotationTool | null>;
  annotationColorRef: React.RefObject<string>;
  annotationSizeRef: React.RefObject<number>;
  annotationSizesRef: React.RefObject<Record<AnnotationTool, number>>;
  markerShapeRef: React.RefObject<MarkerShape>;
  selectMoveTool: () => void;

  // Window Rects
  windowRectsRef: React.RefObject<Rect[]>;
  hoverRectRef: React.RefObject<Rect | null>;
  hoverCandidatesRef: React.RefObject<Rect[]>;
  hoverCandidateIndexRef: React.RefObject<number>;
  setHoverCandidate: (candidate: Rect | null) => void;
  setHoverCandidateList: (candidates: Rect[]) => void;
  loadWindowRects: (force?: boolean) => void;
  clearWindowRects: () => void;

  // Recording & Scroll
  recordingStatus: string;
  recordingStatusRef: React.RefObject<string>;
  recordingPickerModeRef: React.RefObject<any>;
  scrollCaptureModeRef: React.RefObject<string>;
  startManualScrollCapture: () => void;
  finishManualScrollCapture: () => void;
  cancelManualScrollCapture: () => void;
  enterRecordingMode: (mode: "region" | "window" | "display") => void;
  cancelRecordingTargetPicker: () => void;
  cancelRecording: () => void;
  finishRecording: () => void;

  // OCR / Actions
  handleOCR: () => void;
  handleTranslate: () => void;
  confirmScreenshot: (action: "copy" | "save" | "both") => void;
  cancelScreenshot: () => void;
  handlePin: () => void;
  forceCloseScreenshots: () => void;
  runWgcExplicitSelectionDiagnostic?: () => void;
  lastMouseRef?: React.MutableRefObject<{ x: number; y: number }>;

  // State Refs
  selectionStartedAtRef: React.RefObject<number>;
  selectionCompletedAtRef: React.RefObject<number>;
  selectionDragDistanceRef: React.RefObject<number>;
  isOCRingRef: React.RefObject<boolean>;
  isTranslatingRef: React.RefObject<boolean>;
  isScrollCapturingRef: React.RefObject<boolean>;
  analysisImageDataRef: React.RefObject<ImageData | null>;
  pendingConfirmTimerRef: React.RefObject<number | null>;

  draw: (rx: number, ry: number, rw: number, rh: number) => void;
  syncToolbarPosition?: (next: Rect) => void;
}

export function useScreenshotInteraction({
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
  activeSessionIdRef,
  preShowDownOriginRef,
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
  selectAnnotationTool,
  annotationToolRef,
  annotationColorRef,
  annotationSizeRef,
  annotationSizesRef,
  markerShapeRef,
  selectMoveTool,

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
  runWgcExplicitSelectionDiagnostic,
  lastMouseRef: sharedLastMouseRef,

  selectionStartedAtRef,
  selectionCompletedAtRef,
  selectionDragDistanceRef,
  isOCRingRef,
  isTranslatingRef,
  isScrollCapturingRef,
  analysisImageDataRef,
  pendingConfirmTimerRef,

  draw,
  syncToolbarPosition,
}: UseScreenshotInteractionProps) {

  const activePointerIdRef = useRef<number | null>(null);
  const mouseDownRef = useRef({ x: 0, y: 0 });
  const startPosRef = useRef({ x: 0, y: 0 });
  const dragStartRef = useRef({ x: 0, y: 0 });
  const resizeStartRectRef = useRef<Rect>(EMPTY_RECT());
  const isDraggingRef = useRef(false);
  const isResizingRef = useRef<string | null>(null);
  const pendingDetectionRef = useRef<Rect | null>(null);
  const pendingDownRef = useRef<PendingPointerDown | null>(null);
  const pendingDownRafRef = useRef<number | null>(null);
  const annotationDragSnapshotRef = useRef<Annotation[] | null>(null);
  const isDrawingAnnotationRef = useRef(false);
  const isDraggingAnnotationRef = useRef(false);
  const isResizingAnnotationRef = useRef(false);
  const annotationResizeHandleRef = useRef<AnnotationResizeHandle | null>(null);
  const annotationStartRef = useRef({ x: 0, y: 0 });
  const annotationDragStartRef = useRef({ x: 0, y: 0 });
  const internalLastMouseRef = useRef({ x: 0, y: 0 });
  const lastMouseRef = sharedLastMouseRef || internalLastMouseRef;
  const firstPointerDownSessionRef = useRef<string | null>(null);
  const selectionMoveStatsRef = useRef({
    moves: 0,
    drawRequests: 0,
    startedAt: 0,
    lastMoveAt: 0,
    maxMoveGapMs: 0,
  });

  const logFirstPointerDownMetrics = (phase: string, pointerX: number, pointerY: number) => {
    const viewportWidth = Math.max(1, window.innerWidth);
    const viewportHeight = Math.max(1, window.innerHeight);
    const devicePixelRatio = getViewportDevicePixelRatio();
    const physicalBounds = displayedPhysicalBoundsRef?.current || null;
    const currentWindow = getCurrentWindow();
    const sessionKey = activeSessionIdRef?.current || "interaction";

    void Promise.all([
      currentWindow.outerPosition().catch(() => null),
      currentWindow.innerSize().catch(() => null),
      currentWindow.scaleFactor().catch(() => null),
    ]).then(([outerPosition, innerSize, scaleFactor]) => {
      logInteractionBaseline(
        phase,
        `x=${Math.round(pointerX)} y=${Math.round(pointerY)} dpr=${devicePixelRatio.toFixed(3)} viewport=${viewportWidth}x${viewportHeight} physical_bounds=${physicalBounds ? `${physicalBounds.x},${physicalBounds.y},${physicalBounds.width},${physicalBounds.height}` : "none"} outer_position=${outerPosition ? `${outerPosition.x},${outerPosition.y}` : "unknown"} inner_size=${innerSize ? `${innerSize.width}x${innerSize.height}` : "unknown"} scale_factor=${scaleFactor ?? "unknown"} session=${sessionKey}`,
      );
    });
  };

  const drawRafRef = useRef<number | null>(null);
  const drawRectRef = useRef<Rect | null>(null);
  const drawSessionRef = useRef<string | null>(null);
  const hoverDetectionFrameRef = useRef<number | null>(null);
  const hoverDetectionTimerRef = useRef<number | null>(null);
  const pendingHoverDetectionRef = useRef<{ x: number; y: number; sessionId: string | null } | null>(null);
  const lastHoverDetectionAtRef = useRef(0);
  const lastHoverDetectionRef = useRef<{
    x: number;
    y: number;
    rects: Rect[];
    imageData: ImageData | null;
    visualEnabled: boolean;
    sensitivity: number;
    candidates: Rect[];
  } | null>(null);

  const scheduleDraw = (x: number, y: number, w: number, h: number) => {
    drawRectRef.current = { x, y, w, h };
    drawSessionRef.current = activeSessionIdRef?.current || null;
    if (drawRafRef.current === null) {
      drawRafRef.current = requestAnimationFrame(() => {
        drawRafRef.current = null;
        const next = drawRectRef.current;
        const scheduledSession = drawSessionRef.current;
        drawRectRef.current = null;
        drawSessionRef.current = null;
        if (next && overlayVisibleRef.current && scheduledSession === (activeSessionIdRef?.current || null)) {
          draw(next.x, next.y, next.w, next.h);
        }
      });
    }
  };

  const cancelScheduledDraw = () => {
    if (drawRafRef.current !== null) {
      cancelAnimationFrame(drawRafRef.current);
      drawRafRef.current = null;
    }
    drawRectRef.current = null;
    drawSessionRef.current = null;
  };

  const cancelPendingDownResume = () => {
    if (pendingDownRafRef.current !== null) {
      cancelAnimationFrame(pendingDownRafRef.current);
      pendingDownRafRef.current = null;
    }
  };

  const cancelScheduledHoverDetection = () => {
    if (hoverDetectionFrameRef.current !== null) {
      cancelAnimationFrame(hoverDetectionFrameRef.current);
      hoverDetectionFrameRef.current = null;
    }
    if (hoverDetectionTimerRef.current !== null) {
      window.clearTimeout(hoverDetectionTimerRef.current);
      hoverDetectionTimerRef.current = null;
    }
    pendingHoverDetectionRef.current = null;
  };

  useEffect(() => {
    return () => {
      cancelScheduledDraw();
      cancelPendingDownResume();
      cancelScheduledHoverDetection();
    };
  }, []);

  function EMPTY_RECT(): Rect {
    return { x: 0, y: 0, w: 0, h: 0 };
  }

  const focusScreenshotCanvas = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.focus({ preventScroll: true });
  };

  const logInteractionBaseline = (phase: string, detail = "") => {
    const session = activeSessionIdRef?.current || "interaction";
    logScreenshotPerf(`[baseline] session=${session} phase=${phase} elapsed_ms=0 ${detail}`);
  };

  const waitForImageReady = async (timeoutMs = 1500) => {
    if (imageReadyRef.current) return true;
    const startedAt = performance.now();
    while (performance.now() - startedAt < timeoutMs) {
      await new Promise((resolve) => window.setTimeout(resolve, 16));
      if (imageReadyRef.current) return true;
    }
    return false;
  };

  const runWhenImageReady = (action: string, task: () => void) => {
    if (imageReadyRef.current) {
      task();
      return;
    }
    logInteractionBaseline("image_action_pending", `action=${action}`);
    waitForImageReady().then((ready) => {
      if (!ready) {
        logInteractionBaseline("image_action_timeout", `action=${action}`);
        return;
      }
      logInteractionBaseline("image_action_resumed", `action=${action}`);
      task();
    }).catch(() => {});
  };

  const focusScreenshotWindow = () => {
    const focusCanvas = () => {
      focusScreenshotCanvas();
      requestAnimationFrame(focusScreenshotCanvas);
    };
    import("@tauri-apps/api/window").then((m) => {
      const currentWindow = m.getCurrentWindow();
      invoke("activate_screenshot_overlay_for_interaction", { label: currentWindow.label })
        .then(() => currentWindow.setFocus())
        .then(focusCanvas)
        .catch(() => currentWindow.setFocus().then(focusCanvas).catch(focusCanvas));
    });
    focusCanvas();
  };

  const updateCurrentRect = (next: Rect, syncState = false) => {
    setCurrentRect(next, syncState);
    syncToolbarPosition?.(next);
  };

  const captureCanvasPointer = (canvas: HTMLCanvasElement, pointerId: number) => {
    try {
      canvas.setPointerCapture(pointerId);
      activePointerIdRef.current = pointerId;
    } catch {
      activePointerIdRef.current = null;
    }
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

  const hasActivePointerGesture = () => (
    isSelectingRef.current
    || isDraggingRef.current
    || isResizingRef.current !== null
    || isDrawingAnnotationRef.current
    || isDraggingAnnotationRef.current
    || isResizingAnnotationRef.current
  );

  const resetSelectionMoveStats = () => {
    selectionMoveStatsRef.current = {
      moves: 0,
      drawRequests: 0,
      startedAt: 0,
      lastMoveAt: 0,
      maxMoveGapMs: 0,
    };
  };

  const startPlainSelectionAt = (cx: number, cy: number) => {
    if (!frameInteractiveRef.current) return false;
    // 框选优先级最高：即便已存在选区，只要不是在拖动/缩放已有选区，也允许从空白处重新开始框选。
    if (isSelectingRef.current || isDraggingRef.current || isResizingRef.current) return false;
    if (isEditingRef.current || recordingPickerModeRef.current || scrollCaptureModeRef.current !== "idle") return false;
    pendingDetectionRef.current = null;
    mouseDownRef.current = { x: cx, y: cy };
    startPosRef.current = { x: cx, y: cy };
    selectionStartedAtRef.current = performance.now();
    selectionDragDistanceRef.current = 0;
    selectionMoveStatsRef.current = {
      moves: 0,
      drawRequests: 0,
      startedAt: selectionStartedAtRef.current,
      lastMoveAt: 0,
      maxMoveGapMs: 0,
    };
    setIsSelecting(true);
    isSelectingRef.current = true;
    setHoverCandidate(null);
    updateCurrentRect({ x: cx, y: cy, w: 0, h: 0 }, true);
    setSelection(false);
    return true;
  };

  const resumePendingDownIfReady = (event?: React.PointerEvent<HTMLCanvasElement>) => {
    const pending = pendingDownRef.current;
    if (!pending || !frameInteractiveRef.current || !overlayVisibleRef.current) return false;
    const currentSessionId = activeSessionIdRef?.current || null;
    if (pending.sessionId && currentSessionId && pending.sessionId !== currentSessionId) {
      pendingDownRef.current = null;
      cancelPendingDownResume();
      return false;
    }
    if (event && pending.pointerId !== event.pointerId) return false;
    if (event && (event.buttons & 1) !== 1) {
      pendingDownRef.current = null;
      cancelPendingDownResume();
      releaseCanvasPointer(event.currentTarget, event.pointerId);
      return false;
    }
    pendingDownRef.current = null;
    cancelPendingDownResume();
    if (event) {
      captureCanvasPointer(event.currentTarget, event.pointerId);
    }
    const started = startPlainSelectionAt(pending.x, pending.y);
    if (started) {
      logInteractionBaseline(
        "pending_pointer_down_resumed",
        `x=${Math.round(pending.x)} y=${Math.round(pending.y)} pointer=${pending.pointerId} session=${pending.sessionId || "none"}`
      );
    }
    return started;
  };

  const schedulePendingDownResume = () => {
    if (pendingDownRafRef.current !== null) return;
    pendingDownRafRef.current = requestAnimationFrame(() => {
      pendingDownRafRef.current = null;
      const pending = pendingDownRef.current;
      if (!pending) return;
      if (!overlayVisibleRef.current) {
        pendingDownRef.current = null;
        return;
      }
      if (performance.now() - pending.createdAt > 2500) {
        pendingDownRef.current = null;
        return;
      }
      if (!resumePendingDownIfReady()) {
        schedulePendingDownResume();
      }
    });
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

  const getHoverDetectionCandidatesAt = (mx: number, my: number) => {
    const visualEnabled = configRef.current.enableVisualDetection === true;
    const sensitivity = configRef.current.visualDetectionSensitivity || 3;
    const rects = windowRectsRef.current;
    const imageData = analysisImageDataRef.current;
    const cached = lastHoverDetectionRef.current;
    if (
      cached
      && Math.abs(cached.x - mx) < 2
      && Math.abs(cached.y - my) < 2
      && cached.rects === rects
      && cached.imageData === imageData
      && cached.visualEnabled === visualEnabled
      && cached.sensitivity === sensitivity
    ) {
      return cached.candidates;
    }
    const candidates = getDetectionCandidatesAt(mx, my, rects, imageData, visualEnabled, sensitivity);
    lastHoverDetectionRef.current = {
      x: mx,
      y: my,
      rects,
      imageData,
      visualEnabled,
      sensitivity,
      candidates,
    };
    return candidates;
  };

  const runHoverDetection = () => {
    hoverDetectionFrameRef.current = null;
    const pending = pendingHoverDetectionRef.current;
    pendingHoverDetectionRef.current = null;
    if (!pending || !frameInteractiveRef.current || hasActivePointerGesture()) return;
    const currentSession = activeSessionIdRef?.current || null;
    if (pending.sessionId !== currentSession) return;
    lastHoverDetectionAtRef.current = performance.now();
    loadWindowRects();
    setHoverCandidateList(getHoverDetectionCandidatesAt(pending.x, pending.y));
  };

  const scheduleHoverDetection = (mx: number, my: number) => {
    pendingHoverDetectionRef.current = {
      x: mx,
      y: my,
      sessionId: activeSessionIdRef?.current || null,
    };
    if (hoverDetectionFrameRef.current !== null || hoverDetectionTimerRef.current !== null) return;
    const elapsed = performance.now() - lastHoverDetectionAtRef.current;
    const delay = Math.max(0, HOVER_DETECTION_MIN_INTERVAL_MS - elapsed);
    const scheduleFrame = () => {
      hoverDetectionTimerRef.current = null;
      if (hoverDetectionFrameRef.current !== null) return;
      hoverDetectionFrameRef.current = requestAnimationFrame(runHoverDetection);
    };
    if (delay > 0) {
      hoverDetectionTimerRef.current = window.setTimeout(scheduleFrame, delay);
      return;
    }
    scheduleFrame();
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
    updateCurrentRect(next, true);
    setSelection(true);
    setHoverCandidate(null);
    selectionDragDistanceRef.current = Math.hypot(next.w, next.h);
    draw(next.x, next.y, next.w, next.h);
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
    if (!frameInteractiveRef.current) {
      const cx = e.clientX;
      const cy = e.clientY;
      lastMouseRef.current = { x: cx, y: cy };
      focusScreenshotWindow();
      if (e.button === 2) {
        e.preventDefault();
        cancelScreenshot();
        return;
      }
      if (e.button === 0 && overlayVisibleRef.current) {
        e.preventDefault();
        pendingDownRef.current = {
          x: cx,
          y: cy,
          pointerId: e.pointerId,
          sessionId: activeSessionIdRef?.current || null,
          createdAt: performance.now(),
        };
        captureCanvasPointer(e.currentTarget, e.pointerId);
        schedulePendingDownResume();
        logInteractionBaseline(
          "pending_pointer_down",
          `x=${Math.round(cx)} y=${Math.round(cy)} pointer=${e.pointerId} image_ready=${imageReadyRef.current}`
        );
      }
      return;
    }
    const activeSession = activeSessionIdRef?.current || "interaction";
    if (firstPointerDownSessionRef.current !== activeSession) {
      firstPointerDownSessionRef.current = activeSession;
      logInteractionBaseline("first_pointer_down", `x=${Math.round(e.clientX)} y=${Math.round(e.clientY)} image_ready=${imageReadyRef.current}`);
      logFirstPointerDownMetrics("first_pointer_down_metrics", e.clientX, e.clientY);
    }
    focusScreenshotCanvas();
    captureCanvasPointer(e.currentTarget, e.pointerId);
    if (e.button === 2) {
      e.preventDefault();
      if (hasSelectedRef.current) {
        updateCurrentRect(EMPTY_RECT(), true);
        setSelection(false);
        cancelScheduledDraw();
        draw(0, 0, 0, 0);
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
      const tool = annotationToolRef.current;
      if (tool === "text") {
        openTextEditor({ x: cx, y: cy }, null);
        return;
      }
      // F9: number marker - single click to place
      if (tool === "number") {
        const index = annotationsRef.current.filter((a) => a.type === "number").length + 1;
        commitAnnotation(makeNumberAnnotation({ x: cx, y: cy }, annotationColorRef.current, annotationSizeRef.current, markerShapeRef.current, index));
        return;
      }
      if (!tool) return;
      isDrawingAnnotationRef.current = true;
      annotationStartRef.current = { x: cx, y: cy };
      setAnnotationDraft(
        tool === "brush" || tool === "mosaic"
          ? { type: tool, rect: { x: cx, y: cy, w: 0, h: 0 }, points: [{ x: cx, y: cy }], color: annotationColorRef.current, size: annotationSizeRef.current }
          : { type: tool, rect: { x: cx, y: cy, w: 0, h: 0 }, color: annotationColorRef.current, size: annotationSizeRef.current }
      );
      // P0-1: draw the annotation origin immediately on pointerdown.
      scheduleDraw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
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

    // BUG2 修复：按下立即开始拉框，不再被窗口/控件识别抢占而进入 pending 状态。
    // 若用户实际只是“点击未拖动”，handleMouseUp 中 dragDistance < MIN_AUTO_ACTION_DRAG_PX
    // 分支会重新识别并选中悬停窗口，点击选窗的体验保持不变。
    startPlainSelectionAt(cx, cy);
  };

  const handleMouseMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const cx = e.clientX;
    const cy = e.clientY;
    lastMouseRef.current = { x: cx, y: cy };
    if (!frameInteractiveRef.current) return;
    const primaryButtonDown = (e.buttons & 1) === 1;
    resumePendingDownIfReady(e);
    if (!primaryButtonDown) {
      if (pendingDownRef.current) {
        pendingDownRef.current = null;
        cancelPendingDownResume();
      }
      if (activePointerIdRef.current === e.pointerId && !hasActivePointerGesture()) {
        releaseCanvasPointer(e.currentTarget, e.pointerId);
      }
      if (hasActivePointerGesture()) {
        logInteractionBaseline(
          "lost_pointer_up_finalized",
          `x=${Math.round(cx)} y=${Math.round(cy)} pointer=${e.pointerId}`
        );
        handleMouseUp(e);
        return;
      }
    }
    if (
      primaryButtonDown
      && !isSelectingRef.current
      && !isDraggingRef.current
      && !isResizingRef.current
    ) {
      if (activePointerIdRef.current === null) {
        captureCanvasPointer(e.currentTarget, e.pointerId);
        const activeSession = activeSessionIdRef?.current || "interaction";
        if (firstPointerDownSessionRef.current !== activeSession) {
          firstPointerDownSessionRef.current = activeSession;
          logInteractionBaseline(
            "first_pointer_move_down",
            `x=${Math.round(cx)} y=${Math.round(cy)} image_ready=${imageReadyRef.current}`
          );
        }
      }
      // BUG B 修复：选区起点必须用手势真实的按下点，而不是补帧时光标已经移动到的位置。
      // 场景：overlay 在拖拽中途才变为可交互、首个 pointerdown 没能形成 pendingDown 时，
      // 旧逻辑直接用当前光标(往往已接近松开/点击点)作锄点，导致选区起点偶发从松开/点击处开始。
      // 优先采用真实按下点(pendingDownRef)，并立即按当前光标拉到正确尺寸，避免零尺寸框闪烁与跳变。
      const activeSession = activeSessionIdRef?.current || null;
      const preShowOrigin =
        preShowDownOriginRef?.current
        && (!preShowDownOriginRef.current.sessionId || preShowDownOriginRef.current.sessionId === activeSession)
          ? preShowDownOriginRef.current
          : null;
      const pendingOrigin = pendingDownRef.current;
      const anchorOrigin = pendingOrigin || preShowOrigin;
      const selectionAnchorX = anchorOrigin ? anchorOrigin.x : cx;
      const selectionAnchorY = anchorOrigin ? anchorOrigin.y : cy;
      if (pendingOrigin) {
        pendingDownRef.current = null;
        cancelPendingDownResume();
      }
      if (!pendingOrigin && preShowOrigin && preShowDownOriginRef) {
        preShowDownOriginRef.current = null;
        logInteractionBaseline(
          "pre_show_down_origin_used",
          `source=${preShowOrigin.source || "pre-capture"} event_seq=${preShowOrigin.eventSeq || 0} x=${Math.round(preShowOrigin.x)} y=${Math.round(preShowOrigin.y)}`
        );
      }
      const fallbackSelectionStarted = startPlainSelectionAt(selectionAnchorX, selectionAnchorY);
      if (fallbackSelectionStarted && (selectionAnchorX !== cx || selectionAnchorY !== cy)) {
        const grownRect = {
          x: Math.min(selectionAnchorX, cx),
          y: Math.min(selectionAnchorY, cy),
          w: Math.abs(selectionAnchorX - cx),
          h: Math.abs(selectionAnchorY - cy),
        };
        updateCurrentRect(grownRect, false);
        selectionDragDistanceRef.current = Math.hypot(grownRect.w, grownRect.h);
        scheduleDraw(grownRect.x, grownRect.y, grownRect.w, grownRect.h);
      }
    }
    if (mouseTrackerRef.current && mouseTrackerRef.current.style.display !== "none") {
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
        // BUG1 修复：拖动标注时同步重绘，保证实时轨迹（与绘制分支保持一致）。
        scheduleDraw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
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
      // BUG1 修复：缩放标注时同步重绘，保证实时轨迹。
      scheduleDraw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
      return;
    }

    if (isDrawingAnnotationRef.current) {
      const tool = annotationToolRef.current;
      if (tool) {
        if (tool === "brush" || tool === "mosaic") {
          const current = draftAnnotationRef.current;
          const nextPoints = [...(current?.points || []), { x: clamp(cx, rectRef.current.x, rectRef.current.x + rectRef.current.w), y: clamp(cy, rectRef.current.y, rectRef.current.y + rectRef.current.h) }];
          const xs = nextPoints.map((p) => p.x);
          const ys = nextPoints.map((p) => p.y);
          setAnnotationDraft({ type: tool, rect: { x: Math.min(...xs), y: Math.min(...ys), w: Math.max(...xs) - Math.min(...xs), h: Math.max(...ys) - Math.min(...ys) }, points: nextPoints, color: annotationColorRef.current, size: annotationSizeRef.current });
        } else if (tool === "arrow") {
          setAnnotationDraft(makeLineAnnotation("arrow", annotationStartRef.current, { x: cx, y: cy }, rectRef.current, annotationColorRef.current, annotationSizeRef.current));
        } else {
          setAnnotationDraft({
            type: tool,
            rect: normalizedRectFromPoints(annotationStartRef.current, { x: cx, y: cy }, rectRef.current),
            color: annotationColorRef.current,
            size: annotationSizeRef.current,
          });
        }
        // Redraw after each draft update so annotations follow the pointer.
        scheduleDraw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
      }
      return;
    }

    if (pendingDetectionRef.current) {
      const moved = Math.hypot(cx - mouseDownRef.current.x, cy - mouseDownRef.current.y);
      if (moved > MIN_AUTO_ACTION_DRAG_PX) {
        selectionDragDistanceRef.current = moved;
        pendingDetectionRef.current = null;
        setHoverCandidate(null);
        setIsSelecting(true);
        isSelectingRef.current = true;
        setSelection(false);
        const next = { x: Math.min(startPosRef.current.x, cx), y: Math.min(startPosRef.current.y, cy), w: Math.abs(startPosRef.current.x - cx), h: Math.abs(startPosRef.current.y - cy) };
        updateCurrentRect(next, false);
        scheduleDraw(next.x, next.y, next.w, next.h);
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
      updateCurrentRect(next, false);
      scheduleDraw(next.x, next.y, next.w, next.h);
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
      const newX = Math.min(x1, x2);
      const newY = Math.min(y1, y2);
      const newW = Math.abs(x2 - x1);
      const newH = Math.abs(y2 - y1);
      const next = { x: Math.round(newX), y: Math.round(newY), w: Math.round(newW), h: Math.round(newH) };
      updateCurrentRect(next, false);
      scheduleDraw(next.x, next.y, next.w, next.h);
      return;
    }

    if (isSelectingRef.current) {
      const moveNow = performance.now();
      const stats = selectionMoveStatsRef.current;
      if (stats.lastMoveAt > 0) {
        stats.maxMoveGapMs = Math.max(stats.maxMoveGapMs, moveNow - stats.lastMoveAt);
      }
      stats.moves += 1;
      stats.lastMoveAt = moveNow;
      const snapX: number[] = [];
      const snapY: number[] = [];
      for (const wr of windowRectsRef.current) {
        // 不吸附整屏 display 矩形，避免靠近屏幕边缘时选框被异常拉拽。
        if ((wr as { kind?: string }).kind === "display") continue;
        snapX.push(wr.x, wr.x + wr.w);
        snapY.push(wr.y, wr.y + wr.h);
      }
      const snap = (val: number, refs: number[]) => {
        const enabled = configRef.current?.edgeSnapEnabled ?? true;
        if (!enabled) return val;
        const dist = configRef.current?.edgeSnapDistance ?? 8;
        if (dist <= 0) return val;
        for (const r of refs) if (Math.abs(val - r) < dist) return r;
        return val;
      };
      const snapCx = snap(cx, snapX);
      const snapCy = snap(cy, snapY);
      selectionDragDistanceRef.current = Math.max(selectionDragDistanceRef.current, Math.hypot(snapCx - startPosRef.current.x, snapCy - startPosRef.current.y));
      const next = { x: Math.min(startPosRef.current.x, snapCx), y: Math.min(startPosRef.current.y, snapCy), w: Math.abs(startPosRef.current.x - snapCx), h: Math.abs(startPosRef.current.y - snapCy) };
      updateCurrentRect(next, false);
      selectionMoveStatsRef.current.drawRequests += 1;
      scheduleDraw(next.x, next.y, next.w, next.h);
      return;
    }

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
    // 光标修复：选区已完成且悬停在选区内部（可整体拖动）时显示 move；
    // 拉选区阶段仍为 crosshair（行业标准）。
    if (hasSelectedRef.current && isPointInSelection(rectRef.current, true, cx, cy)) {
      e.currentTarget.style.cursor = "move";
      return;
    }
    scheduleHoverDetection(cx, cy);
    // BUG3 修复：框选阶段始终使用 crosshair（行业标准框选手势）。
    // 窗口高亮预览仍保留（hoverRect 继续参与渲染），仅不再把光标改成 pointer。
    e.currentTarget.style.cursor = "crosshair";
  };

  const handleMouseUp = (e?: React.PointerEvent<HTMLCanvasElement>) => {
    if (e && pendingDownRef.current?.pointerId === e.pointerId) {
      pendingDownRef.current = null;
      cancelPendingDownResume();
      releaseCanvasPointer(e.currentTarget, e.pointerId);
    }
    if (!frameInteractiveRef.current) return;
    if (e) releaseCanvasPointer(e.currentTarget, e.pointerId);
    const wasSelecting = isSelectingRef.current;
    const pendingDetection = pendingDetectionRef.current;
    pendingDetectionRef.current = null;
    if (isDrawingAnnotationRef.current) {
      isDrawingAnnotationRef.current = false;
      const draft = draftAnnotationRef.current;
      if (draft && ((draft.type === "brush" || draft.type === "mosaic") ? (draft.points?.length || 0) > 2 : draft.rect.w > 4 && draft.rect.h > 4)) commitAnnotation(draft);
      setAnnotationDraft(null);
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
    updateCurrentRect({ ...rectRef.current }, true);
    cancelScheduledDraw();
    draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
    if (pendingDetection && !wasSelecting && !isDraggingRef.current && !isResizingRef.current) {
      selectDetectedRect(pendingDetection);
      return;
    }

    const valid = rectRef.current.w > 5 && rectRef.current.h > 5;
    const dragDistance = Math.max(selectionDragDistanceRef.current, Math.hypot(rectRef.current.w, rectRef.current.h));
    const hadSelectionMoveStats = selectionMoveStatsRef.current.startedAt > 0;
    if (wasSelecting || hadSelectionMoveStats) {
      const stats = selectionMoveStatsRef.current;
      const durationMs = stats.startedAt > 0 ? performance.now() - stats.startedAt : 0;
      logInteractionBaseline(
        "selection_drag_finished",
        `valid=${valid} moves=${stats.moves} draw_requests=${stats.drawRequests} duration_ms=${Math.round(durationMs)} max_move_gap_ms=${Math.round(stats.maxMoveGapMs)} rect=${Math.round(rectRef.current.x)},${Math.round(rectRef.current.y)},${Math.round(rectRef.current.w)},${Math.round(rectRef.current.h)}`
      );
      resetSelectionMoveStats();
    }
    // 框选优先级最高：仅当“几乎未拖动的单击”(无有效框) 且窗口磁吸/检测开关开启时，才补位识别窗口；真实拖框永不被覆盖。
    const autoDetectEnabled = configRef.current.enableVisualDetection === true || configRef.current.enableUiControlDetection === true;
    if (wasSelecting && !valid && dragDistance < MIN_AUTO_ACTION_DRAG_PX && e && autoDetectEnabled) {
      const detected = getDetectionRectAt(e.clientX, e.clientY);
      if (detected && detected.kind !== "display" && detected.kind !== "taskbar") {
        selectDetectedRect(detected);
        return;
      }
    }
    const explicitSelectionRelease = wasSelecting && dragDistance >= MIN_AUTO_ACTION_DRAG_PX;
    setSelection(valid);
    if (valid) requestAnimationFrame(focusScreenshotWindow);
    if (valid && explicitSelectionRelease && screenshotModeRef.current === "translate") {
      setTimeout(() => runWhenImageReady("auto_translate", handleTranslate), 0);
    }
    if (valid && explicitSelectionRelease && screenshotModeRef.current === "record") {
      // setTimeout(() => enterRecordingMode("region"), 0);
    }
  };

  const handlePointerCancel = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if (pendingDownRef.current?.pointerId === e.pointerId) {
      pendingDownRef.current = null;
      cancelPendingDownResume();
    }
    releaseCanvasPointer(e.currentTarget, e.pointerId);
    isSelectingRef.current = false;
    setIsSelecting(false);
    isDraggingRef.current = false;
    isResizingRef.current = null;
    isDraggingAnnotationRef.current = false;
    isResizingAnnotationRef.current = false;
    annotationResizeHandleRef.current = null;
    resetSelectionMoveStats();
    cancelScheduledDraw();
  };

  const resetInteractionState = () => {
    const canvas = canvasRef.current;
    if (canvas) releaseCanvasPointer(canvas);
    activePointerIdRef.current = null;
    pendingDownRef.current = null;
    cancelPendingDownResume();
    cancelScheduledHoverDetection();
    pendingDetectionRef.current = null;
    annotationDragSnapshotRef.current = null;
    annotationResizeHandleRef.current = null;
    isSelectingRef.current = false;
    isDraggingRef.current = false;
    isResizingRef.current = null;
    isDrawingAnnotationRef.current = false;
    isDraggingAnnotationRef.current = false;
    isResizingAnnotationRef.current = false;
    setIsSelecting(false);
    setAnnotationDraft(null);
    cancelScheduledDraw();
  };

  const handleDoubleClick = () => {
    if (!frameInteractiveRef.current) return;
    const canConfirm = getSelectionConfirmDelayMs();
    if (canConfirm === 0) runWhenImageReady("double_click_copy", () => confirmScreenshot("copy"));
  };

  const getSelectionConfirmDelayMs = (minAgeMs = 120) => {
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

  useEffect(() => {
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
      if (!frameInteractiveRef.current) return;
      if (e.key === "Tab" && hoverCandidatesRef.current.length > 1) {
        e.preventDefault();
        hoverCandidateIndexRef.current = (hoverCandidateIndexRef.current + (e.shiftKey ? -1 : 1) + hoverCandidatesRef.current.length) % hoverCandidatesRef.current.length;
        setHoverCandidate(hoverCandidatesRef.current[hoverCandidateIndexRef.current] || null);
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
      if ((e.ctrlKey || e.metaKey) && e.altKey && (e.key === "w" || e.key === "W")) {
        e.preventDefault();
        runWgcExplicitSelectionDiagnostic?.();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && (e.key === "d" || e.key === "D")) {
        e.preventDefault();
        if (!isOCRingRef.current && !isTranslatingRef.current && !isScrollCapturingRef.current && recordingStatusRef.current === "idle") {
          runWhenImageReady("ocr", handleOCR);
        }
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
        runWhenImageReady("enter_copy", () => confirmScreenshot("copy"));
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "c" || e.key === "C")) {
        e.preventDefault();
        runWhenImageReady("copy", () => confirmScreenshot("copy"));
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
        runWhenImageReady("save", () => confirmScreenshot("save"));
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "q" || e.key === "Q")) {
        e.preventDefault();
        if (isTranslatingRef.current || isOCRingRef.current || isScrollCapturingRef.current) return;
        runWhenImageReady("translate", handleTranslate);
      }
      if (!e.ctrlKey && !e.metaKey && (e.key === "p" || e.key === "P")) {
        e.preventDefault();
        runWhenImageReady("pin", handlePin);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [
    undoAnnotation,
    redoAnnotation,
    deleteSelectedAnnotation,
    commitTextDraft,
    selectAnnotationTool,
    setIsEditing,
    handleOCR,
    handleTranslate,
    handlePin,
    confirmScreenshot,
    forceCloseScreenshots,
    runWgcExplicitSelectionDiagnostic,
    cancelRecordingTargetPicker,
    cancelRecording,
    cancelManualScrollCapture,
    finishManualScrollCapture,
    finishRecording,
    setHoverCandidate,
  ]);

  const getAnnotationState = () => ({
    get isDrawing() { return isDrawingAnnotationRef.current; },
    get isDragging() { return isDraggingAnnotationRef.current; },
    get isResizing() { return isResizingAnnotationRef.current; },
  });

  return {
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
    handlePointerCancel,
    handleDoubleClick,
    focusScreenshotWindow,
    resetInteractionState,
    annotationStateRef: { get current() { return getAnnotationState(); } },
  };
}
