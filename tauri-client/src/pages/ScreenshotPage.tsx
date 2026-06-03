import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { Button, Space, message } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ScreenshotToolbar from "../components/screenshot/ScreenshotToolbar";
import TextAnnotationEditor from "../components/screenshot/TextAnnotationEditor";
import TranslationLoadingOverlay from "../components/screenshot/TranslationLoadingOverlay";
import type { Annotation, AnnotationTool, EditingTextDraft, OcrBlock, Point, Rect, TranslatePair } from "../types/screenshot";
import { clamp, hitAnnotationDetailed, isDraggableAnnotation, makeLineAnnotation, makeTextAnnotation, moveAnnotation, normalizedRectFromPoints, resizeAnnotation, type AnnotationResizeHandle } from "../utils/annotationGeometry";
import { cropSelectionFromLoadedImage, getPhysicalSelection, loadPngImage, renderEditedSelectionBase64 } from "../utils/screenshotImage";
import { openOcrResultWindow } from "../utils/ocrResultWindow";
import { getActionToolbarStyle } from "../utils/screenshotLayout";
import { getHandleAt, isPointInSelection } from "../utils/selectionGeometry";
import { renderTranslatedBlocks } from "../utils/translatedBlocks";
import { openPinWindow } from "../utils/pinWindows";
import { getDetectionCandidatesAt, rectSignature } from "../utils/detectionCandidates";
import { translateWithLocalOcr } from "../utils/localOcrTranslate";
import { renderScreenshotCanvas } from "../utils/renderScreenshotCanvas";
import { openRecordingWindows } from "../utils/recordingWindows";
import { buildOcrNormalizationReport } from "../ocr-processing";
import RecordingTargetPicker from "../components/recording/RecordingTargetPicker";

interface Config {
  serverUrl?: string;
  clientToken?: string;
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  localOcrTimeoutMs?: number;
  targetLang?: string;
  channel?: string;
  enableUiControlDetection?: boolean;
  enableVisualDetection?: boolean;
  detectionBorderWidth?: number;
  toolbarButtonGap?: number;
  visualDetectionSensitivity?: number;
}

const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };
const ACTION_TOOLBAR_FALLBACK_SIZE = { width: 680, height: 86 };
const OCR_WINDOW_SIZE = { width: 460, height: 360 };
const FLOATING_PANEL_MARGIN = 8;
const FLOATING_PANEL_GAP = 8;
const DEFAULT_ANNOTATION_COLOR = "#ff4d4f";
const DEFAULT_ANNOTATION_TOOL: AnnotationTool = "rect";
const DEFAULT_ANNOTATION_SIZES: Record<AnnotationTool, number> = { rect: 4, circle: 4, mosaic: 16, arrow: 4, text: 4, brush: 4 };
const RECORDING_BORDER_COLOR = "#ef4444";
const SCROLL_CAPTURE_BORDER_COLOR = "#f97316";
const RECORDING_TOOLBAR_FALLBACK_SIZE = { width: 980, height: 96 };

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
  const [isSelecting, setIsSelecting] = useState(false);
  const [rect, setRect] = useState<Rect>(EMPTY_RECT);
  const [actionToolbarSize, setActionToolbarSize] = useState(ACTION_TOOLBAR_FALLBACK_SIZE);
  const [hasSelected, setHasSelected] = useState(false);
  const [windowRects, setWindowRects] = useState<Rect[]>([]);
  const [hoverRect, setHoverRect] = useState<Rect | null>(null);
  const [hoverCandidates, setHoverCandidates] = useState<Rect[]>([]);
  const [screenshotMode, setScreenshotMode] = useState("normal");
  const [isTranslating, setIsTranslating] = useState(false);
  const [isOCRing, setIsOCRing] = useState(false);
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
  const [translatedResult, setTranslatedResult] = useState<string | null>(null);
  const [translatePairs, setTranslatePairs] = useState<TranslatePair[] | null>(null);
  const [translateResultPreviewBase64, setTranslateResultPreviewBase64] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [annotationTool, setAnnotationToolState] = useState<AnnotationTool | null>(null);
  const [annotationColor, setAnnotationColor] = useState(DEFAULT_ANNOTATION_COLOR);
  const [annotationSize, setAnnotationSizeState] = useState(DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL]);
  const [selectedAnnotationIndex, setSelectedAnnotationIndex] = useState<number | null>(null);
  const [editingTextDraft, setEditingTextDraft] = useState<EditingTextDraft>(null);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [annotationHistory, setAnnotationHistory] = useState<Annotation[][]>([]);
  const [redoAnnotations, setRedoAnnotations] = useState<Annotation[][]>([]);
  const [draftAnnotation, setDraftAnnotation] = useState<Annotation | null>(null);
  const [dbgStatus, setDbgStatus] = useState({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });
  const [screenshotState, setScreenshotState] = useState<"initializing" | "ready" | "failed">("initializing");
  const [overlayVisible, setOverlayVisible] = useState(false);
  const overlayVisibleRef = useRef(false);
  const isTranslatingRef = useRef(false);
  const isOCRingRef = useRef(false);
  const isScrollCapturingRef = useRef(false);
  const scrollCaptureModeRef = useRef<ScrollCaptureMode>("idle");
  const recordingPickerModeRef = useRef<"window" | "display" | null>(null);
  const scrollFramesRef = useRef<string[]>([]);
  const scrollTimerRef = useRef<number | null>(null);
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
  const annotationsRef = useRef<Annotation[]>([]);
  const annotationHistoryRef = useRef<Annotation[][]>([]);
  const redoAnnotationsRef = useRef<Annotation[][]>([]);
  const draftAnnotationRef = useRef<Annotation | null>(null);
  const annotationDragSnapshotRef = useRef<Annotation[] | null>(null);
  const isEditingRef = useRef(false);
  const annotationToolRef = useRef<AnnotationTool>(DEFAULT_ANNOTATION_TOOL);
  const annotationColorRef = useRef(DEFAULT_ANNOTATION_COLOR);
  const annotationSizeRef = useRef(DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL]);
  const annotationSizesRef = useRef<Record<AnnotationTool, number>>({ ...DEFAULT_ANNOTATION_SIZES });
  const selectedAnnotationIndexRef = useRef<number | null>(null);
  const editingTextDraftRef = useRef<EditingTextDraft>(null);
  const isDrawingAnnotationRef = useRef(false);
  const isDraggingAnnotationRef = useRef(false);
  const isResizingAnnotationRef = useRef(false);
  const annotationResizeHandleRef = useRef<AnnotationResizeHandle | null>(null);
  const annotationStartRef = useRef({ x: 0, y: 0 });
  const annotationDragStartRef = useRef({ x: 0, y: 0 });

  const decodeTextPairs = (encoded: string) => {
    const bytes = Uint8Array.from(atob(encoded), c => c.charCodeAt(0));
    return JSON.parse(new TextDecoder().decode(bytes));
  };

  const startNewCaptureSession = () => {
    captureIdRef.current += 1;
    const currentId = captureIdRef.current;
    // Clear any residual notifications from previous session
    message.destroy();

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    imageRef.current = null;
    translatedImgRef.current = null;
    analysisImageDataRef.current = null;
    setTranslatedResult(null);
    setTranslatePairs(null);
    setTranslateResultPreviewBase64("");
    setIsEditing(false);
    annotationSizesRef.current = { ...DEFAULT_ANNOTATION_SIZES };
    setAnnotationToolState(null);
    setAnnotationColor(DEFAULT_ANNOTATION_COLOR);
    setAnnotationSizeState(DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL]);
    annotationToolRef.current = DEFAULT_ANNOTATION_TOOL;
    annotationColorRef.current = DEFAULT_ANNOTATION_COLOR;
    annotationSizeRef.current = DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL];
    setAnnotations([]);
    setRedoAnnotations([]);
    setSelectedAnnotationIndex(null);
    setEditingTextDraft(null);
    setDraftAnnotation(null);
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
    hoverCandidatesRef.current = [];
    hoverCandidateIndexRef.current = 0;
    hoverCandidatesSignatureRef.current = "";
    pendingDetectionRef.current = null;
    setCurrentRect(EMPTY_RECT, true);
    setSelection(false);
    setScreenshotMode("normal");
    screenshotModeRef.current = "normal";
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

  const setAnnotationDraft = (annotation: Annotation | null) => {
    draftAnnotationRef.current = annotation;
    setDraftAnnotation(annotation);
  };

  const selectAnnotationTool = (tool: AnnotationTool) => {
    const toolSize = annotationSizesRef.current[tool] ?? DEFAULT_ANNOTATION_SIZES[tool];
    annotationToolRef.current = tool;
    annotationSizeRef.current = toolSize;
    setAnnotationToolState(tool);
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
    setAnnotationToolState(null);
    selectedAnnotationIndexRef.current = null;
    setSelectedAnnotationIndex(null);
    setEditingTextDraft(null);
    setAnnotationDraft(null);
    renderNeededRef.current = true;
  };

  const applyAnnotations = (next: Annotation[]) => {
    annotationsRef.current = next;
    setAnnotations(next);
    renderNeededRef.current = true;
  };

  const pushAnnotationHistory = (snapshot = annotationsRef.current) => {
    const nextHistory = [...annotationHistoryRef.current, snapshot];
    annotationHistoryRef.current = nextHistory;
    redoAnnotationsRef.current = [];
    setAnnotationHistory(nextHistory);
    setRedoAnnotations([]);
  };

  const commitAnnotation = (annotation: Annotation) => {
    pushAnnotationHistory();
    const next = [...annotationsRef.current, annotation];
    applyAnnotations(next);
  };

  const replaceAnnotations = (next: Annotation[]) => {
    pushAnnotationHistory();
    applyAnnotations(next);
  };

  const deleteSelectedAnnotation = () => {
    const selectedIndex = selectedAnnotationIndexRef.current;
    if (selectedIndex === null) return;
    const current = annotationsRef.current;
    if (!current[selectedIndex]) return;
    replaceAnnotations(current.filter((_, index) => index !== selectedIndex));
    setSelectedAnnotationIndex(null);
  };

  const undoAnnotation = () => {
    const history = annotationHistoryRef.current;
    if (history.length === 0) return;
    const next = history[history.length - 1];
    const historyNext = history.slice(0, -1);
    const redoNext = [...redoAnnotationsRef.current, annotationsRef.current];
    annotationHistoryRef.current = historyNext;
    redoAnnotationsRef.current = redoNext;
    setAnnotationHistory(historyNext);
    setRedoAnnotations(redoNext);
    applyAnnotations(next);
    setSelectedAnnotationIndex(null);
  };

  const redoAnnotation = () => {
    const redo = redoAnnotationsRef.current;
    if (redo.length === 0) return;
    const restored = redo[redo.length - 1];
    const historyNext = [...annotationHistoryRef.current, annotationsRef.current];
    const redoNext = redo.slice(0, -1);
    annotationHistoryRef.current = historyNext;
    redoAnnotationsRef.current = redoNext;
    setAnnotationHistory(historyNext);
    setRedoAnnotations(redoNext);
    applyAnnotations(restored);
    setSelectedAnnotationIndex(null);
  };

  const nextFrame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

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

    listen<string>("screenshot-mode", (event) => {
      const nextMode = event.payload || "normal";
      setScreenshotMode(nextMode);
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

    listen("screenshot-updated", (event) => {
      const base64 = event.payload as string;
      if (base64) {
        loadFullscreenFromBase64(base64);
      } else {
        loadFullscreen();
      }
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
      setActionToolbarSize((current) => (
        current.width === next.width && current.height === next.height ? current : next
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
      setConfig(JSON.parse(await invoke<string>("get_config")));
    } catch {
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

  const loadFullscreen = async () => {
    const sessionId = startNewCaptureSession();
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

  const loadFullscreenFromBase64 = (base64: string) => {
    const sessionId = startNewCaptureSession();
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

  const loadImageFromBase64 = (base64: string, sessionId: number) => {
    if (sessionId !== captureIdRef.current) return;

    if (!base64 || base64.length < 1000) {
      console.warn("[ScreenshotPage] loadImageFromBase64 invalid payload", base64?.length || 0);
      return;
    }

    const dataUrl = "data:image/png;base64," + base64;
    const img = new Image();

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
        screenshotBytes: Math.round(base64.length * 0.75), 
        errorMsg: "" 
      });
      setScreenshotState("ready");
      await waitForStableViewport(img);
      initCanvas(img);

      requestAnimationFrame(() => {
        requestAnimationFrame(async () => {
          if (sessionId !== captureIdRef.current) return;
          overlayVisibleRef.current = true;
          setOverlayVisible(true);
          await invoke("overlay_ready_to_show", { label: getCurrentWindow().label }).catch((err) => {
            console.error("[ScreenshotPage] overlay_ready_to_show failed:", err);
          });
        });
      });
    };

    img.onerror = () => {
      if (sessionId !== captureIdRef.current) return;
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
      console.warn("[ScreenshotPage] image decode failed", sessionId, dataUrl.length);
      cancelScreenshot();
    };
    img.src = dataUrl;
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
  };

  const openTextEditor = (point: Point, targetIndex: number | null, value = "") => {
    const selection = rectRef.current;
    const width = 180;
    const height = 34;
    const x = clamp(point.x - width / 2, selection.x + 8, selection.x + selection.w - width - 8);
    const y = clamp(point.y - height / 2, selection.y + 8, selection.y + selection.h - height - 8);
    setEditingTextDraft({ x, y, value, targetIndex });
  };

  const commitTextDraft = () => {
    const draft = editingTextDraftRef.current;
    if (!draft) return;
    const value = draft.value.trim();
    if (!value) {
      setEditingTextDraft(null);
      return;
    }
    if (draft.targetIndex !== null) {
      const current = annotationsRef.current[draft.targetIndex];
      if (current) {
        const next = [...annotationsRef.current];
        const fontSize = current.size || Math.max(14, annotationSizeRef.current + 14);
        next[draft.targetIndex] = { ...current, text: value, rect: { ...current.rect, w: Math.max(48, value.length * fontSize * 0.72 + 12), h: fontSize + 8 } };
        replaceAnnotations(next);
      }
    } else {
      commitAnnotation(makeTextAnnotation({ x: draft.x + 90, y: draft.y + 17 }, value, annotationColorRef.current, annotationSizeRef.current));
    }
    setEditingTextDraft(null);
    renderNeededRef.current = true;
  };

  const cancelTextDraft = () => setEditingTextDraft(null);

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!overlayVisibleRef.current) return;
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

    isSelectingRef.current = true;
    setIsSelecting(true);
    setHoverCandidate(null);
    startPosRef.current = { x: cx, y: cy };
    setCurrentRect({ x: cx, y: cy, w: 0, h: 0 }, true);
    setSelection(false);
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!overlayVisibleRef.current) return;
    const cx = e.clientX;
    const cy = e.clientY;
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

  const handleMouseUp = () => {
    if (!overlayVisibleRef.current) return;
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
    setSelection(valid);
    renderNeededRef.current = true;
    if (valid && wasSelecting && screenshotModeRef.current === "translate") {
      setTimeout(() => handleTranslate(), 0);
    }
    if (valid && wasSelecting && screenshotModeRef.current === "record") {
      setTimeout(() => enterRecordingMode("region"), 0);
    }
  };

  const handleDoubleClick = () => {
    if (!overlayVisibleRef.current) return;
    if (hasSelectedRef.current) confirmScreenshot("copy");
  };

  function draw(rx: number, ry: number, rw: number, rh: number, translatedImg?: HTMLImageElement) {
    renderScreenshotCanvas({
      canvas: canvasRef.current,
      image: imageRef.current,
      maskedCanvas: maskedCanvasRef.current,
      hoverRect: hoverRectRef.current,
      hoverCandidatesCount: hoverCandidatesRef.current.length,
      hoverCandidateIndex: hoverCandidateIndexRef.current,
      hasSelected: hasSelectedRef.current,
      selection: { x: rx, y: ry, w: rw, h: rh },
      translatedImg: translatedImgRef.current,
      overrideTranslatedImg: translatedImg,
      annotations: annotationsRef.current,
      draftAnnotation: draftAnnotationRef.current,
      selectedAnnotationIndex: selectedAnnotationIndexRef.current,
      detectionBorderWidth: configRef.current.detectionBorderWidth || 2,
      selectionBorderColor: recordingStatusRef.current !== "idle" ? RECORDING_BORDER_COLOR : scrollCaptureModeRef.current !== "idle" ? SCROLL_CAPTURE_BORDER_COLOR : undefined,
      selectionLabelColor: recordingStatusRef.current !== "idle" ? "rgba(239, 68, 68, 0.9)" : scrollCaptureModeRef.current !== "idle" ? "rgba(249, 115, 22, 0.9)" : undefined,
      selectionOnly: recordingStatusRef.current !== "idle",
    });
  }

  const getCurrentPhysicalSelection = () => getPhysicalSelection({
    canvas: canvasRef.current,
    image: imageRef.current,
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
    image: imageRef.current,
    rect: rectRef.current,
  });

  const captureRegionBase64 = async () => {
    const { x, y, w, h } = getCurrentPhysicalSelection();
    return await invoke<string>("capture_region", { x, y, w, h });
  };

  const renderCurrentEditedSelectionBase64 = async () => renderEditedSelectionBase64({
    canvas: canvasRef.current,
    image: imageRef.current,
    rect: rectRef.current,
    translatedResult,
    annotations: annotationsRef.current,
    fallbackColor: annotationColorRef.current,
    fallbackSize: annotationSizeRef.current,
  });

  const getOutputBase64 = async () => (
    annotationsRef.current.length > 0 ? await renderCurrentEditedSelectionBase64() : (translatedResult || await captureRegionBase64())
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

  

  const normalizeScreenshotTranslateError = (error: any) => {
    const raw = error?.message || error?.toString?.() || String(error || "");
    if (/\u672a\u8bc6\u522b\u5230\u6587\u5b57|did not recognize text|recognized no text|no text/i.test(raw)) {
      return "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u672a\u8bc6\u522b\u5230\u6587\u5b57\u3002\u8bf7\u91cd\u65b0\u6846\u9009\u66f4\u6e05\u6670\u3001\u66f4\u5b8c\u6574\u7684\u6587\u5b57\u533a\u57df\u3002";
    }
    return raw
      .replace(/YSN OCR Runtime/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
      .replace(/PP-OCRv5\s*ONNX\s*OCR/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
      .replace(/PP-OCRv5/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
      .replace(/ONNX/gi, "\u672c\u5730\u6a21\u578b")
      .trim() || "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6682\u4e0d\u53ef\u7528\uff0c\u8bf7\u91cd\u65b0\u6846\u9009\u6587\u5b57\u533a\u57df\u540e\u518d\u8bd5\u3002";
  };


  const handleTranslate = async () => {
    if (isTranslatingRef.current || isOCRingRef.current) return;
    const startTime = performance.now();
    let base64 = "";
    try {
      setIsTranslating(true);
      message.loading({ content: "\u6b63\u5728\u8bc6\u522b\u5e76\u7ffb\u8bd1...", key: "translate", duration: 0 });
      base64 = await captureRegionBase64();

      let resultBase64 = "";
      let usedChannel = configRef.current.channel || configRef.current.targetLang || "auto";
      let blocksCount = 1;
      let translationQuality: Awaited<ReturnType<typeof translateWithLocalOcr>>["translationQuality"] | null = null;
      try {
        const result = await translateWithLocalOcr(base64, configRef.current);
        resultBase64 = result.resultBase64;
        usedChannel = result.usedChannel;
        blocksCount = result.blocksCount;
        translationQuality = result.translationQuality;
        setTranslatePairs(result.pairs);
        setTranslateResultPreviewBase64(resultBase64);
      } catch (localErr: any) {
        console.warn("[Local Translate Flow] failed", localErr);
        throw localErr;
      }

      const overlayImg = await loadPngImage(resultBase64);
      translatedImgRef.current = overlayImg;
      draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h, overlayImg);
      setTranslatedResult(resultBase64);
      if (translationQuality && translationQuality.translatableCount === 0 && translationQuality.preservedCount > 0) {
        message.info({ content: "\u5df2\u8bc6\u522b\u5230\u6587\u5b57\uff0c\u4f46\u9009\u533a\u4e3b\u8981\u662f\u6587\u4ef6\u540d\u3001\u8def\u5f84\u6216\u6280\u672f\u6807\u8bc6\uff0c\u5df2\u6309\u89c4\u5219\u4fdd\u7559\u539f\u6587\u3002", key: "translate", duration: 3 });
      } else if (translationQuality && translationQuality.untranslatedCount > 0) {
        message.warning({ content: `\u7ffb\u8bd1\u5b8c\u6210\uff0c${translationQuality.untranslatedCount} \u884c\u672a\u8fd4\u56de\u6709\u6548\u8bd1\u6587\uff0c\u53ef\u5728\u7ed3\u679c\u91cc\u67e5\u770b\u3002`, key: "translate", duration: 4 });
      } else {
        message.success({ content: "\u7ffb\u8bd1\u5b8c\u6210", key: "translate" });
      }

      try {
        const durationSec = ((performance.now() - startTime) / 1000).toFixed(2);
        const record = {
          id: "rec-" + Date.now(),
          time: new Date().toLocaleString(),
          filename: "Screenshot_" + Date.now() + ".png",
          blocks: blocksCount,
          channel: usedChannel,
          duration: durationSec + "s",
          status: "success",
        };
        await invoke("add_history", { record: JSON.stringify(record) });
      } catch (err) {
        console.error("Failed to save history:", err);
      }
      renderNeededRef.current = true;
      setIsTranslating(false);
    } catch (e: any) {
      const msg = normalizeScreenshotTranslateError(e);
      message.error({ content: `\u7ffb\u8bd1\u5931\u8d25\uff1a${msg}`, key: "translate", duration: 4 });
      setIsTranslating(false);
      if (base64) {
        await openOcrResultWindow({
          selection: rectRef.current,
          text: `\u7ffb\u8bd1\u6682\u4e0d\u53ef\u7528\u3002\n\n${msg}\n\n\u5f53\u524d\u5df2\u4f7f\u7528\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6a21\u578b\u8bc6\u522b\uff1b\u5982\u679c\u9884\u89c8\u662f\u767d\u56fe\uff0c\u8bf7\u91cd\u65b0\u6846\u9009\u771f\u5b9e\u6587\u5b57\u533a\u57df\u3002`,
          previewBase64: base64,
          margin: FLOATING_PANEL_MARGIN,
          gap: FLOATING_PANEL_GAP,
          windowSize: OCR_WINDOW_SIZE,
          title: "\u7ffb\u8bd1\u72b6\u6001",
        });
      }
    }
  };


  const handleShowTranslateResult = async () => {
    if (!translatePairs || translatePairs.length === 0) return;
    const statusLabel = (status?: TranslatePair["status"]) => {
      if (status === "preserved") return "\u72b6\u6001\uff1a\u5df2\u6309\u6280\u672f\u6807\u8bc6\u4fdd\u7559";
      if (status === "untranslated") return "\u72b6\u6001\uff1a\u672a\u8fd4\u56de\u6709\u6548\u8bd1\u6587";
      return "\u72b6\u6001\uff1a\u5df2\u7ffb\u8bd1";
    };
    const text = translatePairs.map((pair) => `${statusLabel(pair.status)}\n${pair.o}\n${pair.t}`).join("\n\n");
    const previewBase64 = translateResultPreviewBase64 || translatedResult || await getOutputBase64();
    await openOcrResultWindow({
      selection: rectRef.current,
      text,
      previewBase64,
      margin: FLOATING_PANEL_MARGIN,
      gap: FLOATING_PANEL_GAP,
      windowSize: OCR_WINDOW_SIZE,
      title: "翻译结果",
    });
    await cancelScreenshot();
  };

  const handleOCR = async () => {
    if (isOCRingRef.current || isTranslatingRef.current) return;
    let base64 = "";
    try {
      setIsOCRing(true);
      message.loading({ content: "\u6b63\u5728\u8bc6\u522b\u6587\u5b57...", key: "ocr", duration: 0 });

      base64 = await captureRegionBase64();
      const ocrBlocks: OcrBlock[] = await invoke("run_local_ocr", {
        imageBase64: base64,
        executablePath: null,
        timeoutMs: configRef.current.localOcrTimeoutMs || 15000
      });
      const normalization = await buildOcrNormalizationReport(ocrBlocks || []);
      const texts = normalization.text || "\u672a\u8bc6\u522b\u5230\u6587\u5b57\u3002\n\n\u8bf7\u91cd\u65b0\u6846\u9009\u66f4\u6e05\u6670\u3001\u66f4\u5b8c\u6574\u7684\u6587\u5b57\u533a\u57df\u3002";

      message.destroy();
      setIsOCRing(false);

      if (texts) {
        try {
          await navigator.clipboard.writeText(texts);
        } catch {}
      }

      await openOcrResultWindow({
        selection: rectRef.current,
        text: texts,
        previewBase64: base64,
        margin: FLOATING_PANEL_MARGIN,
        gap: FLOATING_PANEL_GAP,
        windowSize: OCR_WINDOW_SIZE,
        normalizationSummary: {
          rawCount: normalization.rawCount,
          usefulCount: normalization.usefulCount,
          virtualLineCount: normalization.virtualLineCount,
          droppedCount: normalization.droppedCount,
          routeMissingScripts: normalization.routePlan?.missingScripts || [],
        },
      });
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
    } catch (e: any) {
      const msg = normalizeScreenshotTranslateError(e);
      message.error({ content: `\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u5931\u8d25\uff1a${msg}`, key: "ocr", duration: 3 });
      setIsOCRing(false);
      if (base64) {
        await openOcrResultWindow({
          selection: rectRef.current,
          text: `\u8bc6\u522b\u6682\u4e0d\u53ef\u7528\u3002\n\n${msg}\n\n\u5f53\u524d\u5df2\u7ecf\u68c0\u67e5\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6a21\u578b\u3002`,
          previewBase64: base64,
          margin: FLOATING_PANEL_MARGIN,
          gap: FLOATING_PANEL_GAP,
          windowSize: OCR_WINDOW_SIZE,
          title: "\u8bc6\u522b\u72b6\u6001",
        });
        resetScreenshotState();
        await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
      }
    }
  };
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
      setIsRecordingBusy(true);
      const options = await buildRecordingOptions();
      const normalizedOptions = { ...options, fps: 30, resolution: "1080p", output_dir: null };
      const region = { x: normalizedOptions.region_x, y: normalizedOptions.region_y, w: normalizedOptions.region_w, h: normalizedOptions.region_h };
      recordingPickerModeRef.current = null;
      setRecordingPickerMode(null);
      recordingStatusRef.current = "recording";
      setRecordingStatus("recording");
      await openRecordingWindows({
        options: normalizedOptions,
        countdownSeconds: 0,
        autoStart: false,
      }, region);
      const win = getCurrentWindow();
      await win.setAlwaysOnTop(false).catch(() => {});
      await win.hide().catch(() => {});
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
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setRect(EMPTY_RECT);
    setHasSelected(false);
    setTranslatedResult(null);
    setTranslatePairs(null);
    setTranslateResultPreviewBase64("");
    setIsEditing(false);
    annotationSizesRef.current = { ...DEFAULT_ANNOTATION_SIZES };
    setAnnotationToolState(null);
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
    setIsTranslating(false);
    setIsOCRing(false);
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

      <canvas ref={canvasRef} onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} onDoubleClick={handleDoubleClick} style={{ position: "absolute", top: 0, left: 0, zIndex: 10, cursor: "crosshair" }} />


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

