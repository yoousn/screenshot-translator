import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { Button, Input, InputNumber, Space, Tooltip, message } from "antd";
import {
  CopyOutlined,
  SaveOutlined,
  CloseOutlined,
  CheckOutlined,
  TranslationOutlined,
  ScanOutlined,
  PushpinOutlined,
} from "@ant-design/icons";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

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

type Rect = { x: number; y: number; w: number; h: number; kind?: "window" | "control" | "visual" };
type AnnotationTool = "rect" | "circle" | "mosaic" | "arrow" | "text" | "brush";
type Point = { x: number; y: number };
type Annotation = { type: AnnotationTool; rect: Rect; points?: Point[]; text?: string; color?: string; size?: number };
type EditingTextDraft = { x: number; y: number; value: string; targetIndex: number | null } | null;

const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };
const ACTION_TOOLBAR_FALLBACK_SIZE = { width: 620, height: 86 };
const OCR_WINDOW_SIZE = { width: 460, height: 360 };
const FLOATING_PANEL_MARGIN = 8;
const FLOATING_PANEL_GAP = 8;

const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(value, max));

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
  const [translatePairs, setTranslatePairs] = useState<Array<{o: string, t: string}> | null>(null);
  const [isEditing, setIsEditing] = useState(false);
  const [annotationTool, setAnnotationTool] = useState<AnnotationTool>("rect");
  const [annotationColor, setAnnotationColor] = useState("#ff4d4f");
  const [annotationSize, setAnnotationSize] = useState(4);
  const [selectedAnnotationIndex, setSelectedAnnotationIndex] = useState<number | null>(null);
  const [editingTextDraft, setEditingTextDraft] = useState<EditingTextDraft>(null);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [redoAnnotations, setRedoAnnotations] = useState<Annotation[]>([]);
  const [draftAnnotation, setDraftAnnotation] = useState<Annotation | null>(null);
  const [dbgStatus, setDbgStatus] = useState({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });
  const [screenshotState, setScreenshotState] = useState<"initializing" | "ready" | "failed">("initializing");
  const [overlayVisible, setOverlayVisible] = useState(false);
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
  const redoAnnotationsRef = useRef<Annotation[]>([]);
  const draftAnnotationRef = useRef<Annotation | null>(null);
  const isEditingRef = useRef(false);
  const annotationToolRef = useRef<AnnotationTool>("rect");
  const annotationColorRef = useRef("#ff4d4f");
  const annotationSizeRef = useRef(4);
  const selectedAnnotationIndexRef = useRef<number | null>(null);
  const editingTextDraftRef = useRef<EditingTextDraft>(null);
  const isDrawingAnnotationRef = useRef(false);
  const isDraggingAnnotationRef = useRef(false);
  const annotationStartRef = useRef({ x: 0, y: 0 });
  const annotationDragStartRef = useRef({ x: 0, y: 0 });

  const decodeTextPairs = (encoded: string) => {
    const bytes = Uint8Array.from(atob(encoded), c => c.charCodeAt(0));
    return JSON.parse(new TextDecoder().decode(bytes));
  };

  const startNewCaptureSession = () => {
    captureIdRef.current += 1;
    const currentId = captureIdRef.current;
    console.log("[ScreenshotPage] new capture session", currentId);

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
    setIsEditing(false);
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
  windowRectsRef.current = windowRects;
  hoverRectRef.current = hoverRect;
  hoverCandidatesRef.current = hoverCandidates;
  annotationsRef.current = annotations;
  redoAnnotationsRef.current = redoAnnotations;
  draftAnnotationRef.current = draftAnnotation;
  isEditingRef.current = isEditing;
  annotationToolRef.current = annotationTool;
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

  const commitAnnotation = (annotation: Annotation) => {
    const next = [...annotationsRef.current, annotation];
    annotationsRef.current = next;
    redoAnnotationsRef.current = [];
    setAnnotations(next);
    setRedoAnnotations([]);
  };

  const undoAnnotation = () => {
    const current = annotationsRef.current;
    if (current.length === 0) return;
    const removed = current[current.length - 1];
    const next = current.slice(0, -1);
    const redoNext = [...redoAnnotationsRef.current, removed];
    annotationsRef.current = next;
    redoAnnotationsRef.current = redoNext;
    setAnnotations(next);
    setRedoAnnotations(redoNext);
    setSelectedAnnotationIndex(null);
    renderNeededRef.current = true;
  };

  const redoAnnotation = () => {
    const redo = redoAnnotationsRef.current;
    if (redo.length === 0) return;
    const restored = redo[redo.length - 1];
    const next = [...annotationsRef.current, restored];
    const redoNext = redo.slice(0, -1);
    annotationsRef.current = next;
    redoAnnotationsRef.current = redoNext;
    setAnnotations(next);
    setRedoAnnotations(redoNext);
    renderNeededRef.current = true;
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
    console.log("[ScreenshotPage] init");
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
      if (e.key === "Escape") {
        cancelScreenshot();
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
        setHoverCandidateList(getDetectionCandidatesAt(lastMouseRef.current.x, lastMouseRef.current.y));
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

      console.log("[ScreenshotPage] screenshot payload received", base64.length);
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
      console.log("[ScreenshotPage] screenshot payload received", base64.length);
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
      console.log("[ScreenshotPage] image loaded & decoded", sessionId);
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
          await invoke("overlay_ready_to_show").catch((err) => {
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

  const getHandleAt = (mx: number, my: number, isClick = false) => {
    if (!hasSelectedRef.current) return null;
    const { x, y, w, h } = rectRef.current;
    const tolerance = 8;
    const points = {
      nw: { x, y, cursor: "nwse-resize" },
      ne: { x: x + w, y, cursor: "nesw-resize" },
      sw: { x, y: y + h, cursor: "nesw-resize" },
      se: { x: x + w, y: y + h, cursor: "nwse-resize" },
      n: { x: x + w / 2, y, cursor: "ns-resize" },
      s: { x: x + w / 2, y: y + h, cursor: "ns-resize" },
      w: { x, y: y + h / 2, cursor: "ew-resize" },
      e: { x: x + w, y: y + h / 2, cursor: "ew-resize" },
    };
    for (const [key, pt] of Object.entries(points)) {
      if (Math.abs(mx - pt.x) <= tolerance && Math.abs(my - pt.y) <= tolerance) return { handle: key, cursor: pt.cursor };
    }
    if (mx >= x && mx <= x + w && my >= y && my <= y + h) return { handle: "move", cursor: "move" };
    if (isClick) {
      let nearestKey = "se";
      let minDistance = Infinity;
      let nearestCursor = "nwse-resize";
      for (const [key, pt] of Object.entries(points)) {
        const dist = Math.hypot(mx - pt.x, my - pt.y);
        if (dist < minDistance) {
          minDistance = dist;
          nearestKey = key;
          nearestCursor = pt.cursor;
        }
      }
      return { handle: nearestKey, cursor: nearestCursor };
    }
    return null;
  };

  const getDetectionRectAt = (mx: number, my: number) => {
    const candidates = getDetectionCandidatesAt(mx, my);
    return candidates[hoverCandidateIndexRef.current % Math.max(1, candidates.length)] || null;
  };

  const rectSignature = (rect: Rect) => `${Math.round(rect.x / 3)}:${Math.round(rect.y / 3)}:${Math.round(rect.w / 3)}:${Math.round(rect.h / 3)}`;

  const sortDetectionCandidates = (candidates: Rect[], mx: number, my: number) => {
    const seen = new Set<string>();
    const unique = candidates.filter((candidate) => {
      const key = rectSignature(candidate);
      if (seen.has(key)) return false;
      seen.add(key);
      return candidate.w >= 12 && candidate.h >= 12;
    });
    return unique.sort((a, b) => {
      const priority = (rect: Rect) => rect.kind === "control" ? 0 : rect.kind === "window" ? 1 : 2;
      const areaA = a.w * a.h;
      const areaB = b.w * b.h;
      const centerA = Math.hypot(mx - (a.x + a.w / 2), my - (a.y + a.h / 2));
      const centerB = Math.hypot(mx - (b.x + b.w / 2), my - (b.y + b.h / 2));
      return priority(a) - priority(b) || areaA - areaB || centerA - centerB;
    });
  };

  const getDetectionCandidatesAt = (mx: number, my: number) => {
    const sensitivity = clamp(configRef.current.visualDetectionSensitivity || 3, 1, 5);
    const visualEnabled = configRef.current.enableVisualDetection === true;
    const candidates: Rect[] = [];
    for (const candidate of windowRectsRef.current) {
      if (mx >= candidate.x && mx <= candidate.x + candidate.w && my >= candidate.y && my <= candidate.y + candidate.h) {
        candidates.push(candidate);
      }
    }
    if (visualEnabled && (candidates.length === 0 || sensitivity >= 4)) candidates.push(...getVisualRectsAt(mx, my));
    return sortDetectionCandidates(candidates, mx, my);
  };

  const getPixel = (imageData: ImageData, x: number, y: number) => {
    const px = clamp(Math.round(x), 0, imageData.width - 1);
    const py = clamp(Math.round(y), 0, imageData.height - 1);
    const idx = (py * imageData.width + px) * 4;
    const data = imageData.data;
    return [data[idx], data[idx + 1], data[idx + 2]];
  };

  const pixelDiff = (a: number[], b: number[]) => (
    Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2])
  ) / 3;

  const verticalEdgeScore = (imageData: ImageData, x: number, y: number, span: number) => {
    let score = 0;
    let count = 0;
    for (let yy = y - span; yy <= y + span; yy += 8) {
      if (yy <= 1 || yy >= imageData.height - 2) continue;
      score += pixelDiff(getPixel(imageData, x, yy), getPixel(imageData, x - 1, yy));
      count += 1;
    }
    return count ? score / count : 0;
  };

  const horizontalEdgeScore = (imageData: ImageData, x: number, y: number, span: number) => {
    let score = 0;
    let count = 0;
    for (let xx = x - span; xx <= x + span; xx += 8) {
      if (xx <= 1 || xx >= imageData.width - 2) continue;
      score += pixelDiff(getPixel(imageData, xx, y), getPixel(imageData, xx, y - 1));
      count += 1;
    }
    return count ? score / count : 0;
  };

  const findVisualBoundary = (
    imageData: ImageData,
    mx: number,
    my: number,
    direction: "left" | "right" | "top" | "bottom",
    span: number,
    threshold: number,
  ) => {
    const step = direction === "left" || direction === "top" ? -2 : 2;
    const horizontal = direction === "top" || direction === "bottom";
    const max = horizontal ? imageData.height - 2 : imageData.width - 2;
    let pos = horizontal ? my : mx;
    for (pos += step; pos > 2 && pos < max; pos += step) {
      const score = horizontal
        ? horizontalEdgeScore(imageData, mx, pos, span)
        : verticalEdgeScore(imageData, pos, my, span);
      if (score >= threshold) return pos;
    }
    return null;
  };

  const getVisualRectAt = (mx: number, my: number): Rect | null => {
    return getVisualRectsAt(mx, my)[0] || null;
  };

  const getVisualRectsAt = (mx: number, my: number): Rect[] => {
    const imageData = analysisImageDataRef.current;
    if (!imageData) return [];
    const width = imageData.width;
    const height = imageData.height;
    if (mx < 0 || my < 0 || mx >= width || my >= height) return [];

    const sensitivity = clamp(configRef.current.visualDetectionSensitivity || 3, 1, 5);
    const thresholdOffset = (3 - sensitivity) * 4;
    const attempts = [
      { span: 128, threshold: 18 + thresholdOffset },
      { span: 96, threshold: 16 + thresholdOffset },
      { span: 64, threshold: 20 + thresholdOffset },
      { span: 36, threshold: 26 + thresholdOffset },
    ];

    const matches: Rect[] = [];
    for (const attempt of attempts) {
      const left = findVisualBoundary(imageData, mx, my, "left", attempt.span, attempt.threshold);
      const right = findVisualBoundary(imageData, mx, my, "right", attempt.span, attempt.threshold);
      const top = findVisualBoundary(imageData, mx, my, "top", attempt.span, attempt.threshold);
      const bottom = findVisualBoundary(imageData, mx, my, "bottom", attempt.span, attempt.threshold);
      if (left === null || right === null || top === null || bottom === null) continue;
      const rect = {
        x: clamp(Math.min(left, right), 0, width - 1),
        y: clamp(Math.min(top, bottom), 0, height - 1),
        w: Math.max(1, Math.abs(right - left)),
        h: Math.max(1, Math.abs(bottom - top)),
        kind: "visual" as const,
      };
      const area = rect.w * rect.h;
      const screenArea = width * height;
      const minW = sensitivity >= 4 ? 56 : 80;
      const minH = sensitivity >= 4 ? 28 : 40;
      if (rect.w >= minW && rect.h >= minH && area < screenArea * 0.9) {
        const cursorMarginX = Math.min(mx - rect.x, rect.x + rect.w - mx);
        const cursorMarginY = Math.min(my - rect.y, rect.y + rect.h - my);
        const cursorTooCloseToEdge = cursorMarginX < 3 || cursorMarginY < 3;
        if (!cursorTooCloseToEdge || sensitivity >= 5) matches.push(rect);
      }
    }
    return sortDetectionCandidates(matches, mx, my);
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

  const pointInSelection = (x: number, y: number) => {
    const r = rectRef.current;
    return hasSelectedRef.current && x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
  };

  const normalizedRectFromPoints = (start: { x: number; y: number }, end: { x: number; y: number }): Rect => {
    const selection = rectRef.current;
    const x1 = clamp(start.x, selection.x, selection.x + selection.w);
    const y1 = clamp(start.y, selection.y, selection.y + selection.h);
    const x2 = clamp(end.x, selection.x, selection.x + selection.w);
    const y2 = clamp(end.y, selection.y, selection.y + selection.h);
    return { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
  };

  const makeLineAnnotation = (tool: AnnotationTool, start: Point, end: Point): Annotation => ({
    type: tool,
    rect: normalizedRectFromPoints(start, end),
    color: annotationColorRef.current,
    size: annotationSizeRef.current,
    points: [
      { x: clamp(start.x, rectRef.current.x, rectRef.current.x + rectRef.current.w), y: clamp(start.y, rectRef.current.y, rectRef.current.y + rectRef.current.h) },
      { x: clamp(end.x, rectRef.current.x, rectRef.current.x + rectRef.current.w), y: clamp(end.y, rectRef.current.y, rectRef.current.y + rectRef.current.h) },
    ],
  });

  const makeTextAnnotation = (point: Point, text: string): Annotation => {
    const fontSize = Math.max(14, annotationSizeRef.current + 14);
    return {
      type: "text",
      rect: { x: point.x, y: point.y, w: Math.max(48, text.length * fontSize * 0.72 + 12), h: fontSize + 8 },
      text,
      color: annotationColorRef.current,
      size: fontSize,
    };
  };

  const isDraggableAnnotation = (annotation: Annotation) => annotation.type === "rect" || annotation.type === "circle" || annotation.type === "text";

  const hitAnnotation = (x: number, y: number) => {
    for (let i = annotationsRef.current.length - 1; i >= 0; i--) {
      const annotation = annotationsRef.current[i];
      const r = annotation.rect;
      const tolerance = Math.max(8, annotation.size || annotationSizeRef.current);
      if (x >= r.x - tolerance && x <= r.x + r.w + tolerance && y >= r.y - tolerance && y <= r.y + r.h + tolerance) return i;
    }
    return null;
  };

  const moveAnnotation = (annotation: Annotation, dx: number, dy: number): Annotation => ({
    ...annotation,
    rect: { ...annotation.rect, x: annotation.rect.x + dx, y: annotation.rect.y + dy },
    points: annotation.points?.map((point) => ({ x: point.x + dx, y: point.y + dy })),
  });

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
        annotationsRef.current = next;
        setAnnotations(next);
      }
    } else {
      commitAnnotation(makeTextAnnotation({ x: draft.x + 90, y: draft.y + 17 }, value));
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
    if (isEditingRef.current && pointInSelection(cx, cy)) {
      if (annotationToolRef.current === "text") {
        const input = window.prompt("输入标注文字", "");
        if (input && input.trim()) {
          commitAnnotation(makeTextAnnotation({ x: cx, y: cy }, input.trim()));
          renderNeededRef.current = true;
        }
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
    const handleInfo = getHandleAt(cx, cy, true);
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
        const next = annotationsRef.current.map((annotation, index) => index === selectedAnnotationIndexRef.current ? moveAnnotation(annotation, dx, dy) : annotation);
        annotationsRef.current = next;
        setAnnotations(next);
        renderNeededRef.current = true;
      }
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
        setAnnotationDraft(makeLineAnnotation("arrow", annotationStartRef.current, { x: cx, y: cy }));
      } else {
        setAnnotationDraft({
          type: annotationToolRef.current,
          rect: normalizedRectFromPoints(annotationStartRef.current, { x: cx, y: cy }),
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

    const handleInfo = getHandleAt(cx, cy);
    if (handleInfo) {
      e.currentTarget.style.cursor = handleInfo.cursor;
      return;
    }
    const candidates = getDetectionCandidatesAt(cx, cy);
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
    isSelectingRef.current = false;
    setIsSelecting(false);
    isDraggingAnnotationRef.current = false;
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

  const drawAnnotation = (ctx: CanvasRenderingContext2D, annotation: Annotation, index?: number) => {
    const { x, y, w, h } = annotation.rect;
    const color = annotation.color || "#ff4d4f";
    const size = annotation.size || 4;
    if (annotation.type === "brush") {
      const points = annotation.points || [];
      if (points.length < 2) return;
      ctx.strokeStyle = color;
      ctx.lineWidth = size;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.beginPath();
      ctx.moveTo(points[0].x, points[0].y);
      for (const point of points.slice(1)) ctx.lineTo(point.x, point.y);
      ctx.stroke();
      ctx.lineCap = "butt";
      return;
    }
    if (annotation.type === "arrow") {
      const points = annotation.points || [];
      if (points.length < 2) return;
      const [start, end] = points;
      const angle = Math.atan2(end.y - start.y, end.x - start.x);
      const head = Math.max(12, size * 3);
      ctx.strokeStyle = color;
      ctx.fillStyle = color;
      ctx.lineWidth = size;
      ctx.beginPath();
      ctx.moveTo(start.x, start.y);
      ctx.lineTo(end.x, end.y);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(end.x, end.y);
      ctx.lineTo(end.x - head * Math.cos(angle - Math.PI / 6), end.y - head * Math.sin(angle - Math.PI / 6));
      ctx.lineTo(end.x - head * Math.cos(angle + Math.PI / 6), end.y - head * Math.sin(angle + Math.PI / 6));
      ctx.closePath();
      ctx.fill();
      return;
    }
    if (annotation.type === "text") {
      if (!annotation.text) return;
      const fontSize = annotation.size || 18;
      ctx.font = fontSize + "px Microsoft YaHei, sans-serif";
      ctx.fillStyle = "rgba(255,255,255,0.72)";
      const width = ctx.measureText(annotation.text).width + 14;
      const height = fontSize + 10;
      ctx.fillRect(x, y, width, height);
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.strokeRect(x, y, width, height);
      ctx.fillStyle = color;
      ctx.fillText(annotation.text, x + 7, y + fontSize + 2);
      annotation.rect.w = width;
      annotation.rect.h = height;
      return;
    }
    if (w <= 0 || h <= 0) return;
    if (annotation.type === "mosaic") {
      const block = 10;
      const temp = document.createElement("canvas");
      temp.width = Math.max(1, Math.ceil(w / block));
      temp.height = Math.max(1, Math.ceil(h / block));
      const tempCtx = temp.getContext("2d");
      if (tempCtx) {
        tempCtx.imageSmoothingEnabled = false;
        tempCtx.drawImage(ctx.canvas, x, y, w, h, 0, 0, temp.width, temp.height);
        ctx.imageSmoothingEnabled = false;
        ctx.drawImage(temp, 0, 0, temp.width, temp.height, x, y, w, h);
        ctx.imageSmoothingEnabled = true;
      }
      ctx.strokeStyle = "rgba(250, 84, 28, 0.85)";
      ctx.lineWidth = 1;
      ctx.strokeRect(x, y, w, h);
      return;
    }
    ctx.strokeStyle = color;
    ctx.lineWidth = size;
    ctx.setLineDash([]);
    if (annotation.type === "circle") {
      ctx.beginPath();
      ctx.ellipse(x + w / 2, y + h / 2, Math.max(1, w / 2), Math.max(1, h / 2), 0, 0, Math.PI * 2);
      ctx.stroke();
    } else {
      ctx.strokeRect(x, y, w, h);
    }
    if (index !== undefined && selectedAnnotationIndexRef.current === index && isDraggableAnnotation(annotation)) {
      ctx.save();
      ctx.setLineDash([4, 3]);
      ctx.strokeStyle = "#1677ff";
      ctx.lineWidth = 1;
      ctx.strokeRect(annotation.rect.x - 4, annotation.rect.y - 4, annotation.rect.w + 8, annotation.rect.h + 8);
      ctx.restore();
    }
  };

  function draw(rx: number, ry: number, rw: number, rh: number, translatedImg?: HTMLImageElement) {
    const canvas = canvasRef.current;
    if (!canvas || !imageRef.current) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    if (maskedCanvasRef.current) ctx.drawImage(maskedCanvasRef.current, 0, 0);
    else {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(imageRef.current, 0, 0, canvas.width, canvas.height);
      ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
    }
    const preview = hoverRectRef.current;
    if (!hasSelectedRef.current && preview && preview.w > 0 && preview.h > 0) {
      const scaleX = imageRef.current.naturalWidth / canvas.width;
      const scaleY = imageRef.current.naturalHeight / canvas.height;
      ctx.clearRect(preview.x, preview.y, preview.w, preview.h);
      ctx.drawImage(
        imageRef.current,
        preview.x * scaleX,
        preview.y * scaleY,
        preview.w * scaleX,
        preview.h * scaleY,
        preview.x,
        preview.y,
        preview.w,
        preview.h
      );
      ctx.strokeStyle = "#1677ff";
      ctx.lineWidth = clamp(configRef.current.detectionBorderWidth || 2, 1, 6);
      ctx.setLineDash([]);
      ctx.strokeRect(preview.x, preview.y, preview.w, preview.h);
      ctx.fillStyle = "#1677ff";
      const hs = 7;
      const halfHs = hs / 2;
      const points = [
        { x: preview.x, y: preview.y },
        { x: preview.x + preview.w, y: preview.y },
        { x: preview.x, y: preview.y + preview.h },
        { x: preview.x + preview.w, y: preview.y + preview.h },
        { x: preview.x + preview.w / 2, y: preview.y },
        { x: preview.x + preview.w / 2, y: preview.y + preview.h },
        { x: preview.x, y: preview.y + preview.h / 2 },
        { x: preview.x + preview.w, y: preview.y + preview.h / 2 },
      ];
      for (const p of points) ctx.fillRect(p.x - halfHs, p.y - halfHs, hs, hs);
      const totalCandidates = hoverCandidatesRef.current.length;
      const layerText = totalCandidates > 1 ? ` / ${hoverCandidateIndexRef.current + 1}/${totalCandidates} / Tab切换` : "";
      const kindLabel = preview.kind === "control" ? "控件" : preview.kind === "visual" ? "视觉" : preview.kind === "window" ? "窗口" : "";
      const kindText = kindLabel ? ` / ${kindLabel}` : "";
      const sizeText = `${Math.round(preview.w)} x ${Math.round(preview.h)}${kindText}${layerText} / Enter确认`;
      ctx.font = "12px sans-serif";
      const sizeWidth = ctx.measureText(sizeText).width;
      const labelY = preview.y - 24 >= 0 ? preview.y - 24 : preview.y + 4;
      ctx.fillStyle = "#1677ff";
      ctx.fillRect(preview.x, labelY, sizeWidth + 12, 20);
      ctx.fillStyle = "#ffffff";
      ctx.fillText(sizeText, preview.x + 6, labelY + 14);
    }
    if (rw > 0 && rh > 0) {
      ctx.clearRect(rx, ry, rw, rh);
      const activeImg = translatedImg || translatedImgRef.current;
      if (activeImg) ctx.drawImage(activeImg, rx, ry, rw, rh);
      else {
        const scaleX = imageRef.current.naturalWidth / canvas.width;
        const scaleY = imageRef.current.naturalHeight / canvas.height;
        ctx.drawImage(imageRef.current, rx * scaleX, ry * scaleY, rw * scaleX, rh * scaleY, rx, ry, rw, rh);
      }
      [...annotationsRef.current, ...(draftAnnotationRef.current ? [draftAnnotationRef.current] : [])].forEach((annotation, index) => drawAnnotation(ctx, annotation, index));
      ctx.strokeStyle = "#1677ff";
      ctx.lineWidth = clamp(configRef.current.detectionBorderWidth || 2, 1, 6);
      ctx.strokeRect(rx, ry, rw, rh);
      ctx.fillStyle = "#ffffff";
      ctx.strokeStyle = "#1677ff";
      const hs = 6;
      const halfHs = 3;
      const handlePoints = [
        { x: rx, y: ry }, { x: rx + rw, y: ry }, { x: rx, y: ry + rh }, { x: rx + rw, y: ry + rh },
        { x: rx + rw / 2, y: ry }, { x: rx + rw / 2, y: ry + rh }, { x: rx, y: ry + rh / 2 }, { x: rx + rw, y: ry + rh / 2 },
      ];
      for (const p of handlePoints) {
        ctx.fillRect(p.x - halfHs, p.y - halfHs, hs, hs);
        ctx.strokeRect(p.x - halfHs, p.y - halfHs, hs, hs);
      }
      ctx.fillStyle = "rgba(22, 119, 255, 0.85)";
      ctx.font = "12px sans-serif";
      const text = `${Math.round(rw)} x ${Math.round(rh)}`;
      const textWidth = ctx.measureText(text).width;
      const tipY = ry - 22 >= 0 ? ry - 22 : ry + rh + 4;
      ctx.fillRect(rx, tipY, textWidth + 12, 20);
      ctx.fillStyle = "#ffffff";
      ctx.fillText(text, rx + 6, tipY + 14);
    }
  }

  const getPhysicalSelection = () => {
    const canvas = canvasRef.current;
    const image = imageRef.current;
    const r = rectRef.current;
    if (!canvas || !image || r.w <= 0 || r.h <= 0) throw new Error("选区范围无效");
    const scaleX = image.naturalWidth / canvas.width;
    const scaleY = image.naturalHeight / canvas.height;
    const x = Math.max(0, Math.min(image.naturalWidth - 1, Math.round(r.x * scaleX)));
    const y = Math.max(0, Math.min(image.naturalHeight - 1, Math.round(r.y * scaleY)));
    const w = Math.max(1, Math.min(image.naturalWidth - x, Math.round(r.w * scaleX)));
    const h = Math.max(1, Math.min(image.naturalHeight - y, Math.round(r.h * scaleY)));
    return { x, y, w, h };
  };

  const cropSelectionFromLoadedImage = () => {
    const image = imageRef.current;
    if (!image) throw new Error("截图图片未加载");
    const { x, y, w, h } = getPhysicalSelection();
    const cropCanvas = document.createElement("canvas");
    cropCanvas.width = w;
    cropCanvas.height = h;
    const ctx = cropCanvas.getContext("2d");
    if (!ctx) throw new Error("Canvas 不可用");
    ctx.drawImage(image, x, y, w, h, 0, 0, w, h);
    return { base64: cropCanvas.toDataURL("image/png").split(",")[1] || "", x, y, w, h };
  };

  const captureRegionBase64 = async () => {
    const { x, y, w, h } = getPhysicalSelection();
    return await invoke<string>("capture_region", { x, y, w, h });
  };

  const loadPngImage = (base64: string) => new Promise<HTMLImageElement>((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = "data:image/png;base64," + base64;
  });

  const renderEditedSelectionBase64 = async () => {
    const image = imageRef.current;
    if (!image) throw new Error("截图图片未加载");
    const physical = getPhysicalSelection();
    const cropCanvas = document.createElement("canvas");
    cropCanvas.width = physical.w;
    cropCanvas.height = physical.h;
    const ctx = cropCanvas.getContext("2d");
    if (!ctx) throw new Error("Canvas 不可用");

    if (translatedResult) {
      const translatedImage = await loadPngImage(translatedResult);
      ctx.drawImage(translatedImage, 0, 0, physical.w, physical.h);
    } else {
      ctx.drawImage(image, physical.x, physical.y, physical.w, physical.h, 0, 0, physical.w, physical.h);
    }

    const scaleX = image.naturalWidth / (canvasRef.current?.width || window.innerWidth);
    const scaleY = image.naturalHeight / (canvasRef.current?.height || window.innerHeight);
    const mapPoint = (point: Point) => ({ x: Math.round((point.x - rectRef.current.x) * scaleX), y: Math.round((point.y - rectRef.current.y) * scaleY) });
    for (const annotation of annotationsRef.current) {
      const ax = Math.max(0, Math.round((annotation.rect.x - rectRef.current.x) * scaleX));
      const ay = Math.max(0, Math.round((annotation.rect.y - rectRef.current.y) * scaleY));
      const aw = Math.max(1, Math.round(annotation.rect.w * scaleX));
      const ah = Math.max(1, Math.round(annotation.rect.h * scaleY));
      if (annotation.type === "brush") {
        const points = (annotation.points || []).map(mapPoint);
        if (points.length < 2) continue;
        ctx.strokeStyle = "#ff4d4f";
        ctx.lineWidth = 4;
        ctx.lineCap = "round";
        ctx.lineJoin = "round";
        ctx.beginPath();
        ctx.moveTo(points[0].x, points[0].y);
        for (const point of points.slice(1)) ctx.lineTo(point.x, point.y);
        ctx.stroke();
        ctx.lineCap = "butt";
      } else if (annotation.type === "arrow") {
        const points = (annotation.points || []).map(mapPoint);
        if (points.length < 2) continue;
        const [start, end] = points;
        const angle = Math.atan2(end.y - start.y, end.x - start.x);
        const head = 14;
        ctx.strokeStyle = "#ff4d4f";
        ctx.fillStyle = "#ff4d4f";
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.moveTo(start.x, start.y);
        ctx.lineTo(end.x, end.y);
        ctx.stroke();
        ctx.beginPath();
        ctx.moveTo(end.x, end.y);
        ctx.lineTo(end.x - head * Math.cos(angle - Math.PI / 6), end.y - head * Math.sin(angle - Math.PI / 6));
        ctx.lineTo(end.x - head * Math.cos(angle + Math.PI / 6), end.y - head * Math.sin(angle + Math.PI / 6));
        ctx.closePath();
        ctx.fill();
      } else if (annotation.type === "text") {
        if (!annotation.text) continue;
        ctx.font = `${Math.max(18, Math.round(18 * scaleY))}px Microsoft YaHei, sans-serif`;
        const width = ctx.measureText(annotation.text).width + 14;
        ctx.fillStyle = "rgba(255,255,255,0.92)";
        ctx.fillRect(ax, ay, width, 30);
        ctx.strokeStyle = "#ff4d4f";
        ctx.strokeRect(ax, ay, width, 30);
        ctx.fillStyle = "#ff4d4f";
        ctx.fillText(annotation.text, ax + 7, ay + 22);
      } else if (annotation.type === "mosaic") {
        const block = 10;
        const temp = document.createElement("canvas");
        temp.width = Math.max(1, Math.ceil(aw / block));
        temp.height = Math.max(1, Math.ceil(ah / block));
        const tempCtx = temp.getContext("2d");
        if (tempCtx) {
          tempCtx.imageSmoothingEnabled = false;
          tempCtx.drawImage(cropCanvas, ax, ay, aw, ah, 0, 0, temp.width, temp.height);
          ctx.imageSmoothingEnabled = false;
          ctx.drawImage(temp, 0, 0, temp.width, temp.height, ax, ay, aw, ah);
          ctx.imageSmoothingEnabled = true;
        }
      } else {
        ctx.strokeStyle = "#ff4d4f";
        ctx.lineWidth = Math.max(2, Math.round((configRef.current.detectionBorderWidth || 2) * scaleX));
        ctx.strokeRect(ax, ay, aw, ah);
      }
    }
    return cropCanvas.toDataURL("image/png").split(",")[1] || "";
  };

  const getOutputBase64 = async () => (
    annotationsRef.current.length > 0 ? await renderEditedSelectionBase64() : (translatedResult || await captureRegionBase64())
  );

  interface OcrBlock {
    text: string;
    confidence: number;
    box_coords: [number, number][];
  }

  const renderTranslatedBlocks = (
    base64Image: string,
    blocks: OcrBlock[],
    translations: string[]
  ): Promise<string> => {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.src = "data:image/png;base64," + base64Image;
      img.onload = () => {
        const canvas = document.createElement("canvas");
        canvas.width = img.width;
        canvas.height = img.height;
        const ctx = canvas.getContext("2d", { willReadFrequently: true });
        if (!ctx) {
          reject(new Error("无法创建 2D 画布上下文"));
          return;
        }

        // 绘制原始裁剪截图
        ctx.drawImage(img, 0, 0);

        // 逐块擦除并重绘翻译文字
        blocks.forEach((block, idx) => {
          const transText = translations[idx] || block.text;
          const box = block.box_coords;
          if (box.length < 4) return;

          const xs = box.map(p => p[0]);
          const ys = box.map(p => p[1]);
          const minX = Math.min(...xs);
          const maxX = Math.max(...xs);
          const minY = Math.min(...ys);
          const maxY = Math.max(...ys);
          const w = maxX - minX;
          const h = maxY - minY;

          // 1. 多点背景 RGB 采样
          const corners = [
            [minX + 2, minY + 2],
            [maxX - 2, minY + 2],
            [maxX - 2, maxY - 2],
            [minX + 2, maxY - 2]
          ];
          
          let sumR = 0, sumG = 0, sumB = 0, samples = 0;
          corners.forEach(([px, py]) => {
            const cx = Math.max(0, Math.min(img.width - 1, px));
            const cy = Math.max(0, Math.min(img.height - 1, py));
            const pixel = ctx.getImageData(cx, cy, 1, 1).data;
            sumR += pixel[0];
            sumG += pixel[1];
            sumB += pixel[2];
            samples++;
          });

          const avgR = Math.round(sumR / samples);
          const avgG = Math.round(sumG / samples);
          const avgB = Math.round(sumB / samples);

          // 擦除原文字区块
          ctx.fillStyle = `rgb(${avgR}, ${avgG}, ${avgB})`;
          ctx.fillRect(minX, minY, w, h);

          // 2. 相对亮度反色计算
          const luminance = 0.299 * avgR + 0.587 * avgG + 0.114 * avgB;
          const fontColor = luminance > 128 ? "#000000" : "#ffffff";

          // 3. 自适应高度排版
          const fontSize = Math.max(12, Math.min(48, Math.round(h * 0.85)));
          ctx.font = `${fontSize}px 'Microsoft YaHei', -apple-system, sans-serif`;
          ctx.fillStyle = fontColor;
          ctx.textBaseline = "middle";
          ctx.textAlign = "center";

          // 智能按最大宽度折行
          const chars = transText.split("");
          let line = "";
          const lines: string[] = [];
          
          for (let n = 0; n < chars.length; n++) {
            const testLine = line + chars[n];
            const metrics = ctx.measureText(testLine);
            if (metrics.width > w && n > 0) {
              lines.push(line);
              line = chars[n];
            } else {
              line = testLine;
            }
          }
          lines.push(line);

          // 居中垂直绘制
          const totalTextHeight = lines.length * fontSize * 1.1;
          let startY = minY + h / 2 - totalTextHeight / 2 + fontSize / 2;

          lines.forEach(l => {
            ctx.fillText(l, minX + w / 2, startY);
            startY += fontSize * 1.1;
          });
        });

        // 导出 PNG base64 字节流
        const base64Png = canvas.toDataURL("image/png").replace(/^data:image\/png;base64,/, "");
        resolve(base64Png);
      };
      img.onerror = (e) => reject(new Error("原始截图解码失败：" + e));
    });
  };

  const handlePin = async () => {
    if (!hasSelected || rect.w <= 0 || rect.h <= 0) return;
    const { base64, x: px, y: py, w: pw, h: ph } = cropSelectionFromLoadedImage();
    if (!base64) return;
    
    const pinId = Date.now().toString();
    const label = `pin_${pinId}`;
    const imgData = await getOutputBase64();
    
    let finalX = px;
    let finalY = py;
    let factor = 1;
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      const pos = await win.outerPosition();
      factor = await win.scaleFactor();
      finalX += pos.x;
      finalY += pos.y;
    } catch (e) {
      console.warn("Failed to get window position", e);
    }
    
    // Convert physical to logical for WebviewWindow creation
    const logicalX = finalX / factor;
    const logicalY = finalY / factor;
    const logicalW = pw / factor;
    const logicalH = ph / factor;
    
    try {
      let sent = false;
      const unlistenReady = await listen(`pin-ready-${label}`, () => {
        sent = true;
        emit(`pin-image-${label}`, imgData).catch(() => {});
      });
      const win = new WebviewWindow(label, {
        url: "index.html",
        title: "Pin",
        transparent: true,
        decorations: false,
        alwaysOnTop: true,
        x: logicalX,
        y: logicalY,
        width: logicalW,
        height: logicalH,
        skipTaskbar: true
      });
      
      win.once('tauri://created', () => {
        setTimeout(() => {
          if (!sent) emit(`pin-image-${label}`, imgData).catch(() => {});
        }, 1000);
      });
      win.once('tauri://destroyed', () => unlistenReady());
      
      cancelScreenshot();
    } catch (e) {
      console.error("Failed to create pin window", e);
      message.error("钉图失败");
    }
  };

  const handlePreview = async () => {
    if (!hasSelected || rect.w <= 0 || rect.h <= 0) return;
    try {
      const imgData = await getOutputBase64();
      const label = `pin_preview_${Date.now()}`;
      const maxW = 720;
      const maxH = 520;
      const scale = Math.min(1, maxW / rect.w, maxH / rect.h);
      let sent = false;
      const unlistenReady = await listen(`pin-ready-${label}`, () => {
        sent = true;
        emit(`pin-image-${label}`, imgData).catch(() => {});
      });
      const win = new WebviewWindow(label, {
        url: "index.html",
        title: "截图预览",
        transparent: false,
        decorations: true,
        alwaysOnTop: true,
        width: Math.max(240, Math.round(rect.w * scale)),
        height: Math.max(180, Math.round(rect.h * scale)),
        resizable: true,
        skipTaskbar: false,
      });
      win.once("tauri://created", () => {
        setTimeout(() => {
          if (!sent) emit(`pin-image-${label}`, imgData).catch(() => {});
        }, 500);
      });
      win.once("tauri://destroyed", () => unlistenReady());
    } catch (e: any) {
      message.error("预览失败: " + (e?.message || e?.toString?.() || String(e)));
    }
  };
  
  const handleTranslate = async () => {
    const startTime = performance.now();
    const serverUrl = configRef.current.serverUrl || "https://ocr.yousn.me";
    const token = configRef.current.clientToken || "";
    const targetLang = configRef.current.targetLang || "zh";
    try {
      setIsTranslating(true);
      message.loading({ content: "正在请求翻译重绘...", key: "translate", duration: 0 });
      const base64 = await captureRegionBase64();
      
      let usedChannel = configRef.current.channel || configRef.current.targetLang || "auto";
      let blocksCount = 1;
      
      let resultBase64 = "";
      try {
        console.log("[Local OCR Flow] \u6b63\u5728\u8c03\u7528\u672c\u5730 OCR...");
        const ocrBlocks: OcrBlock[] = await invoke("run_local_ocr", {
          imageBase64: base64,
          executablePath: configRef.current.localOcrExecutablePath || null,
          timeoutMs: configRef.current.localOcrTimeoutMs || 15000
        });

        if (!ocrBlocks || ocrBlocks.length === 0) {
          throw new Error("\u672c\u5730 OCR \u672a\u8bc6\u522b\u5230\u6587\u5b57");
        }

        console.log("[Local OCR Flow] \u672c\u5730 OCR \u5b8c\u6210\uff0c\u6b63\u5728\u8bf7\u6c42\u6587\u672c\u7ffb\u8bd1...", ocrBlocks.length);
        const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/translate_text`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "x-api-key": token
          },
          body: JSON.stringify({
            blocks: ocrBlocks.map(b => ({
              text: b.text,
              confidence: b.confidence,
              box: b.box_coords
            })),
            source_lang: "auto",
            target_lang: targetLang
          })
        });

        if (!response.ok) {
          throw new Error(`\u6587\u672c\u7ffb\u8bd1\u63a5\u53e3\u5f02\u5e38\uff1a${response.status}`);
        }

        const transData = await response.json();
        if (transData.status !== "success") {
          throw new Error(transData.error || "\u6587\u672c\u7ffb\u8bd1\u5931\u8d25");
        }

        usedChannel = transData.channel || usedChannel;
        blocksCount = ocrBlocks.length;
        resultBase64 = await renderTranslatedBlocks(base64, ocrBlocks, transData.translations || []);
        setTranslatePairs(ocrBlocks.map((b, i) => ({ o: b.text, t: (transData.translations || [])[i] || b.text })));
      } catch (localErr: any) {
        console.warn("[Local OCR Flow] \u672c\u5730 OCR \u6216\u6587\u672c\u7ffb\u8bd1\u5931\u8d25", localErr);
        throw localErr;
      }

      const dataUrl = "data:image/png;base64," + resultBase64;
      const overlayImg = await new Promise<HTMLImageElement>((resolve, reject) => {
        const img = new Image();
        img.onload = () => resolve(img);
        img.onerror = () => reject(new Error("翻译结果图片解码失败"));
        img.src = dataUrl;
      });

      translatedImgRef.current = overlayImg;
      draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h, overlayImg);
      setTranslatedResult(resultBase64);
      message.success({ content: "翻译完成！", key: "translate" });
      
      try {
        const durationSec = ((performance.now() - startTime) / 1000).toFixed(2);
        const record = {
          id: "rec-" + Date.now(),
          time: new Date().toLocaleString(),
          filename: "Screenshot_" + Date.now() + ".png",
          blocks: blocksCount,
          channel: usedChannel,
          duration: durationSec + "s",
          status: "success"
        };
        await invoke("add_history", { record: JSON.stringify(record) });
      } catch (err) {
        console.error("Failed to save history:", err);
      }
      renderNeededRef.current = true;
      setIsTranslating(false);
    } catch (e: any) {
      message.error({ content: `翻译失败: ${e.message || e}`, key: "translate" });
      setIsTranslating(false);
    }
  };

  const getOcrWindowPosition = async () => {
    const selection = rectRef.current;
    let screenX = 0;
    let screenY = 0;
    let factor = 1;

    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      const win = getCurrentWindow();
      const pos = await win.outerPosition();
      factor = await win.scaleFactor();
      screenX = pos.x / factor;
      screenY = pos.y / factor;
    } catch (error) {
      console.warn("Failed to get screenshot window position", error);
    }

    const minLeft = screenX + FLOATING_PANEL_MARGIN;
    const minTop = screenY + FLOATING_PANEL_MARGIN;
    const maxLeft = Math.max(minLeft, screenX + window.innerWidth - OCR_WINDOW_SIZE.width - FLOATING_PANEL_MARGIN);
    const maxTop = Math.max(minTop, screenY + window.innerHeight - OCR_WINDOW_SIZE.height - FLOATING_PANEL_MARGIN);
    const hasSpaceRight = selection.x + selection.w + FLOATING_PANEL_GAP + OCR_WINDOW_SIZE.width <= window.innerWidth - FLOATING_PANEL_MARGIN;
    const leftCandidate = screenX + (hasSpaceRight ? selection.x + selection.w + FLOATING_PANEL_GAP : selection.x);
    const topCandidate = screenY + selection.y;

    return {
      x: clamp(leftCandidate, minLeft, maxLeft),
      y: clamp(topCandidate, minTop, maxTop),
    };
  };

  const openOcrResultWindow = async (text: string, previewBase64: string) => {
    const label = `ocr_${Date.now()}`;
    const payload = JSON.stringify({ text, previewBase64 });
    const position = await getOcrWindowPosition();
    let isPayloadDelivered = false;
    let resolvePayload: () => void = () => {};
    const payloadReady = new Promise<void>((resolve) => {
      resolvePayload = resolve;
    });

    const sendPayload = (finish = false) => {
      if (isPayloadDelivered) return;
      emit(`ocr-result-${label}`, payload).finally(() => {
        if (finish) {
          isPayloadDelivered = true;
          resolvePayload();
        }
      });
    };

    const unlistenReady = await listen(`ocr-ready-${label}`, () => sendPayload(true));
    try {
      const win = new WebviewWindow(label, {
        url: "index.html",
        title: "OCR 识字结果",
        decorations: false,
        alwaysOnTop: true,
        focus: true,
        x: position.x,
        y: position.y,
        width: OCR_WINDOW_SIZE.width,
        height: OCR_WINDOW_SIZE.height,
        minWidth: 360,
        minHeight: 260,
        resizable: true,
        skipTaskbar: true,
        preventOverflow: true,
        shadow: true,
      });

      win.once("tauri://created", () => {
        win.setFocus().catch(() => {});
        setTimeout(sendPayload, 500);
      });
      win.once("tauri://destroyed", () => {
        if (!isPayloadDelivered) {
          isPayloadDelivered = true;
          resolvePayload();
        }
      });

      const retryTimer = window.setInterval(() => sendPayload(), 300);
      const timeoutTimer = window.setTimeout(() => {
        if (!isPayloadDelivered) {
          isPayloadDelivered = true;
          resolvePayload();
        }
      }, 5000);
      await payloadReady;
      window.clearInterval(retryTimer);
      window.clearTimeout(timeoutTimer);
    } finally {
      unlistenReady();
    }
  };

  const handleOCR = async () => {
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

      await openOcrResultWindow(texts, base64);
      resetScreenshotState();
      await invoke("cancel_screenshot").catch(() => {});
    } catch (e: any) {
      const msg = e?.message || e?.toString?.() || String(e);
      message.error({ content: `\u672c\u5730 OCR \u5931\u8d25\uff1a${msg}`, key: "ocr", duration: 3 });
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
    setIsEditing(false);
    setAnnotations([]);
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
    await invoke("cancel_screenshot").catch(() => {});
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
      await invoke("cancel_screenshot");
      if (action === "save") {
        try {
          await invoke<string>("save_image_to_file", { imageBase64: base64 });
        } catch (saveErr: any) {
          if (saveErr !== "用户取消了保存") {
            message.error("保存失败: " + (saveErr.message || saveErr.toString()));
          }
        }
      }
    } catch (e: any) {
      message.error("截图操作失败: " + (e.message || e.toString()));
    }
  };

  const getActionToolbarStyle = (): React.CSSProperties => {
    const toolbarWidth = actionToolbarSize.width || ACTION_TOOLBAR_FALLBACK_SIZE.width;
    const toolbarHeight = actionToolbarSize.height || ACTION_TOOLBAR_FALLBACK_SIZE.height;
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;
    const maxLeft = Math.max(FLOATING_PANEL_MARGIN, viewportWidth - toolbarWidth - FLOATING_PANEL_MARGIN);
    const maxTop = Math.max(FLOATING_PANEL_MARGIN, viewportHeight - toolbarHeight - FLOATING_PANEL_MARGIN);
    const hasSpaceBelow = rect.y + rect.h + FLOATING_PANEL_GAP + toolbarHeight <= viewportHeight - FLOATING_PANEL_MARGIN;
    const topCandidate = hasSpaceBelow
      ? rect.y + rect.h + FLOATING_PANEL_GAP
      : rect.y - toolbarHeight - FLOATING_PANEL_GAP;
    const leftCandidate = rect.x + rect.w - toolbarWidth >= FLOATING_PANEL_MARGIN
      ? rect.x + rect.w - toolbarWidth
      : rect.x;

    return {
      position: "absolute",
      top: clamp(topCandidate, FLOATING_PANEL_MARGIN, maxTop),
      left: clamp(leftCandidate, FLOATING_PANEL_MARGIN, maxLeft),
      zIndex: 100,
      background: "#fff",
      padding: "6px 10px",
      borderRadius: 8,
      boxShadow: "0 2px 12px rgba(0, 0, 0, 0.12)",
      border: "1px solid #e8e8e8",
      width: "max-content",
      maxWidth: `calc(100vw - ${FLOATING_PANEL_MARGIN * 2}px)`,
      whiteSpace: "nowrap",
    };
  };

  return (
    <div
      {overlayVisible && !hasSelected && (
        <div ref={mouseTrackerRef} style={{ position: "absolute", top: -100, left: -100, zIndex: 9999, background: "rgba(0, 0, 0, 0.75)", color: "#fff", padding: "2px 8px", borderRadius: "4px", fontSize: "11px", fontFamily: "Consolas, Monaco, monospace", pointerEvents: "none", whiteSpace: "nowrap", lineHeight: "18px", display: "none" }}>0, 0</div>
      )}

      {isTranslating && rect.w > 0 && rect.h > 0 && (
        <div style={{ position: "absolute", top: rect.y, left: rect.x, width: rect.w, height: rect.h, zIndex: 200, background: "rgba(240, 240, 245, 0.75)", border: "2px dashed #1677ff", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", boxSizing: "border-box", overflow: "hidden" }}>
          <div style={{ width: 32, height: 32, minWidth: 32, minHeight: 32, flex: "0 0 32px", borderRadius: "50%", border: "3px solid #e0e0e0", borderTopColor: "#1677ff", animation: "spin 0.8s linear infinite" }} />
          {rect.h > 40 && rect.w > 80 && <div style={{ marginTop: 8, color: "#1677ff", fontSize: 12, fontFamily: "'Inter', sans-serif", fontWeight: 500, whiteSpace: "nowrap", textShadow: "0 1px 2px rgba(255,255,255,0.8)" }}>???...</div>}
        </div>
      )}

      {editingTextDraft && (
        <div style={{ position: "absolute", left: editingTextDraft.x, top: editingTextDraft.y, zIndex: 80, display: "flex", gap: 6, alignItems: "center", padding: 6, borderRadius: 8, background: "rgba(255,255,255,0.96)", boxShadow: "0 8px 24px rgba(0,0,0,0.16)" }} onMouseDown={(event) => event.stopPropagation()}>
          <Input autoFocus size="small" value={editingTextDraft.value} placeholder="????" style={{ width: 170 }} onChange={(event) => setEditingTextDraft((draft) => draft ? { ...draft, value: event.target.value } : draft)} onPressEnter={commitTextDraft} onKeyDown={(event) => { if (event.key === "Escape") cancelTextDraft(); }} />
          <Button size="small" type="primary" onClick={commitTextDraft}>??</Button>
        </div>
      )}

      <canvas ref={canvasRef} onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} onDoubleClick={handleDoubleClick} style={{ position: "absolute", top: 0, left: 0, zIndex: 10, cursor: "crosshair" }} />

      {overlayVisible && hasSelected && !isSelecting && (
        <div ref={actionToolbarRef} style={getActionToolbarStyle()} onContextMenu={(e) => e.stopPropagation()}>
          <Space size={6} style={{ display: "inline-flex", flexWrap: "nowrap", whiteSpace: "nowrap", alignItems: "center" }}>
            {[
              { key: "rect", tip: "??", icon: <BorderOutlined /> },
              { key: "circle", tip: "??", icon: <span style={{ fontSize: 22, lineHeight: 1 }}>?</span> },
              { key: "arrow", tip: "??", icon: <span style={{ fontSize: 21, lineHeight: 1 }}>?</span> },
              { key: "brush", tip: "??", icon: <HighlightOutlined /> },
              { key: "mosaic", tip: "???", icon: <span style={{ fontSize: 20, lineHeight: 1 }}>?</span> },
              { key: "text", tip: "??", icon: <span style={{ fontWeight: 800, fontSize: 19 }}>T</span> },
            ].map((item) => (
              <Tooltip key={item.key} title={item.tip}>
                <Button size="middle" style={{ width: 36, height: 36, padding: 0, fontSize: 18 }} type={annotationTool === item.key ? "primary" : "default"} icon={item.icon} onClick={() => { setIsEditing(true); setAnnotationTool(item.key as AnnotationTool); }} />
              </Tooltip>
            ))}
            <Tooltip title="??"><Button size="middle" style={{ width: 42, height: 36, padding: 0 }} type="primary" ghost onClick={handleTranslate} loading={isTranslating} icon={<span style={{ fontSize: 13, fontWeight: 800 }}>A/?</span>} /></Tooltip>
            <Tooltip title="????"><Button size="middle" style={{ width: 36, height: 36, padding: 0, fontSize: 18 }} icon={<ScanOutlined />} onClick={handleOCR} loading={isOCRing} /></Tooltip>
            <Tooltip title="??"><Button size="middle" style={{ width: 36, height: 36, padding: 0, fontSize: 18 }} icon={<PushpinOutlined />} onClick={handlePin} /></Tooltip>
            <Tooltip title="?? Ctrl+Z"><Button size="middle" style={{ width: 36, height: 36, padding: 0, fontSize: 18 }} disabled={annotations.length === 0} icon={<UndoOutlined />} onClick={undoAnnotation} /></Tooltip>
            <Tooltip title="?? Ctrl+Y / Ctrl+Shift+Z"><Button size="middle" style={{ width: 36, height: 36, padding: 0, fontSize: 18 }} disabled={redoAnnotations.length === 0} icon={<RedoOutlined />} onClick={redoAnnotation} /></Tooltip>
            <Tooltip title="??"><Button size="middle" style={{ width: 36, height: 36, padding: 0 }} onClick={handlePreview} icon={<span style={{ fontSize: 18 }}>??</span>} /></Tooltip>
            <Tooltip title="??"><Button size="middle" style={{ width: 36, height: 36, padding: 0, fontSize: 18 }} icon={<SaveOutlined />} onClick={() => confirmScreenshot("save")} /></Tooltip>
            <Tooltip title="??"><Button size="middle" style={{ width: 42, height: 36, padding: 0, color: "#ef4444", borderColor: "#fca5a5", background: "#fff1f2", fontSize: 20, borderRadius: 10 }} icon={<CloseOutlined />} onClick={cancelScreenshot} /></Tooltip>
            <Tooltip title="?????"><Button size="middle" style={{ width: 42, height: 36, padding: 0, color: "#fff", background: "#16a34a", borderColor: "#16a34a", fontSize: 20, borderRadius: 10, boxShadow: "0 4px 12px rgba(22,163,74,0.28)" }} icon={<CheckOutlined />} onClick={() => confirmScreenshot("copy")} /></Tooltip>
          </Space>
          {isEditing && (
            <div style={{ marginTop: 8, display: "flex", alignItems: "center", gap: 8, color: "#ffffff", fontSize: 12, textShadow: "0 1px 2px rgba(0,0,0,0.45)" }}>
              <span>??</span>
              <InputNumber size="small" min={1} max={48} value={annotationSize} onChange={(value) => setAnnotationSize(Number(value || 1))} style={{ width: 74 }} />
              {annotationTool !== "mosaic" && (<><span>??</span><input type="color" value={annotationColor} onChange={(event) => setAnnotationColor(event.target.value)} style={{ width: 30, height: 26, padding: 0, border: "1px solid #d9d9d9", borderRadius: 5, background: "#fff" }} /></>)}
              <span>{annotationTool === "text" ? "???????????????????" : (annotationTool === "rect" || annotationTool === "circle") ? "??/??????" : "????????"}</span>
            </div>
          )}
        </div>
      )}

      {hasSelected && !isSelecting && translatePairs !== null && (
        <div
          style={(() => {
            const panelWidth = 350;
            const panelGap = 12;
            const rightLeft = rect.x + rect.w + panelGap;
            const leftLeft = rect.x - panelWidth - panelGap;
            const hasRightSpace = rightLeft + panelWidth <= window.innerWidth - 8;
            const hasLeftSpace = leftLeft >= 8;
            const left = hasRightSpace ? rightLeft : hasLeftSpace ? leftLeft : clamp(rightLeft, 8, window.innerWidth - panelWidth - 8);
            return { position: "absolute", top: Math.max(8, Math.min(rect.y, window.innerHeight - 360)), left, width: panelWidth, maxHeight: "80vh", overflowY: "auto", zIndex: 120, background: "#fff", padding: 12, borderRadius: 10, boxShadow: "0 6px 24px rgba(0, 0, 0, 0.18)", border: "1px solid #e8e8e8" } as React.CSSProperties;
          })()}
          onMouseDown={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.stopPropagation()}
        >
          <div style={{ marginBottom: 12, fontWeight: "bold", fontSize: 14 }}>??????</div>
          <div style={{ display: "flex", flexDirection: "column", gap: 10, marginBottom: 12 }}>
            {translatePairs.map((p, i) => (
              <div key={i} style={{ padding: 8, background: "#f5f5f5", borderRadius: 6, fontSize: 12 }}>
                <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 8, marginBottom: 6 }}><div style={{ color: "#8c8c8c", lineHeight: 1.45, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>{p.o}</div><Button size="small" onClick={() => navigator.clipboard.writeText(p.o)}>????</Button></div>
                <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 8 }}><div style={{ color: "#1f1f1f", fontWeight: "bold", lineHeight: 1.45, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>{p.t}</div><Button size="small" type="primary" ghost onClick={() => navigator.clipboard.writeText(p.t)}>????</Button></div>
              </div>
            ))}
          </div>
          <Space size="small">
            <Button size="small" type="primary" icon={<CopyOutlined />} onClick={() => navigator.clipboard.writeText(translatePairs.map(p => p.t).join("\n"))}>??????</Button>
            <Button size="small" icon={<CloseOutlined />} onClick={() => setTranslatePairs(null)}>??</Button>
          </Space>
        </div>
      )}
              关闭
            </Button>
          </Space>
        </div>
      )}

    </div>
  );
}
