import React, { useEffect, useRef } from "react";
import type { Rect, Annotation, Point, AnnotationTool } from "../types/screenshot";
import type { Config } from "../types/config";
import { clamp, hitAnnotationDetailed, isDraggableAnnotation, makeLineAnnotation, moveAnnotation, normalizedRectFromPoints, resizeAnnotation, type AnnotationResizeHandle } from "../utils/annotationGeometry";
import { getHandleAt, isPointInSelection } from "../utils/selectionGeometry";
import { getDetectionCandidatesAt } from "../utils/detectionCandidates";
import { getPhysicalSelection } from "../utils/screenshotImage";

const MIN_AUTO_ACTION_DRAG_PX = 8;

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
  const annotationDragSnapshotRef = useRef<Annotation[] | null>(null);
  const isDrawingAnnotationRef = useRef(false);
  const isDraggingAnnotationRef = useRef(false);
  const isResizingAnnotationRef = useRef(false);
  const annotationResizeHandleRef = useRef<AnnotationResizeHandle | null>(null);
  const annotationStartRef = useRef({ x: 0, y: 0 });
  const annotationDragStartRef = useRef({ x: 0, y: 0 });
  const internalLastMouseRef = useRef({ x: 0, y: 0 });
  const lastMouseRef = sharedLastMouseRef || internalLastMouseRef;

  function EMPTY_RECT(): Rect {
    return { x: 0, y: 0, w: 0, h: 0 };
  }

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
    import("@tauri-apps/api/window").then((m) => {
      m.getCurrentWindow().setFocus().then(focusCanvas).catch(focusCanvas);
    });
    focusCanvas();
  };

  const updateCurrentRect = (next: Rect, syncState = false) => {
    setCurrentRect(next, syncState);
    syncToolbarPosition?.(next);
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
    if (!frameInteractiveRef.current) return false;
    if (hasSelectedRef.current || isSelectingRef.current || isDraggingRef.current || isResizingRef.current) return false;
    if (isEditingRef.current || recordingPickerModeRef.current || scrollCaptureModeRef.current !== "idle") return false;
    pendingDetectionRef.current = null;
    mouseDownRef.current = { x: cx, y: cy };
    startPosRef.current = { x: cx, y: cy };
    selectionStartedAtRef.current = performance.now();
    selectionDragDistanceRef.current = 0;
    setIsSelecting(true);
    isSelectingRef.current = true;
    setHoverCandidate(null);
    updateCurrentRect({ x: cx, y: cy, w: 0, h: 0 }, true);
    setSelection(false);
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
    updateCurrentRect(next, true);
    setSelection(true);
    setHoverCandidate(null);
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
    if (!frameInteractiveRef.current) return;
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
        updateCurrentRect(EMPTY_RECT(), true);
        setSelection(false);
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
      if (!tool) return;
      isDrawingAnnotationRef.current = true;
      annotationStartRef.current = { x: cx, y: cy };
      setAnnotationDraft(
        tool === "brush" || tool === "mosaic"
          ? { type: tool, rect: { x: cx, y: cy, w: 0, h: 0 }, points: [{ x: cx, y: cy }], color: annotationColorRef.current, size: annotationSizeRef.current }
          : { type: tool, rect: { x: cx, y: cy, w: 0, h: 0 }, color: annotationColorRef.current, size: annotationSizeRef.current }
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
    if (!frameInteractiveRef.current) return;
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
      }
      return;
    }

    if (pendingDetectionRef.current) {
      const moved = Math.hypot(cx - mouseDownRef.current.x, cy - mouseDownRef.current.y);
      if (moved > 4) {
        selectionDragDistanceRef.current = moved;
        pendingDetectionRef.current = null;
        setHoverCandidate(null);
        setIsSelecting(true);
        isSelectingRef.current = true;
        setSelection(false);
        const next = { x: Math.min(startPosRef.current.x, cx), y: Math.min(startPosRef.current.y, cy), w: Math.abs(startPosRef.current.x - cx), h: Math.abs(startPosRef.current.y - cy) };
        updateCurrentRect(next, false);
        draw(next.x, next.y, next.w, next.h);
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
      draw(next.x, next.y, next.w, next.h);
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
      updateCurrentRect(next, false);
      draw(next.x, next.y, next.w, next.h);
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
      updateCurrentRect(next, false);
      draw(next.x, next.y, next.w, next.h);
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
    if (pendingDetection && !wasSelecting && !isDraggingRef.current && !isResizingRef.current) {
      selectDetectedRect(pendingDetection);
      return;
    }

    const valid = rectRef.current.w > 5 && rectRef.current.h > 5;
    const dragDistance = Math.max(selectionDragDistanceRef.current, Math.hypot(rectRef.current.w, rectRef.current.h));
    const explicitSelectionRelease = wasSelecting && dragDistance >= MIN_AUTO_ACTION_DRAG_PX;
    setSelection(valid);
    if (valid) requestAnimationFrame(focusScreenshotWindow);
    if (valid && explicitSelectionRelease && screenshotModeRef.current === "translate") {
      setTimeout(() => handleTranslate(), 0);
    }
    if (valid && explicitSelectionRelease && screenshotModeRef.current === "record") {
      // setTimeout(() => enterRecordingMode("region"), 0);
    }
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

  const handleDoubleClick = () => {
    if (!frameInteractiveRef.current) return;
    const canConfirm = getSelectionConfirmDelayMs();
    if (canConfirm === 0) confirmScreenshot("copy");
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
        handleTranslate();
      }
      if (!e.ctrlKey && !e.metaKey && (e.key === "p" || e.key === "P")) {
        e.preventDefault();
        handlePin();
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
    annotationStateRef: { get current() { return getAnnotationState(); } },
  };
}
