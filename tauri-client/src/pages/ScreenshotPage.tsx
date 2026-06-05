import React, { useEffect, useRef, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { Button, Space, message } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ScreenshotToolbar from "../components/screenshot/ScreenshotToolbar";
import TextAnnotationEditor from "../components/screenshot/TextAnnotationEditor";
import TranslationLoadingOverlay from "../components/screenshot/TranslationLoadingOverlay";
import type { Config } from "../types/config";
import type { Annotation, AnnotationTool, EditingTextDraft, OcrBlock, Point, Rect, TranslatePair } from "../types/screenshot";
import { useScreenshotOcr } from "../hooks/useScreenshotOcr";
import { useScreenshotAnnotation, DEFAULT_ANNOTATION_COLOR, DEFAULT_ANNOTATION_TOOL, DEFAULT_ANNOTATION_SIZES } from "../hooks/useScreenshotAnnotation";
import { clamp, hitAnnotationDetailed, isDraggableAnnotation, makeLineAnnotation, makeTextAnnotation, moveAnnotation, normalizedRectFromPoints, resizeAnnotation, type AnnotationResizeHandle } from "../utils/annotationGeometry";
import { cropSelectionFromLoadedImage, getPhysicalSelection, loadPngImage, renderEditedSelectionBase64 } from "../utils/screenshotImage";
import { openOcrResultWindow } from "../utils/ocrResultWindow";
import { getActionToolbarStyle, FLOATING_PANEL_MARGIN, FLOATING_PANEL_GAP, OCR_WINDOW_SIZE } from "../utils/screenshotLayout";
import { getHandleAt, isPointInSelection } from "../utils/selectionGeometry";
import { renderTranslatedBlocks } from "../utils/translatedBlocks";
import { openPinWindow } from "../utils/pinWindows";
import { getDetectionCandidatesAt, rectSignature } from "../utils/detectionCandidates";
import { prewarmTranslationServices, translateOcrBlocks, translateWithLocalOcr } from "../utils/localOcrTranslate";
import { renderScreenshotCanvas } from "../utils/renderScreenshotCanvas";
import { openRecordingWindows } from "../utils/recordingWindows";
import { buildTextSourceBlocksForPhysicalSelection } from "../utils/textSourceSelection";
import { buildOcrNormalizationReport } from "../ocr-processing";
import RecordingTargetPicker from "../components/recording/RecordingTargetPicker";


const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };
const ACTION_TOOLBAR_FALLBACK_SIZE = { width: 680, height: 86 };
const RECORDING_BORDER_COLOR = "#ef4444";
const RECORDING_READY_BORDER_COLOR = "#2563eb";
const SCROLL_CAPTURE_BORDER_COLOR = "#f97316";
const RECORDING_TOOLBAR_FALLBACK_SIZE = { width: 980, height: 96 };
const MIN_NATIVE_RECOVERY_DRAG_PX = 4;
const MIN_AUTO_ACTION_DRAG_PX = 8;
const MIN_SELECTION_CONFIRM_AGE_MS = 120;

type RecordingStatus = "idle" | "ready" | "recording";
type RecordingMode = "region" | "window" | "display";
type ScrollCaptureMode = "idle" | "ready" | "capturing";

type RecordingInfo = {
  ffmpegFound: boolean;
  ffmpegPath?: string;
  isRecording: boolean;
  audioDevices?: string[];
};

type RecordingTarget = {
  id: string;
  title: string;
  exeName?: string;
  processPath?: string;
  iconDataUrl?: string | null;
  x: number;
  y: number;
  w: number;
  h: number;
};

type RecordingTargets = {
  windows: RecordingTarget[];
  displays: RecordingTarget[];
};

type NativePointerState = {
  leftDown?: boolean;
  x?: number;
  y?: number;
};

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

const formatRecordingTime = (ms: number) => {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60).toString().padStart(2, "0");
  const seconds = (totalSeconds % 60).toString().padStart(2, "0");
  return `${minutes}:${seconds}`;
};

const getImageDataFromImage = (image: HTMLImageElement) => {
  const canvas = document.createElement("canvas");
  canvas.width = image.width;
  canvas.height = image.height;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("Canvas unavailable");
  ctx.drawImage(image, 0, 0);
  return ctx.getImageData(0, 0, image.width, image.height);
};

const sampledRegionDiff = (
  prev: ImageData,
  next: ImageData,
  prevStartY: number,
  nextStartY: number,
  height: number,
  sampleCols = 36,
  sampleRows = 28,
) => {
  const width = Math.min(prev.width, next.width);
  if (width <= 0 || height <= 0) return Number.POSITIVE_INFINITY;
  let total = 0;
  let count = 0;
  for (let row = 0; row < sampleRows; row += 1) {
    const yRatio = sampleRows === 1 ? 0 : row / (sampleRows - 1);
    const prevY = Math.min(prev.height - 1, Math.max(0, Math.round(prevStartY + yRatio * (height - 1))));
    const nextY = Math.min(next.height - 1, Math.max(0, Math.round(nextStartY + yRatio * (height - 1))));
    for (let col = 0; col < sampleCols; col += 1) {
      const xRatio = sampleCols === 1 ? 0 : col / (sampleCols - 1);
      const x = Math.min(width - 1, Math.max(0, Math.round(xRatio * (width - 1))));
      const prevOffset = (prevY * prev.width + x) * 4;
      const nextOffset = (nextY * next.width + x) * 4;
      total += Math.abs(prev.data[prevOffset] - next.data[nextOffset]);
      total += Math.abs(prev.data[prevOffset + 1] - next.data[nextOffset + 1]);
      total += Math.abs(prev.data[prevOffset + 2] - next.data[nextOffset + 2]);
      count += 3;
    }
  }
  return count ? total / count : Number.POSITIVE_INFINITY;
};

const findScrollOverlap = (prev: ImageData, next: ImageData) => {
  const comparableHeight = Math.min(prev.height, next.height);
  const minOverlap = Math.max(24, Math.round(comparableHeight * 0.08));
  const maxOverlap = Math.max(minOverlap, Math.round(comparableHeight * 0.58));
  const step = Math.max(4, Math.round(comparableHeight / 80));
  let bestOverlap = Math.round(comparableHeight * 0.2);
  let bestScore = Number.POSITIVE_INFINITY;
  for (let overlap = minOverlap; overlap <= maxOverlap; overlap += step) {
    const score = sampledRegionDiff(prev, next, prev.height - overlap, 0, overlap);
    if (score < bestScore) {
      bestScore = score;
      bestOverlap = overlap;
    }
  }
  return { overlap: bestOverlap, score: bestScore };
};

const isSameScrollFrame = (prev: ImageData, next: ImageData) => {
  const height = Math.min(prev.height, next.height);
  const score = sampledRegionDiff(prev, next, 0, 0, height, 40, 32);
  return score < 2;
};

const stitchScrollFrames = async (frames: string[]) => {
  if (frames.length === 0) throw new Error("未采集到滚动截图帧");
  const images = await Promise.all(frames.map((frame) => loadPngImage(frame)));
  const imageDataList = images.map(getImageDataFromImage);
  const segments: Array<{ image: HTMLImageElement; cropTop: number; drawHeight: number }> = [
    { image: images[0], cropTop: 0, drawHeight: images[0].height },
  ];

  for (let index = 1; index < images.length; index += 1) {
    const previousData = imageDataList[index - 1];
    const currentData = imageDataList[index];
    if (isSameScrollFrame(previousData, currentData)) continue;
    const { overlap, score } = findScrollOverlap(previousData, currentData);
    const fallbackOverlap = Math.round(images[index].height * 0.2);
    const cropTop = score <= 42 ? overlap : fallbackOverlap;
    const drawHeight = Math.max(1, images[index].height - cropTop);
    segments.push({ image: images[index], cropTop, drawHeight });
  }

  const width = Math.max(...segments.map(({ image }) => image.width));
  const height = segments.reduce((sum, segment) => sum + segment.drawHeight, 0);
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas unavailable");
  let offsetY = 0;
  segments.forEach(({ image, cropTop, drawHeight }) => {
    ctx.drawImage(image, 0, cropTop, image.width, drawHeight, 0, offsetY, image.width, drawHeight);
    offsetY += drawHeight;
  });
  return canvas.toDataURL("image/png").replace(/^data:image\/png;base64,/, "");
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
  const [windowRects, setWindowRects] = useState<Rect[]>([]);
  const [hoverRect, setHoverRect] = useState<Rect | null>(null);
  const [hoverCandidates, setHoverCandidates] = useState<Rect[]>([]);
  const [screenshotMode, setScreenshotMode] = useState("normal");
  const [isScrollCapturing, setIsScrollCapturing] = useState(false);
  const [scrollCaptureMode, setScrollCaptureMode] = useState<ScrollCaptureMode>("idle");
  const [scrollPreviewBase64, setScrollPreviewBase64] = useState("");
  const [recordingStatus, setRecordingStatus] = useState<RecordingStatus>("idle");
  const [recordingPickerMode, setRecordingPickerMode] = useState<"window" | "display" | null>(null);
  const [recordingFps, setRecordingFps] = useState(30);
  const [recordingResolution, setRecordingResolution] = useState("1080p");
  const [recordingAudioMode, setRecordingAudioMode] = useState("none");
  const [recordingCountdownSeconds, setRecordingCountdownSeconds] = useState(1);
  const [recordingMode, setRecordingMode] = useState<RecordingMode>("region");
  const [recordingTargets, setRecordingTargets] = useState<RecordingTargets>({ windows: [], displays: [] });
  const [selectedWindowTargetId, setSelectedWindowTargetId] = useState<string | null>(null);
  const [selectedDisplayTargetId, setSelectedDisplayTargetId] = useState<string | null>(null);
  const [recordingInfo, setRecordingInfo] = useState<RecordingInfo | null>(null);
  const [isRecordingBusy, setIsRecordingBusy] = useState(false);
  const [recordingStartedAt, setRecordingStartedAt] = useState<number | null>(null);
  const [recordingElapsedMs, setRecordingElapsedMs] = useState(0);
  const [config, setConfig] = useState<Config>({});
  const [isEditing, setIsEditing] = useState(false);
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
    cancelTextDraft, commitTextDraft, deleteSelectedAnnotation, applyAnnotations,
    replaceAnnotations, resetAnnotations
  } = useScreenshotAnnotation(() => {
    renderNeededRef.current = true;
  });
  const [dbgStatus, setDbgStatus] = useState({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });
  const [screenshotState, setScreenshotState] = useState<"initializing" | "ready" | "failed">("initializing");
  const [overlayVisible, setOverlayVisible] = useState(false);
  const overlayVisibleRef = useRef(false);
  const isScrollCapturingRef = useRef(false);
  const scrollCaptureModeRef = useRef<ScrollCaptureMode>("idle");
  const recordingPickerModeRef = useRef<"window" | "display" | null>(null);
  const scrollFramesRef = useRef<string[]>([]);
  const scrollTimerRef = useRef<number | null>(null);
  const nativePointerRecoveryTimerRef = useRef<number | null>(null);
  const nativePointerRecoveryPendingRef = useRef(false);
  const nativePointerRecoveryStartedRef = useRef(false);
  const nativePointerRecoveryInitialRef = useRef<Point | null>(null);
  const selectionStartedAtRef = useRef(0);
  const selectionCompletedAtRef = useRef(0);
  const selectionDragDistanceRef = useRef(0);
  const pendingConfirmTimerRef = useRef<number | null>(null);
  const textSourceSnapshotPromiseRef = useRef<Promise<TextSourceSnapshot | null> | null>(null);
  const isScrollFramePendingRef = useRef(false);
  const recordingStatusRef = useRef<RecordingStatus>("idle");
  const recordingSegmentsRef = useRef<string[]>([]);
  const isRecordingBusyRef = useRef(false);
  const recordingStartedAtRef = useRef<number | null>(null);
  const recordingModeRef = useRef<RecordingMode>("region");
  const recordingRegionRef = useRef<RecordingTarget | null>(null);
  const timeoutRef = useRef<any>(null);
  const captureIdRef = useRef<number>(0);
  const lastRectQueryRef = useRef(0);
  const rectQueryPendingRef = useRef(false);
  const hoverRectRef = useRef<Rect | null>(null);
  const hoverCandidatesRef = useRef<Rect[]>([]);
  const hoverCandidateIndexRef = useRef(0);
  const hoverCandidatesSignatureRef = useRef("");
  const lastMouseRef = useRef({ x: 0, y: 0 });
  const pendingDetectionRef = useRef<Rect | null>(null);
  const analysisImageDataRef = useRef<ImageData | null>(null);
  const annotationDragSnapshotRef = useRef<Annotation[] | null>(null);
  const isEditingRef = useRef(false);
  const isDrawingAnnotationRef = useRef(false);
  const isDraggingAnnotationRef = useRef(false);
  const isResizingAnnotationRef = useRef(false);
  const annotationResizeHandleRef = useRef<AnnotationResizeHandle | null>(null);
  const annotationStartRef = useRef({ x: 0, y: 0 });
  const annotationDragStartRef = useRef({ x: 0, y: 0 });

  const clearPendingConfirm = () => {
    if (pendingConfirmTimerRef.current !== null) {
      window.clearTimeout(pendingConfirmTimerRef.current);
      pendingConfirmTimerRef.current = null;
    }
  };

  const decodeTextPairs = (encoded: string) => {
    const bytes = Uint8Array.from(atob(encoded), c => c.charCodeAt(0));
    return JSON.parse(new TextDecoder().decode(bytes));
  };

  const startNewCaptureSession = (mode = "normal") => {
    clearPendingConfirm();
    captureIdRef.current += 1;
    const currentId = captureIdRef.current;
    // Clear any residual notifications from previous session
    message.destroy();

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    stopNativePointerRecovery();

    imageRef.current = null;
    translatedImgRef.current = null;
    analysisImageDataRef.current = null;
    textSourceSnapshotPromiseRef.current = null;
    setTranslatedResult(null);
    setTranslatePairs(null);
    setIsEditing(false);
    annotationSizesRef.current = { ...DEFAULT_ANNOTATION_SIZES };
    setAnnotationTool(null);
    setAnnotationColor(DEFAULT_ANNOTATION_COLOR);
    setAnnotationSizeState(DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL]);
    annotationToolRef.current = DEFAULT_ANNOTATION_TOOL;
    annotationColorRef.current = DEFAULT_ANNOTATION_COLOR;
    annotationSizeRef.current = DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL];
    setAnnotations([]);
    setRedoAnnotations([]);
    setSelectedAnnotationIndex(null);
    setEditingTextDraft(null);
    setAnnotationDraft(null);
    if (scrollTimerRef.current) {
      window.clearInterval(scrollTimerRef.current);
      scrollTimerRef.current = null;
    }
    scrollFramesRef.current = [];
    setScrollPreviewBase64("");
    scrollCaptureModeRef.current = "idle";
    setIsScrollCapturing(false);
    setScrollCaptureMode("idle");
    recordingPickerModeRef.current = null;
    setRecordingPickerMode(null);
    recordingSegmentsRef.current = [];
    recordingRegionRef.current = null;
    recordingStatusRef.current = "idle";
    setRecordingStatus("idle");
    setIsRecordingBusy(false);
    recordingStartedAtRef.current = null;
    setRecordingStartedAt(null);
    setRecordingElapsedMs(0);
    windowRectsRef.current = [];
    setWindowRects([]);
    setHoverCandidate(null);
    setHoverCandidates([]);
    nativePointerRecoveryInitialRef.current = null;
    nativePointerRecoveryStartedRef.current = false;
    selectionStartedAtRef.current = 0;
    selectionCompletedAtRef.current = 0;
    selectionDragDistanceRef.current = 0;
    hoverCandidatesRef.current = [];
    hoverCandidateIndexRef.current = 0;
    hoverCandidatesSignatureRef.current = "";
    pendingDetectionRef.current = null;
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

  const imageRef = useRef<HTMLImageElement | null>(null);
  const translatedImgRef = useRef<HTMLImageElement | null>(null);
  const hasSelectedRef = useRef(false);
  const rectRef = useRef<Rect>(EMPTY_RECT);
  const configRef = useRef<Config>({});

  const {
    isOCRing,
    isTranslating,
    translatePairs,
    translatedResult,
    translateResultPreviewBase64,
    prewarmLocalOcrWorker,
    handleOCR,
    handleTranslate,
    handleShowTranslateResult,
    resetOcrState,
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

  const windowRectsRef = useRef<Rect[]>([]);
  const screenshotModeRef = useRef("normal");
  const isSelectingRef = useRef(false);
  const isDraggingRef = useRef(false);
  const isResizingRef = useRef<string | null>(null);
  const mouseDownRef = useRef({ x: 0, y: 0 });
  const startPosRef = useRef({ x: 0, y: 0 });
  const dragStartRef = useRef({ x: 0, y: 0 });
  const resizeStartRectRef = useRef<Rect>(EMPTY_RECT);
  const maskedCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const requestRef = useRef<number | null>(null);
  const renderNeededRef = useRef(false);
  const drawRef = useRef(draw);

  hasSelectedRef.current = hasSelected;
  rectRef.current = rect;
  configRef.current = config;
  isTranslatingRef.current = isTranslating;
  isOCRingRef.current = isOCRing;
  isScrollCapturingRef.current = isScrollCapturing;
  scrollCaptureModeRef.current = scrollCaptureMode;
  recordingPickerModeRef.current = recordingPickerMode;
  recordingStatusRef.current = recordingStatus;
  isRecordingBusyRef.current = isRecordingBusy;
  recordingStartedAtRef.current = recordingStartedAt;
  recordingModeRef.current = recordingMode;
  windowRectsRef.current = windowRects;
  hoverRectRef.current = hoverRect;
  hoverCandidatesRef.current = hoverCandidates;
  annotationsRef.current = annotations;
  annotationHistoryRef.current = annotationHistory;
  redoAnnotationsRef.current = redoAnnotations;
  draftAnnotationRef.current = draftAnnotation;
  isEditingRef.current = isEditing;
  if (annotationTool) annotationToolRef.current = annotationTool;
  annotationColorRef.current = annotationColor;
  annotationSizeRef.current = annotationSize;
  selectedAnnotationIndexRef.current = selectedAnnotationIndex;
  editingTextDraftRef.current = editingTextDraft;
  screenshotModeRef.current = screenshotMode;
  drawRef.current = draw;

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

  const setHoverCandidate = (candidate: Rect | null) => {
    hoverRectRef.current = candidate;
    setHoverRect(candidate);
  };

  const setHoverCandidateList = (candidates: Rect[]) => {
    const signature = candidates.map(rectSignature).join("|");
    if (signature !== hoverCandidatesSignatureRef.current) {
      hoverCandidateIndexRef.current = 0;
      hoverCandidatesSignatureRef.current = signature;
    }
    hoverCandidatesRef.current = candidates;
    setHoverCandidates(candidates);
    const nextIndex = candidates.length === 0 ? 0 : hoverCandidateIndexRef.current % candidates.length;
    hoverCandidateIndexRef.current = nextIndex;
    setHoverCandidate(candidates[nextIndex] || null);
  };

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
      return buildTextSourceBlocksForPhysicalSelection([], undefined, { x: 0, y: 0, w: 0, h: 0 });
    }
    let physicalSelection: Rect;
    try {
      physicalSelection = getPhysicalSelection({
        canvas: canvasRef.current,
        image: imageRef.current as any,
        rect: selection,
      });
    } catch {
      return buildTextSourceBlocksForPhysicalSelection([], undefined, { x: 0, y: 0, w: 0, h: 0 });
    }
    return buildTextSourceBlocksForPhysicalSelection(snapshot.elements, snapshot.screen, physicalSelection);
  };

  const getTextSourceBlocksForCurrentSelection = async (timeoutMs = 80) => {
    const started = performance.now();
    const snapshot = await Promise.race([
      textSourceSnapshotPromiseRef.current || readTextSourceSnapshot(timeoutMs),
      sleep(timeoutMs).then(() => null),
    ]);
    const textSourceSelection = buildTextSourceBlocksForSelection(snapshot, rectRef.current);
    const blocks = textSourceSelection.blocks;
    const charCount = blocks.reduce((sum, block) => sum + block.text.length, 0);
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

  const waitForStableViewport = async (img: HTMLImageElement) => {
    let lastW = 0;
    let lastH = 0;
    for (let i = 0; i < 3; i++) {
      await nextFrame();
      const w = window.innerWidth;
      const h = window.innerHeight;
      const largeEnough = w >= Math.min(img.naturalWidth, screen.width) * 0.6 && h >= Math.min(img.naturalHeight, screen.height) * 0.6;
      if (w === lastW && h === lastH && largeEnough) return;
      lastW = w;
      lastH = h;
    }
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
    } catch {
    }
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
    isSelectingRef.current = true;
    setIsSelecting(true);
    setHoverCandidate(null);
    setCurrentRect({ x: cx, y: cy, w: 0, h: 0 }, true);
    setSelection(false);
    nativePointerRecoveryStartedRef.current = true;
    renderNeededRef.current = true;
    return true;
  };

  const stopNativePointerRecovery = () => {
    if (nativePointerRecoveryTimerRef.current !== null) {
      window.clearInterval(nativePointerRecoveryTimerRef.current);
      nativePointerRecoveryTimerRef.current = null;
    }
    nativePointerRecoveryPendingRef.current = false;
    nativePointerRecoveryInitialRef.current = null;
  };

  const tryRecoverNativePointerDown = async () => {
    if (nativePointerRecoveryPendingRef.current || nativePointerRecoveryStartedRef.current) return;
    if (!overlayVisibleRef.current || hasSelectedRef.current || isSelectingRef.current) return;
    nativePointerRecoveryPendingRef.current = true;
    try {
      const state = await invoke<NativePointerState>("get_screenshot_pointer_state", { label: getCurrentWindow().label });
      const x = Math.round(Number(state?.x ?? -1));
      const y = Math.round(Number(state?.y ?? -1));
      const inBounds = x >= 0 && y >= 0 && x <= window.innerWidth && y <= window.innerHeight;
      if (state?.leftDown && inBounds) {
        const initial = nativePointerRecoveryInitialRef.current;
        if (!initial) {
          nativePointerRecoveryInitialRef.current = { x, y };
          return;
        }
        const moved = Math.hypot(x - initial.x, y - initial.y);
        if (moved >= MIN_NATIVE_RECOVERY_DRAG_PX && startPlainSelectionAt(initial.x, initial.y)) {
          selectionDragDistanceRef.current = moved;
          setCurrentRect({ x: Math.min(initial.x, x), y: Math.min(initial.y, y), w: Math.abs(initial.x - x), h: Math.abs(initial.y - y) }, true);
          renderNeededRef.current = true;
          stopNativePointerRecovery();
        }
      } else {
        nativePointerRecoveryInitialRef.current = null;
      }
    } catch {
    } finally {
      nativePointerRecoveryPendingRef.current = false;
    }
  };

  const startNativePointerRecovery = () => {
    stopNativePointerRecovery();
    nativePointerRecoveryStartedRef.current = false;
    nativePointerRecoveryInitialRef.current = null;
    const deadline = performance.now() + 900;
    nativePointerRecoveryTimerRef.current = window.setInterval(() => {
      if (!overlayVisibleRef.current || performance.now() > deadline || nativePointerRecoveryStartedRef.current) {
        stopNativePointerRecovery();
        return;
      }
      tryRecoverNativePointerDown();
    }, 16);
    tryRecoverNativePointerDown();
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
        recordingModeRef.current = "region";
      }
    })
      .then((unsub) => { unlistenMode = unsub; })
      .catch(() => {});

    listen("recording-ended", () => {
      recordingSegmentsRef.current = [];
      recordingStartedAtRef.current = null;
      recordingRegionRef.current = null;
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);
      recordingStatusRef.current = "idle";
      setRecordingStatus("idle");
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
      clearPendingConfirm();
      stopNativePointerRecovery();
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

  useEffect(() => {
    if (recordingStatus !== "recording" || !recordingStartedAt) return;
    const updateElapsed = () => setRecordingElapsedMs(Date.now() - recordingStartedAt);
    updateElapsed();
    const timer = window.setInterval(updateElapsed, 500);
    return () => window.clearInterval(timer);
  }, [recordingStatus, recordingStartedAt]);

  const loadConfig = async () => {
    try {
      const raw = await invoke<string>("get_config");
      console.log("[ScreenshotPage] loadConfig raw string length:", raw.length, "content:", raw);
      const parsedConfig = JSON.parse(raw);
      console.log("[ScreenshotPage] loadConfig parsed config:", parsedConfig);
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

  const loadWindowRects = async (force = false) => {
    const now = performance.now();
    if (!force && (rectQueryPendingRef.current || now - lastRectQueryRef.current < 50)) return;
    lastRectQueryRef.current = now;
    rectQueryPendingRef.current = true;
    try {
      const includeControls = Boolean(configRef.current.enableUiControlDetection);
      const nextRects = JSON.parse(await invoke<string>("get_window_rects", { includeControls }));
      windowRectsRef.current = nextRects;
      setWindowRects(nextRects);
      if (!hasSelectedRef.current && !isSelectingRef.current && !isDraggingRef.current && !isResizingRef.current) {
        setHoverCandidateList(getDetectionCandidatesAt(lastMouseRef.current.x, lastMouseRef.current.y, windowRectsRef.current, analysisImageDataRef.current, configRef.current.enableVisualDetection === true, configRef.current.visualDetectionSensitivity || 3));
      }
      renderNeededRef.current = true;
    } catch {
      setWindowRects([]);
    } finally {
      rectQueryPendingRef.current = false;
    }
  };

  const loadFullscreen = async (mode = screenshotModeRef.current || "normal") => {
    const sessionId = startNewCaptureSession(mode);
    try {
      loadWindowRects(true);
      const base64 = await invoke<string>("get_fullscreen_image");
      if (sessionId !== captureIdRef.current) return;

      if (!base64 || base64.length < 1000) {
        console.warn("[ScreenshotPage] Stale or invalid base64 ignored during get_fullscreen_image", base64?.length || 0);
        return;
      }

      loadImageFromBase64(base64, sessionId);
    } catch (err: any) {
      if (sessionId !== captureIdRef.current) return;
      const msg = err?.message || err?.toString?.() || String(err);
      console.error("[ScreenshotPage] loadFullscreen failed:", msg);
      cancelScreenshot();
    }
  };

  const loadFullscreenFromBase64 = (base64: string, mode = "normal") => {
    const sessionId = startNewCaptureSession(mode);
    try {
      if (!base64 || base64.length < 1000) {
        console.warn("[ScreenshotPage] Stale or invalid base64 event payload ignored", base64?.length || 0);
        return;
      }

      loadWindowRects(true);
      loadImageFromBase64(base64, sessionId);
    } catch (err: any) {
      if (sessionId !== captureIdRef.current) return;
      const msg = err?.message || err?.toString?.() || String(err);
      console.error("[ScreenshotPage] loadFullscreenFromBase64 failed:", msg);
      cancelScreenshot();
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
      const msg = err?.message || err?.toString?.() || String(err);
      console.error("[ScreenshotPage] loadFullscreenFromFile failed:", msg);
      loadFullscreen(mode);
    }
  };

  const loadImageFromBase64 = (base64: string, sessionId: number) => {
    if (sessionId !== captureIdRef.current) return;

    if (!base64 || base64.length < 1000) {
      console.warn("[ScreenshotPage] loadImageFromBase64 invalid payload", base64?.length || 0);
      return;
    }

    const dataUrl = "data:image/png;base64," + base64;
    loadImageFromSource(dataUrl, sessionId, Math.round(base64.length * 0.75));
  };

  const loadImageFromSource = (source: string, sessionId: number, bytes?: number) => {
    if (sessionId !== captureIdRef.current) return;

    const img = new Image();
    img.crossOrigin = "anonymous";

    // Start a 1500ms fallback safety timer
    timeoutRef.current = setTimeout(() => {
      if (sessionId !== captureIdRef.current) return;
      if (imageRef.current === null) {
        console.warn("[ScreenshotPage] Screenshot loading timeout reached (1500ms)");
        cancelScreenshot();
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
      initCanvas(img);

      requestAnimationFrame(() => {
        if (sessionId !== captureIdRef.current) return;
        overlayVisibleRef.current = true;
        setOverlayVisible(true);
        focusScreenshotWindow();
        invoke("overlay_ready_to_show", { label: getCurrentWindow().label })
          .catch((err) => {
            console.error("[ScreenshotPage] overlay_ready_to_show failed:", err);
          })
          .finally(() => {
            focusScreenshotWindow();
            window.setTimeout(focusScreenshotWindow, 60);
          });
        startNativePointerRecovery();
      });
    };

    img.onerror = () => {
      if (sessionId !== captureIdRef.current) return;
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      console.warn("[ScreenshotPage] image decode failed", sessionId, source.length);
      cancelScreenshot();
    };
    img.src = source;
  };

  const initCanvas = (img: HTMLImageElement) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const width = Math.max(1, window.innerWidth);
    const height = Math.max(1, window.innerHeight);
    canvas.width = width;
    canvas.height = height;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;

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

  const snap = (val: number, refs: number[]) => {
    const dist = 15;
    for (const r of refs) if (Math.abs(val - r) < dist) return r;
    return val;
  };

  const getDetectionRectAt = (mx: number, my: number) => {
    const candidates = getDetectionCandidatesAt(
      mx,
      my,
      windowRectsRef.current,
      analysisImageDataRef.current,
      configRef.current.enableVisualDetection === true,
      configRef.current.visualDetectionSensitivity || 3,
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
      && !nativePointerRecoveryStartedRef.current
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
        isSelectingRef.current = true;
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

  const getCurrentPhysicalSelection = () => getPhysicalSelection({
    canvas: canvasRef.current,
    image: imageRef.current as any,
    rect: rectRef.current,
  });

  const getCurrentAbsoluteSelection = async (): Promise<RecordingTarget> => {
    const selection = getCurrentPhysicalSelection();
    const origin = await getCurrentWindow().outerPosition().catch(() => ({ x: 0, y: 0 }));
    return {
      id: "region",
      title: "翻译结果",
      x: Math.round(origin.x + selection.x),
      y: Math.round(origin.y + selection.y),
      w: Math.round(selection.w),
      h: Math.round(selection.h),
    };
  };

  const rectFromAbsoluteTarget = async (target: RecordingTarget) => {
    const canvas = canvasRef.current;
    const image = imageRef.current;
    const origin = await getCurrentWindow().outerPosition().catch(() => ({ x: 0, y: 0 }));
    const scaleX = image && canvas ? image.naturalWidth / canvas.width : 1;
    const scaleY = image && canvas ? image.naturalHeight / canvas.height : 1;
    return {
      x: Math.round((target.x - origin.x) / scaleX),
      y: Math.round((target.y - origin.y) / scaleY),
      w: Math.round(target.w / scaleX),
      h: Math.round(target.h / scaleY),
      kind: "window",
    } as Rect;
  };


  const cropCurrentSelectionFromLoadedImage = () => cropSelectionFromLoadedImage({
    canvas: canvasRef.current,
    image: imageRef.current as any,
    rect: rectRef.current,
  });

  const captureRegionBase64 = async () => {
    const { x, y, w, h } = getCurrentPhysicalSelection();
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
  const isLikelySystemAudioDevice = (device: string) => /wasapi:|stereo mix|立体声|混音|loopback|virtual audio|output|speaker|扬声器/i.test(device);
  const isLikelyMicrophoneDevice = (device: string) => !isLikelySystemAudioDevice(device);

  const formatAudioDeviceLabel = (device: string) => {
    if (device === "wasapi:default") return "系统声音（默认输出）";
    if (device.startsWith("wasapi:")) return `系统声音：${device.slice("wasapi:".length)}`;
    if (device.startsWith("dshow:")) return device.slice("dshow:".length);
    return device;
  };

  const getRecordingDevices = () => {
    const devices = recordingInfo?.audioDevices || [];
    return {
      mic: devices.find(isLikelyMicrophoneDevice) || null,
      system: devices.find(isLikelySystemAudioDevice) || null,
      hasSystem: devices.some(isLikelySystemAudioDevice),
    };
  };

  const loadRecordingPrerequisites = async () => {
    const [info, targets] = await Promise.all([
      invoke<RecordingInfo>("get_recording_info"),
      invoke<RecordingTargets>("get_recording_targets").catch(() => ({ windows: [], displays: [] })),
    ]);
    setRecordingInfo(info);
    setRecordingTargets(targets);
    if (!selectedWindowTargetId && targets.windows.length > 0) setSelectedWindowTargetId(targets.windows[0].id);
    if (!selectedDisplayTargetId && targets.displays.length > 0) setSelectedDisplayTargetId(targets.displays[0].id);
    if (!info.ffmpegFound) {
      throw new Error("未找到 ffmpeg.exe，请先在模型/视频配置里下载或选择 FFmpeg。");
    }
    return { info, targets };
  };

  const applyRecordingTarget = async (target: RecordingTarget) => {
    recordingRegionRef.current = target;
    const nextRect = await rectFromAbsoluteTarget(target);
    setCurrentRect(nextRect, true);
    setSelection(true);
    setHoverCandidate(null);
    renderNeededRef.current = true;
  };

  const enterRecordingMode = async (mode: RecordingMode = "region") => {
    if (isTranslatingRef.current || isOCRingRef.current || isScrollCapturingRef.current || isRecordingBusyRef.current) return;
    try {
      setRecordingMode(mode);
      recordingModeRef.current = mode;
      const { targets } = await loadRecordingPrerequisites();
      recordingSegmentsRef.current = [];
      recordingStartedAtRef.current = null;
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);

      if (mode === "region") {
        if (!hasSelectedRef.current) {
          if (screenshotModeRef.current !== "record") {
            setScreenshotMode("record");
            screenshotModeRef.current = "record";
            setCurrentRect(EMPTY_RECT, true);
            setSelection(false);
          }
          message.info("Please select a recording area first");
          renderNeededRef.current = true;
          return;
        }
      } else if (mode === "window") {
        const target = targets.windows.find((item) => item.id === selectedWindowTargetId) || targets.windows[0];
        if (!target) throw new Error("No recordable window detected");
        setSelectedWindowTargetId(target.id);
        await applyRecordingTarget(target);
        recordingPickerModeRef.current = "window";
        setRecordingPickerMode("window");
        message.info("请选择要录制的窗口，蓝框确认无误后点击确认。");
        return;
      } else {
        const target = targets.displays.find((item) => item.id === selectedDisplayTargetId) || targets.displays[0];
        if (!target) throw new Error("No display detected");
        setSelectedDisplayTargetId(target.id);
        await applyRecordingTarget(target);
        recordingPickerModeRef.current = "display";
        setRecordingPickerMode("display");
        message.info("请选择要录制的显示器，蓝框确认无误后点击确认。");
        return;
      }

      await startRecording();
    } catch (error: any) {
      recordingStatusRef.current = "idle";
      setRecordingStatus("idle");
      message.error(`Failed to enter recording mode: ${error?.message || error}`);
    }
  };

  const cancelRecordingTargetPicker = () => {
    recordingPickerModeRef.current = null;
    setRecordingPickerMode(null);
    recordingRegionRef.current = null;
    setRecordingMode("region");
    recordingModeRef.current = "region";
    message.destroy();
    if (!screenshotModeRef.current || screenshotModeRef.current === "normal") {
      setCurrentRect(EMPTY_RECT, true);
      setSelection(false);
    }
    renderNeededRef.current = true;
  };

  const confirmRecordingTargetPicker = async () => {
    if (!recordingPickerModeRef.current) return;
    recordingPickerModeRef.current = null;
    setRecordingPickerMode(null);
    await startRecording();
  };

  const selectRecordingTarget = async (mode: "window" | "display", targetId: string) => {
    const list = mode === "window" ? recordingTargets.windows : recordingTargets.displays;
    const target = list.find((item) => item.id === targetId);
    if (!target) return;
    if (mode === "window") setSelectedWindowTargetId(targetId);
    if (mode === "display") setSelectedDisplayTargetId(targetId);
    await applyRecordingTarget(target);
  };

  const buildRecordingOptions = async () => {
    const devices = getRecordingDevices();
    if ((recordingAudioMode === "system" || recordingAudioMode === "system_mic") && !devices.system) throw new Error("当前未检测到系统声音设备");
    if ((recordingAudioMode === "mic" || recordingAudioMode === "system_mic") && !devices.mic) throw new Error("当前未检测到麦克风设备");
    const region = recordingModeRef.current === "region" ? await getCurrentAbsoluteSelection() : recordingRegionRef.current;
    if (!region || region.w <= 0 || region.h <= 0) throw new Error("请先选择有效录制区域");
    return {
      fps: recordingFps,
      resolution: recordingResolution,
      audio_mode: recordingAudioMode,
      mic_device: devices.mic,
      system_audio_device: devices.system,
      output_dir: null,
      region_x: Math.round(region.x),
      region_y: Math.round(region.y),
      region_w: Math.round(region.w),
      region_h: Math.round(region.h),
    };
  };

  const startRecording = async () => {
    if (isRecordingBusyRef.current) return;
    try {
      const active = await invoke<boolean>('is_recording_active').catch(() => false);
      if (active) {
        message.error('当前已有录像正在进行，请先停止');
        return;
      }
      setIsRecordingBusy(true);
      const options = await buildRecordingOptions();
      const normalizedOptions = { ...options, fps: 30, resolution: "1080p", output_dir: null };
      const region = { x: normalizedOptions.region_x, y: normalizedOptions.region_y, w: normalizedOptions.region_w, h: normalizedOptions.region_h };
      recordingPickerModeRef.current = null;
      setRecordingPickerMode(null);
      recordingStatusRef.current = "ready";
      setRecordingStatus("ready");
      console.log("[screenshot-trace] startRecording: openRecordingWindows before, recordingStatus=", recordingStatusRef.current, "hasSelected=", hasSelectedRef.current, "shouldCloseScreenshot=", true);
      await openRecordingWindows({
        options: normalizedOptions,
        countdownSeconds: 0,
        autoStart: false,
      }, region);
      console.log("[screenshot-trace] startRecording: openRecordingWindows after");
      const win = getCurrentWindow();
      console.log("[screenshot-trace] startRecording: closing screenshot window before");
      await win.setAlwaysOnTop(false).catch(() => {});
      await win.hide().catch(() => {});
      console.log("[screenshot-trace] startRecording: closing screenshot window after");
      await invoke('set_capturing_state', { state: false }).catch(() => {});
    } catch (error: any) {
      recordingStatusRef.current = "idle";
      setRecordingStatus("idle");
      message.error("Failed to open recording controls: " + (error?.message || error));
    } finally {
      setIsRecordingBusy(false);
    }
  };

  const finishRecording = async () => {
    if (recordingStatusRef.current === "ready") {
      await startRecording();
      return;
    }
    if (isRecordingBusyRef.current || recordingStatusRef.current === "idle") return;
    try {
      setIsRecordingBusy(true);
      if (recordingStatusRef.current === "recording") await invoke("stop_recording");
      const segments = [...recordingSegmentsRef.current];
      if (segments.length === 0) throw new Error("没有可保存的录屏片段");
      const win = getCurrentWindow();
      await win.setAlwaysOnTop(false).catch(() => {});
      await win.hide().catch(() => {});
      await invoke('set_capturing_state', { state: false }).catch(() => {});
      const savedPath = await invoke<string>("concat_recording_segments", { segmentPaths: segments });
      await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      recordingSegmentsRef.current = [];
      recordingStartedAtRef.current = null;
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);
      recordingStatusRef.current = "idle";
      setRecordingStatus("idle");
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
      message.success(`录屏已保存：${savedPath}`);
      renderNeededRef.current = true;
    } catch (error: any) {
      message.error("完成录屏失败：" + (error?.message || error));
      if (recordingStatusRef.current !== "idle") {
        recordingStatusRef.current = "recording";
        setRecordingStatus("recording");
      }
      await getCurrentWindow().show().catch(() => {});
    } finally {
      setIsRecordingBusy(false);
    }
  };

  const cancelRecording = async () => {
    if (recordingStatusRef.current === "idle" && recordingSegmentsRef.current.length === 0) return;
    const segments = [...recordingSegmentsRef.current];
    try {
      setIsRecordingBusy(true);
      await invoke("cancel_recording_process").catch(() => {});
      await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      message.info("已取消录屏并清理临时片段");
    } finally {
      recordingSegmentsRef.current = [];
      recordingStartedAtRef.current = null;
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);
      recordingStatusRef.current = "idle";
      setRecordingStatus("idle");
      setIsRecordingBusy(false);
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
      renderNeededRef.current = true;
    }
  };

  const captureManualScrollFrame = async () => {
    if (isScrollFramePendingRef.current || scrollCaptureModeRef.current !== "capturing") return;
    const selection = getCurrentPhysicalSelection();
    if (selection.w <= 0 || selection.h <= 0) return;
    try {
      isScrollFramePendingRef.current = true;
      const frame = await invoke<string>("capture_live_region", {
        x: Math.round(selection.x),
        y: Math.round(selection.y),
        w: Math.round(selection.w),
        h: Math.round(selection.h),
      });
      const frames = scrollFramesRef.current;
      if (frames.length === 0) {
        scrollFramesRef.current = [frame];
        setScrollPreviewBase64(frame);
      } else {
        const [prev, next] = await Promise.all([loadPngImage(frames[frames.length - 1]), loadPngImage(frame)]);
        const diff = sampledRegionDiff(getImageDataFromImage(prev), getImageDataFromImage(next), 0, 0, Math.min(prev.height, next.height), 24, 18);
        if (diff > 1.2) {
          scrollFramesRef.current = [...frames, frame];
          setScrollPreviewBase64(frame);
        }
      }
      message.loading({ content: `手动滚动采集中，已采集 ${scrollFramesRef.current.length} 帧`, key: "scroll-shot", duration: 0 });
      if (scrollFramesRef.current.length >= 30) await finishManualScrollCapture();
    } catch (error: any) {
      message.error({ content: `采集滚动帧失败：${error?.message || error}`, key: "scroll-shot", duration: 3 });
    } finally {
      isScrollFramePendingRef.current = false;
    }
  };

  const handleScrollCapture = () => {
    if (!hasSelectedRef.current || isScrollCapturingRef.current || isTranslatingRef.current || isOCRingRef.current || recordingStatusRef.current !== "idle") return;
    scrollCaptureModeRef.current = "ready";
    setScrollCaptureMode("ready");
    setScrollPreviewBase64("");
    message.info("已进入滚动截图模式，请点击“开始采集”后手动滚动目标窗口。 ");
    renderNeededRef.current = true;
  };

  const scrollSelectedRegionDown = async () => {
    const selection = getCurrentPhysicalSelection();
    if (selection.w <= 0 || selection.h <= 0) return;
    await invoke("scroll_mouse_at", {
      x: Math.round(selection.x + selection.w / 2),
      y: Math.round(selection.y + selection.h / 2),
      delta: -520,
    }).catch(() => {});
  };

  const startManualScrollCapture = async () => {
    if (scrollCaptureModeRef.current !== "ready") return;
    scrollFramesRef.current = [];
    setScrollPreviewBase64("");
    scrollCaptureModeRef.current = "capturing";
    setScrollCaptureMode("capturing");
    setIsScrollCapturing(true);
    await invoke("set_window_capture_excluded", { label: getCurrentWindow().label, excluded: true }).catch(() => {});
    message.loading({ content: "手动滚动采集中，请自己滚动目标窗口...", key: "scroll-shot", duration: 0 });
    await captureManualScrollFrame();
    await scrollSelectedRegionDown();
    scrollTimerRef.current = window.setInterval(async () => {
      await captureManualScrollFrame();
      await scrollSelectedRegionDown();
    }, 760);
    renderNeededRef.current = true;
  };

  const finishManualScrollCapture = async () => {
    if (scrollTimerRef.current) {
      window.clearInterval(scrollTimerRef.current);
      scrollTimerRef.current = null;
    }
    try {
      const frames = [...scrollFramesRef.current];
      if (frames.length === 0) throw new Error("还没有采集到滚动截图帧");
      const stitched = frames.length === 1 ? frames[0] : await stitchScrollFrames(frames);
      await invoke("copy_image_to_clipboard", { imageBase64: stitched });
      await emit("screenshot-captured", stitched);
      message.success({ content: `滚动截图已复制，共 ${frames.length} 帧`, key: "scroll-shot", duration: 3 });
    } catch (error: any) {
      message.error({ content: `滚动截图失败：${error?.message || error}`, key: "scroll-shot", duration: 4 });
    } finally {
      await invoke("set_window_capture_excluded", { label: getCurrentWindow().label, excluded: false }).catch(() => {});
      scrollFramesRef.current = [];
      setScrollPreviewBase64("");
      setIsScrollCapturing(false);
      scrollCaptureModeRef.current = "idle";
      setScrollCaptureMode("idle");
      renderNeededRef.current = true;
    }
  };

  const cancelManualScrollCapture = () => {
    if (scrollTimerRef.current) {
      window.clearInterval(scrollTimerRef.current);
      scrollTimerRef.current = null;
    }
    scrollFramesRef.current = [];
    setScrollPreviewBase64("");
    setIsScrollCapturing(false);
    scrollCaptureModeRef.current = "idle";
    setScrollCaptureMode("idle");
    message.destroy("scroll-shot");
    message.info("已取消滚动截图");
    renderNeededRef.current = true;
  };

  const resetScreenshotState = () => {
    clearPendingConfirm();
    stopNativePointerRecovery();
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setRect(EMPTY_RECT);
    setHasSelected(false);
    setTranslatedResult(null);
    setTranslatePairs(null);
    setIsEditing(false);
    annotationSizesRef.current = { ...DEFAULT_ANNOTATION_SIZES };
    setAnnotationTool(null);
    setAnnotationColor(DEFAULT_ANNOTATION_COLOR);
    setAnnotationSizeState(DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL]);
    annotationToolRef.current = DEFAULT_ANNOTATION_TOOL;
    annotationColorRef.current = DEFAULT_ANNOTATION_COLOR;
    annotationSizeRef.current = DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL];
    setAnnotations([]);
    setAnnotationHistory([]);
    setRedoAnnotations([]);
    annotationsRef.current = [];
    annotationHistoryRef.current = [];
    redoAnnotationsRef.current = [];
    setSelectedAnnotationIndex(null);
    setEditingTextDraft(null);
    setAnnotationDraft(null);
    if (scrollTimerRef.current) {
      window.clearInterval(scrollTimerRef.current);
      scrollTimerRef.current = null;
    }
    scrollFramesRef.current = [];
    setScrollPreviewBase64("");
    scrollCaptureModeRef.current = "idle";
    setScrollCaptureMode("idle");
    invoke("set_window_capture_excluded", { label: getCurrentWindow().label, excluded: false }).catch(() => {});
    recordingPickerModeRef.current = null;
    setRecordingPickerMode(null);
    recordingSegmentsRef.current = [];
    recordingRegionRef.current = null;
    recordingStatusRef.current = "idle";
    setRecordingStatus("idle");
    setIsRecordingBusy(false);
    windowRectsRef.current = [];
    setWindowRects([]);
    setHoverCandidate(null);
    pendingDetectionRef.current = null;
    nativePointerRecoveryInitialRef.current = null;
    nativePointerRecoveryStartedRef.current = false;
    selectionStartedAtRef.current = 0;
    selectionCompletedAtRef.current = 0;
    selectionDragDistanceRef.current = 0;
    isTranslatingRef.current = (false);
    isOCRingRef.current = (false);
    setIsScrollCapturing(false);
    setScreenshotMode("normal");
    screenshotModeRef.current = "normal";
    setScreenshotState("initializing");
    setOverlayVisible(false);
    setDbgStatus({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });
    imageRef.current = null;
    translatedImgRef.current = null;
    analysisImageDataRef.current = null;
  };

  const cancelScreenshot = async () => {
    resetScreenshotState();
    await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
  };

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
      clearPendingConfirm();
      pendingConfirmTimerRef.current = window.setTimeout(() => {
        pendingConfirmTimerRef.current = null;
        confirmScreenshot(action);
      }, confirmDelayMs + 16);
      return;
    }
    clearPendingConfirm();
    try {
      const base64 = await getOutputBase64();
      await emit("screenshot-captured", base64);
      if (action === "copy" || action === "both") {
        await invoke("copy_image_to_clipboard", { imageBase64: base64 });
      }
      // Clear messages and close overlay first
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

  const currentToolbarStyle = getActionToolbarStyle({ rect, toolbarSize: actionToolbarSize, fallbackSize: recordingStatus !== "idle" || recordingPickerMode || scrollCaptureMode !== "idle" ? RECORDING_TOOLBAR_FALLBACK_SIZE : ACTION_TOOLBAR_FALLBACK_SIZE, viewportWidth: window.innerWidth, viewportHeight: window.innerHeight, margin: FLOATING_PANEL_MARGIN, gap: FLOATING_PANEL_GAP });
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
          onScrollCapture={handleScrollCapture}
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

