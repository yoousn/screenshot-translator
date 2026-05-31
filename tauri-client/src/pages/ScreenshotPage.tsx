import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { message } from "antd";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
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

interface Config {
  serverUrl?: string;
  clientToken?: string;
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  localOcrExecutablePath?: string;
  localOcrTimeoutMs?: number;
  targetLang?: string;
  channel?: string;
  enableUiControlDetection?: boolean;
  enableVisualDetection?: boolean;
  detectionBorderWidth?: number;
  visualDetectionSensitivity?: number;
}

const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };
const ACTION_TOOLBAR_FALLBACK_SIZE = { width: 620, height: 86 };
const OCR_WINDOW_SIZE = { width: 460, height: 360 };
const FLOATING_PANEL_MARGIN = 8;
const FLOATING_PANEL_GAP = 8;
const DEFAULT_ANNOTATION_COLOR = "#ff4d4f";
const DEFAULT_ANNOTATION_TOOL: AnnotationTool = "rect";
const DEFAULT_ANNOTATION_SIZES: Record<AnnotationTool, number> = { rect: 4, circle: 4, mosaic: 16, arrow: 4, text: 4, brush: 4 };

const makeImageFormData = (base64: string) => {
  const byteCharacters = atob(base64);
  const byteNumbers = new Array(byteCharacters.length);
  for (let i = 0; i < byteCharacters.length; i++) {
    byteNumbers[i] = byteCharacters.charCodeAt(i);
  }
  const byteArray = new Uint8Array(byteNumbers);
  const blob = new Blob([byteArray], { type: "image/png" });
  const formData = new FormData();
  formData.append("image", blob, "screenshot.png");
  return formData;
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
  const isTranslatingRef = useRef(false);
  const isOCRingRef = useRef(false);
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
    setScreenshotState("initializing");
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

    listen<string>("screenshot-mode", (event) => setScreenshotMode(event.payload || "normal"))
      .then((unsub) => { unlistenMode = unsub; })
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
      if (editingTextDraftRef.current) return;
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
        confirmScreenshot("save");
        return;
      }
      if ((e.ctrlKey || e.metaKey) && (e.key === "q" || e.key === "Q")) {
        e.preventDefault();
        if (isTranslatingRef.current || isOCRingRef.current) return;
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
  }, [hasSelected]);

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

    const dataUrl = "data:image/jpeg;base64," + base64;
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
          await invoke("overlay_ready_to_show", { label: getCurrentWindow().label }).catch((err) => {
            console.error("[ScreenshotPage] overlay_ready_to_show failed:", err);
          });
          setOverlayVisible(true);
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
    if (!overlayVisible) return;
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
        annotationToolRef.current === "brush"
          ? { type: "brush", rect: { x: cx, y: cy, w: 0, h: 0 }, points: [{ x: cx, y: cy }] }
          : { type: annotationToolRef.current, rect: { x: cx, y: cy, w: 0, h: 0 } }
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
    if (!overlayVisible) return;
    const cx = e.clientX;
    const cy = e.clientY;
    lastMouseRef.current = { x: cx, y: cy };
    if (mouseTrackerRef.current) {
      mouseTrackerRef.current.style.left = `${cx + 16}px`;
      mouseTrackerRef.current.style.top = `${cy + 20}px`;
      mouseTrackerRef.current.textContent = `${cx}, ${cy}${hasSelectedRef.current ? ` | ${rectRef.current.w}×${rectRef.current.h}` : ""}`;
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
    if (!overlayVisible) return;
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
  };

  const handleDoubleClick = () => {
    if (!overlayVisible) return;
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
    });
  }

  const getCurrentPhysicalSelection = () => getPhysicalSelection({
    canvas: canvasRef.current,
    image: imageRef.current,
    rect: rectRef.current,
  });

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

  
  const handleTranslate = async () => {
    if (isTranslatingRef.current || isOCRingRef.current) return;
    const startTime = performance.now();
    try {
      setIsTranslating(true);
      message.loading({ content: "正在请求翻译重绘...", key: "translate", duration: 0 });
      const base64 = await captureRegionBase64();

      let resultBase64 = "";
      let usedChannel = configRef.current.channel || configRef.current.targetLang || "auto";
      let blocksCount = 1;
      try {
        const result = await translateWithLocalOcr(base64, configRef.current);
        resultBase64 = result.resultBase64;
        usedChannel = result.usedChannel;
        blocksCount = result.blocksCount;
        setTranslatePairs(result.pairs);
        setTranslateResultPreviewBase64(resultBase64);
      } catch (localErr: any) {
        console.warn("[Local OCR Flow] 本地 OCR 或文本翻译失败", localErr);
        throw localErr;
      }

      const overlayImg = await loadPngImage(resultBase64);
      translatedImgRef.current = overlayImg;
      draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h, overlayImg);
      setTranslatedResult(resultBase64);
      message.success({ content: "翻译完成", key: "translate" });

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
      message.error({ content: `翻译失败：${e.message || e}`, key: "translate" });
      setIsTranslating(false);
    }
  };


  const handleShowTranslateResult = async () => {
    if (!translatePairs || translatePairs.length === 0) return;
    const text = translatePairs.map((pair) => `${pair.o}\n${pair.t}`).join("\n\n");
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
    try {
      setIsOCRing(true);
      message.loading({ content: "正在使用本地 OCR 识别文字...", key: "ocr", duration: 0 });

      const base64 = await captureRegionBase64();
      const ocrBlocks: OcrBlock[] = await invoke("run_local_ocr", {
        imageBase64: base64,
        executablePath: configRef.current.localOcrExecutablePath || null,
        timeoutMs: configRef.current.localOcrTimeoutMs || 15000
      });
      const texts = (ocrBlocks || []).map((item) => item.text).filter(Boolean).join("\n");

      message.destroy();
      setIsOCRing(false);

      if (texts) {
        try {
          await navigator.clipboard.writeText(texts);
        } catch {}
      }

      await openOcrResultWindow({ selection: rectRef.current, text: texts, previewBase64: base64, margin: FLOATING_PANEL_MARGIN, gap: FLOATING_PANEL_GAP, windowSize: OCR_WINDOW_SIZE });
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
    } catch (e: any) {
      const msg = e?.message || e?.toString?.() || String(e);
      message.error({ content: `本地 OCR 失败：${msg}`, key: "ocr", duration: 3 });
      setIsOCRing(false);
    }
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
    windowRectsRef.current = [];
    setWindowRects([]);
    setHoverCandidate(null);
    pendingDetectionRef.current = null;
    setIsTranslating(false);
    setIsOCRing(false);
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

      {overlayVisible && hasSelected && !isSelecting && (
        <ScreenshotToolbar
          containerRef={actionToolbarRef}
          style={getActionToolbarStyle({ rect, toolbarSize: actionToolbarSize, fallbackSize: ACTION_TOOLBAR_FALLBACK_SIZE, viewportWidth: window.innerWidth, viewportHeight: window.innerHeight, margin: FLOATING_PANEL_MARGIN, gap: FLOATING_PANEL_GAP })}
          annotationTool={annotationTool}
          annotationColor={annotationColor}
          annotationSize={annotationSize}
          isEditing={isEditing}
          isTranslating={isTranslating}
          isOCRing={isOCRing}
          canUndo={annotationHistory.length > 0}
          canRedo={redoAnnotations.length > 0}
          onSetEditing={setIsEditing}
          onSetAnnotationTool={selectAnnotationTool}
          onSetAnnotationColor={setAnnotationColor}
          onSetAnnotationSize={setCurrentAnnotationSize}
          onTranslate={handleTranslate}
          onShowTranslateResult={handleShowTranslateResult}
          canShowTranslateResult={Boolean(translatePairs && translatePairs.length > 0)}
          onOCR={handleOCR}
          onPin={handlePin}
          onUndo={undoAnnotation}
          onRedo={redoAnnotation}
          onSave={() => confirmScreenshot("save")}
          onCancel={cancelScreenshot}
          onCopy={() => confirmScreenshot("copy")}
        />
      )}


    </div>
  );
}
