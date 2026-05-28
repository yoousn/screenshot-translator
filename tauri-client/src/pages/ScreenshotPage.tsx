import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button, Space, message, Input } from "antd";
import {
  CopyOutlined,
  SaveOutlined,
  CloseOutlined,
  CheckOutlined,
  TranslationOutlined,
  ScanOutlined,
} from "@ant-design/icons";

interface Config {
  serverUrl?: string;
  clientToken?: string;
}

type Rect = { x: number; y: number; w: number; h: number };

const EMPTY_RECT: Rect = { x: 0, y: 0, w: 0, h: 0 };

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
  const [isSelecting, setIsSelecting] = useState(false);
  const [rect, setRect] = useState<Rect>(EMPTY_RECT);
  const [hasSelected, setHasSelected] = useState(false);
  const [windowRects, setWindowRects] = useState<Rect[]>([]);
  const [screenshotMode, setScreenshotMode] = useState("normal");
  const [isTranslating, setIsTranslating] = useState(false);
  const [isOCRing, setIsOCRing] = useState(false);
  const [config, setConfig] = useState<Config>({});
  const [translatedResult, setTranslatedResult] = useState<string | null>(null);
  const [ocrResultText, setOcrResultText] = useState<string | null>(null);
  const [ocrPreviewBase64, setOcrPreviewBase64] = useState<string | null>(null);
  const [dbgStatus, setDbgStatus] = useState({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });

  const imageRef = useRef<HTMLImageElement | null>(null);
  const translatedImgRef = useRef<HTMLImageElement | null>(null);
  const hasSelectedRef = useRef(false);
  const rectRef = useRef<Rect>(EMPTY_RECT);
  const configRef = useRef<Config>({});
  const screenshotModeRef = useRef("normal");
  const isSelectingRef = useRef(false);
  const isDraggingRef = useRef(false);
  const isResizingRef = useRef<string | null>(null);
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
      if (e.key === "Escape") {
        cancelScreenshot();
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
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      if (unlistenEvent) unlistenEvent();
      if (unlistenMode) unlistenMode();
      if (requestRef.current) cancelAnimationFrame(requestRef.current);
    };
  }, []);

  const loadConfig = async () => {
    try {
      setConfig(JSON.parse(await invoke<string>("get_config")));
    } catch {
      setConfig({});
    }
  };

  const loadWindowRects = async () => {
    try {
      setWindowRects(JSON.parse(await invoke<string>("get_window_rects")));
    } catch {
      setWindowRects([]);
    }
  };

  const loadFullscreen = async () => {
    try {
      imageRef.current = null;
      translatedImgRef.current = null;
      setTranslatedResult(null);
      setOcrResultText(null);
      setOcrPreviewBase64(null);
      setCurrentRect(EMPTY_RECT, true);
      setSelection(false);
      loadWindowRects();
      setDbgStatus((prev) => ({ ...prev, errorMsg: "", imageLoaded: false }));

      const base64 = await invoke<string>("get_fullscreen_image");
      if (!base64) throw new Error("截屏Base64数据为空");
      loadImageFromBase64(base64);
    } catch (err: any) {
      const msg = err?.message || err?.toString?.() || String(err);
      setDbgStatus((prev) => ({ ...prev, errorMsg: msg, imageLoaded: false }));
      message.error("加载截屏图像失败: " + msg);
    }
  };

  const loadFullscreenFromBase64 = (base64: string) => {
    try {
      imageRef.current = null;
      translatedImgRef.current = null;
      setTranslatedResult(null);
      setOcrResultText(null);
      setOcrPreviewBase64(null);
      setCurrentRect(EMPTY_RECT, true);
      setSelection(false);
      loadWindowRects();
      setDbgStatus((prev) => ({ ...prev, errorMsg: "", imageLoaded: false }));
      loadImageFromBase64(base64);
    } catch (err: any) {
      const msg = err?.message || err?.toString?.() || String(err);
      setDbgStatus((prev) => ({ ...prev, errorMsg: msg, imageLoaded: false }));
      message.error("加载截屏图像失败: " + msg);
    }
  };

  const loadImageFromBase64 = (base64: string) => {
    const dataUrl = "data:image/jpeg;base64," + base64;
    const img = new Image();
    img.onload = async () => {
      imageRef.current = img;
      setDbgStatus({ imageLoaded: true, imageWidth: img.naturalWidth, imageHeight: img.naturalHeight, screenshotBytes: Math.round(base64.length * 0.75), errorMsg: "" });
      await waitForStableViewport(img);
      initCanvas(img);
    };
    img.onerror = () => setDbgStatus((prev) => ({ ...prev, errorMsg: "HTML Image 元素解码 Base64 截图字节流失败", imageLoaded: false }));
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

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (e.button === 2) {
      e.preventDefault();
      if (hasSelectedRef.current) {
        setCurrentRect(EMPTY_RECT, true);
        setSelection(false);
        setTranslatedResult(null);
        translatedImgRef.current = null;
        setOcrResultText(null);
        setOcrPreviewBase64(null);
        renderNeededRef.current = true;
      } else {
        cancelScreenshot();
      }
      return;
    }

    const cx = e.clientX;
    const cy = e.clientY;
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

    isSelectingRef.current = true;
    setIsSelecting(true);
    startPosRef.current = { x: cx, y: cy };
    setCurrentRect({ x: cx, y: cy, w: 0, h: 0 }, true);
    setSelection(false);
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const cx = e.clientX;
    const cy = e.clientY;
    if (mouseTrackerRef.current) {
      mouseTrackerRef.current.style.left = `${cx + 16}px`;
      mouseTrackerRef.current.style.top = `${cy + 20}px`;
      mouseTrackerRef.current.textContent = `${cx}, ${cy}${hasSelectedRef.current ? ` | ${rectRef.current.w}×${rectRef.current.h}` : ""}`;
    }

    if (isDraggingRef.current) {
      const dx = cx - dragStartRef.current.x;
      const dy = cy - dragStartRef.current.y;
      dragStartRef.current = { x: cx, y: cy };
      const canvas = canvasRef.current;
      const maxW = canvas?.width || window.innerWidth;
      const maxH = canvas?.height || window.innerHeight;
      rectRef.current = {
        x: Math.max(0, Math.min(maxW - rectRef.current.w, rectRef.current.x + dx)),
        y: Math.max(0, Math.min(maxH - rectRef.current.h, rectRef.current.y + dy)),
        w: rectRef.current.w,
        h: rectRef.current.h,
      };
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
      rectRef.current = { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
      renderNeededRef.current = true;
      return;
    }

    if (isSelectingRef.current) {
      const snapX: number[] = [];
      const snapY: number[] = [];
      for (const wr of windowRects) {
        snapX.push(wr.x, wr.x + wr.w);
        snapY.push(wr.y, wr.y + wr.h);
      }
      const snapCx = snap(cx, snapX);
      const snapCy = snap(cy, snapY);
      rectRef.current = { x: Math.min(startPosRef.current.x, snapCx), y: Math.min(startPosRef.current.y, snapCy), w: Math.abs(startPosRef.current.x - snapCx), h: Math.abs(startPosRef.current.y - snapCy) };
      renderNeededRef.current = true;
      return;
    }

    const handleInfo = getHandleAt(cx, cy);
    e.currentTarget.style.cursor = handleInfo ? handleInfo.cursor : "crosshair";
  };

  const handleMouseUp = () => {
    const wasSelecting = isSelectingRef.current;
    isSelectingRef.current = false;
    setIsSelecting(false);
    isDraggingRef.current = false;
    isResizingRef.current = null;
    setCurrentRect({ ...rectRef.current }, true);
    const valid = rectRef.current.w > 5 && rectRef.current.h > 5;
    setSelection(valid);
    renderNeededRef.current = true;
    if (valid && wasSelecting && screenshotModeRef.current === "translate") {
      setTimeout(() => handleTranslate(), 0);
    }
  };

  const handleDoubleClick = () => {
    if (hasSelectedRef.current) confirmScreenshot("copy");
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
    if (windowRects.length > 0) {
      ctx.strokeStyle = "rgba(82, 196, 26, 0.35)";
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      for (const wr of windowRects) ctx.strokeRect(wr.x, wr.y, wr.w, wr.h);
      ctx.setLineDash([]);
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
      ctx.strokeStyle = "#1677ff";
      ctx.lineWidth = 2;
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

  const handleTranslate = async () => {
    const serverUrl = configRef.current.serverUrl || "https://ocr.yousn.me";
    const token = configRef.current.clientToken || "";
    try {
      setIsTranslating(true);
      message.loading({ content: "正在请求翻译重绘...", key: "translate", duration: 0 });
      const base64 = await captureRegionBase64();
      const resultBase64 = await invoke<string>("api_translate", { base64Image: base64, serverUrl, clientToken: token });
      const dataUrl = "data:image/png;base64," + resultBase64;
      const overlayImg = new Image();
      overlayImg.onload = () => {
        translatedImgRef.current = overlayImg;
        draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h, overlayImg);
        setTranslatedResult(resultBase64);
        message.success({ content: "翻译完成！", key: "translate" });
        renderNeededRef.current = true;
        setIsTranslating(false);
      };
      overlayImg.onerror = () => { throw new Error("翻译结果图片解码失败"); };
      overlayImg.src = dataUrl;
    } catch (e: any) {
      message.error({ content: `翻译失败: ${e.message || e}`, key: "translate" });
      setIsTranslating(false);
    }
  };

  const parseOCRHttpError = async (resp: Response) => {
    const contentType = resp.headers.get("content-type") || "";
    const raw = await resp.text().catch(() => "");

    if (contentType.includes("application/json")) {
      try {
        const json = JSON.parse(raw);
        return json.detail || json.error || json.message || `服务器返回 HTTP ${resp.status}`;
      } catch {
        return `服务器返回 HTTP ${resp.status}`;
      }
    }

    const text = raw
      .replace(/<script[\s\S]*?<\/script>/gi, "")
      .replace(/<style[\s\S]*?<\/style>/gi, "")
      .replace(/<[^>]+>/g, " ")
      .replace(/\s+/g, " ")
      .trim();

    if (resp.status === 502) return text ? `OCR 服务网关异常：${text}` : "OCR 服务网关异常：502 Bad Gateway";
    if (text) return `OCR 服务异常：${text}`;
    return `OCR 服务异常：HTTP ${resp.status}`;
  };

  const handleOCR = async () => {
    const serverUrl = configRef.current.serverUrl || "https://ocr.yousn.me";
    const token = configRef.current.clientToken || "";

    try {
      setIsOCRing(true);
      message.loading({ content: "正在识别文字...", key: "ocr", duration: 0 });

      const base64 = await captureRegionBase64();
      const resp = await fetch(`${serverUrl.replace(/\/$/, "")}/api/ocr`, {
        method: "POST",
        headers: { "x-api-key": token },
        body: makeImageFormData(base64),
      });

      if (!resp.ok) {
        throw new Error(await parseOCRHttpError(resp));
      }

      const data = await resp.json().catch(() => null);
      if (!data) throw new Error("OCR 服务返回内容不是有效 JSON");
      if (data.status !== "success") throw new Error(data.error || data.detail || "OCR 服务返回失败状态");

      const items = Array.isArray(data.ocr) ? data.ocr : [];
      if (items.length > 0) {
        const texts = items.map((item: any) => item.text).filter(Boolean).join("\n");
        setOcrResultText(texts);
        setOcrPreviewBase64(base64);
        message.success({ content: `识别到 ${items.length} 条文本，可编辑后复制`, key: "ocr" });
      } else {
        setOcrResultText("");
        setOcrPreviewBase64(base64);
        message.info({ content: "未识别到文字，可手动输入后复制", key: "ocr" });
      }
    } catch (e: any) {
      const msg = e?.message || e?.toString?.() || String(e);
      message.error({ content: `OCR 失败：${msg}`, key: "ocr", duration: 5 });
    } finally {
      setIsOCRing(false);
    }
  };

  const copyOCRText = async () => {
    try {
      await navigator.clipboard.writeText(ocrResultText || "");
      message.success({ content: "OCR 文本已复制到剪贴板", key: "ocr-copy" });
    } catch (e: any) {
      message.error({ content: `复制失败：${e?.message || e}`, key: "ocr-copy" });
    }
  };

  const resetScreenshotState = () => {
    setRect(EMPTY_RECT);
    setHasSelected(false);
    setTranslatedResult(null);
    setOcrResultText(null);
    setOcrPreviewBase64(null);
    setIsTranslating(false);
    setIsOCRing(false);
    setDbgStatus({ imageLoaded: false, imageWidth: 0, imageHeight: 0, screenshotBytes: 0, errorMsg: "" });
    if (imageRef.current) imageRef.current.src = "";
    if (translatedImgRef.current) translatedImgRef.current.src = "";
  };

  const cancelScreenshot = async () => {
    resetScreenshotState();
    await invoke("cancel_screenshot").catch(() => {});
  };

  const confirmScreenshot = async (action: "copy" | "save" | "both") => {
    try {
      const base64 = translatedResult || await captureRegionBase64();
      if (action === "copy" || action === "both") {
        await invoke("copy_image_to_clipboard", { imageBase64: base64 });
        message.success("图片已成功复制至剪贴板");
      }
      if (action === "save") {
        const savePath = await invoke<string>("save_image_to_file", { imageBase64: base64 });
        message.success(`图片成功保存至: ${savePath}`);
      }
      resetScreenshotState();
      await invoke("cancel_screenshot");
    } catch (e: any) {
      if (e === "用户取消了保存") message.info("已取消保存");
      else message.error("截图操作失败: " + (e.message || e.toString()));
    }
  };

  return (
    <div style={{ position: "relative", width: "100vw", height: "100vh", overflow: "hidden", userSelect: "none" }} onContextMenu={(e) => { e.preventDefault(); cancelScreenshot(); }}>
      <div ref={mouseTrackerRef} style={{ position: "absolute", top: -100, left: -100, zIndex: 9999, background: "rgba(0, 0, 0, 0.75)", color: "#fff", padding: "2px 8px", borderRadius: "4px", fontSize: "11px", fontFamily: "Consolas, Monaco, monospace", pointerEvents: "none", whiteSpace: "nowrap", lineHeight: "18px" }}>0, 0</div>

      {!dbgStatus.imageLoaded && dbgStatus.errorMsg && (
        <div style={{ position: "absolute", top: "50%", left: "50%", transform: "translate(-50%, -50%)", background: "rgba(0, 0, 0, 0.9)", color: "#fff", padding: "28px 36px", borderRadius: 12, textAlign: "center", border: "2px solid #ff4d4f", zIndex: 10000, maxWidth: "80%", boxShadow: "0 8px 32px rgba(0,0,0,0.5)" }}>
          <h3 style={{ color: "#ff4d4f", margin: "0 0 12px 0", fontSize: 16 }}>截图图像加载失败</h3>
          <p style={{ margin: "0 0 20px 0", fontSize: 13, opacity: 0.85, wordBreak: "break-all" }}>{dbgStatus.errorMsg}</p>
          <Button type="primary" danger onClick={cancelScreenshot}>关闭截图</Button>
        </div>
      )}

      {isTranslating && rect.w > 0 && rect.h > 0 && (
        <div style={{ position: "absolute", top: rect.y, left: rect.x, width: rect.w, height: rect.h, zIndex: 200, background: "rgba(240, 240, 245, 0.75)", border: "2px dashed #1677ff", display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", boxSizing: "border-box", overflow: "hidden" }}>
          <div style={{ width: Math.min(32, Math.max(16, rect.w * 0.2)), height: Math.min(32, Math.max(16, rect.w * 0.2)), borderRadius: "50%", border: "3px solid #e0e0e0", borderTopColor: "#1677ff", animation: "spin 0.8s linear infinite" }} />
          {rect.h > 40 && rect.w > 80 && <div style={{ marginTop: 8, color: "#1677ff", fontSize: 12, fontFamily: "'Inter', sans-serif", fontWeight: 500, whiteSpace: "nowrap", textShadow: "0 1px 2px rgba(255,255,255,0.8)" }}>翻译中…</div>}
        </div>
      )}

      <canvas ref={canvasRef} onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} onDoubleClick={handleDoubleClick} style={{ position: "absolute", top: 0, left: 0, zIndex: 10, cursor: "crosshair" }} />

      {hasSelected && !isSelecting && (
        <div style={{ position: "absolute", top: rect.y + rect.h + 8 + 36 > window.innerHeight ? rect.y - 44 : rect.y + rect.h + 8, left: Math.max(8, Math.min(rect.x + rect.w - 480, window.innerWidth - 496)), zIndex: 100, background: "#fff", padding: "6px 10px", borderRadius: 8, boxShadow: "0 2px 12px rgba(0, 0, 0, 0.12)", border: "1px solid #e8e8e8" }} onContextMenu={(e) => e.stopPropagation()}>
          <Space size="small" wrap>
            <Button size="small" icon={<TranslationOutlined />} type="primary" ghost onClick={handleTranslate} loading={isTranslating}>翻译 (Ctrl+Q)</Button>
            <Button size="small" icon={<ScanOutlined />} onClick={handleOCR} loading={isOCRing}>识字</Button>
            <Button size="small" icon={<CopyOutlined />} onClick={() => confirmScreenshot("copy")}>复制</Button>
            <Button size="small" icon={<SaveOutlined />} onClick={() => confirmScreenshot("save")}>保存</Button>
            <Button size="small" type="primary" icon={<CheckOutlined />} onClick={() => confirmScreenshot("both")}>完成</Button>
            <Button size="small" icon={<CloseOutlined />} onClick={cancelScreenshot} danger />
          </Space>
        </div>
      )}

      {hasSelected && !isSelecting && ocrResultText !== null && (
        <div
          style={{
            position: "absolute",
            top: Math.max(8, Math.min(rect.y, window.innerHeight - 360)),
            left: Math.max(8, Math.min(rect.x + rect.w + 12, window.innerWidth - 420)),
            width: 400,
            zIndex: 120,
            background: "#fff",
            padding: 12,
            borderRadius: 10,
            boxShadow: "0 6px 24px rgba(0, 0, 0, 0.18)",
            border: "1px solid #e8e8e8",
          }}
          onMouseDown={(e) => e.stopPropagation()}
          onContextMenu={(e) => e.stopPropagation()}
        >
          <Input.TextArea
            value={ocrResultText}
            onChange={(e) => setOcrResultText(e.target.value)}
            autoSize={{ minRows: 6, maxRows: 10 }}
            placeholder="OCR 识别结果"
            style={{ marginBottom: 10 }}
          />

          {ocrPreviewBase64 && (
            <div
              style={{
                maxHeight: 140,
                overflow: "hidden",
                borderRadius: 6,
                border: "1px solid #f0f0f0",
                marginBottom: 10,
                background: "#fafafa",
                textAlign: "center",
              }}
            >
              <img
                src={`data:image/png;base64,${ocrPreviewBase64}`}
                style={{ maxWidth: "100%", maxHeight: 140, objectFit: "contain" }}
              />
            </div>
          )}

          <Space size="small">
            <Button size="small" type="primary" icon={<CopyOutlined />} onClick={copyOCRText}>
              复制文本
            </Button>
            <Button size="small" icon={<CloseOutlined />} onClick={() => setOcrResultText(null)}>
              关闭
            </Button>
          </Space>
        </div>
      )}
    </div>
  );
}
