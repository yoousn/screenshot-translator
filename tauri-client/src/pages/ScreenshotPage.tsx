import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { Button, Space, message } from "antd";
import {
  CopyOutlined,
  SaveOutlined,
  CloseOutlined,
  CheckOutlined,
  TranslationOutlined,
  PushpinOutlined,
  ScanOutlined,
} from "@ant-design/icons";

interface Config {
  serverUrl?: string;
  clientToken?: string;
}

export default function ScreenshotPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const mouseTrackerRef = useRef<HTMLDivElement>(null);
  const [imgSrc, setImgSrc] = useState<string>("");
  const [isSelecting, setIsSelecting] = useState(false);
  const [isDragging, setIsDragging] = useState(false);
  const [startPos, setStartPos] = useState({ x: 0, y: 0 });
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [rect, setRect] = useState({ x: 0, y: 0, w: 0, h: 0 });
  const [hasSelected, setHasSelected] = useState(false);
  const [windowRects, setWindowRects] = useState<Array<{ x: number; y: number; w: number; h: number }>>([]);
  const [screenshotMode, setScreenshotMode] = useState<string>("normal");
  const [isTranslating, setIsTranslating] = useState(false);
  const [isOCRing, setIsOCRing] = useState(false);
  const [config, setConfig] = useState<Config>({});

  // Debug Panel States
  const [dbgStatus, setDbgStatus] = useState({
    imageLoaded: false,
    imageWidth: 0,
    imageHeight: 0,
    screenshotBytes: 0,
    errorMsg: ""
  });

  const imageRef = useRef<HTMLImageElement | null>(null);
  const hasSelectedRef = useRef(false);
  hasSelectedRef.current = hasSelected;
  const rectRef = useRef({ x: 0, y: 0, w: 0, h: 0 });
  rectRef.current = rect;
  const configRef = useRef<Config>({});
  configRef.current = config;

  useEffect(() => {
    // Load config for server API calls
    loadConfig();

    // 1. Force background to be transparent
    const origBodyBg = document.body.style.backgroundColor;
    const origHtmlBg = document.documentElement.style.backgroundColor;
    document.body.style.setProperty("background-color", "transparent", "important");
    document.documentElement.style.setProperty("background-color", "transparent", "important");

    // 2. Load window rects for UI snapping
    loadWindowRects();

    // 3. Load the initial screenshot
    loadFullscreen();

    // 3. Listen for screenshot-mode event
    let unlistenMode: (() => void) | null = null;
    const setupModeListener = async () => {
      try {
        const unsub = await listen<string>("screenshot-mode", (event) => {
          setScreenshotMode(event.payload);
        });
        unlistenMode = unsub;
      } catch (err) {
        console.error("Failed to listen to screenshot-mode", err);
      }
    };
    setupModeListener();

    // 4. Listen for screenshot-updated
    let unlistenEvent: (() => void) | null = null;
    const setupListener = async () => {
      try {
        const unsub = await listen("screenshot-updated", () => {
          loadFullscreen();
        });
        unlistenEvent = unsub;
      } catch (err) {
        console.error("Failed to listen to screenshot-updated", err);
      }
    };
    setupListener();

    // 5. Global keyboard listener for Esc, Enter, Ctrl+Q
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        cancelScreenshot();
      }
      if (e.key === "Enter" && hasSelectedRef.current) {
        confirmScreenshot("copy");
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "q") {
        e.preventDefault();
        if (hasSelectedRef.current) {
          handleTranslate();
        } else {
          message.info("请先框选需要翻译的区域");
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.backgroundColor = origBodyBg;
      document.documentElement.style.backgroundColor = origHtmlBg;
      window.removeEventListener("keydown", handleKeyDown);
      if (unlistenEvent) unlistenEvent();
      if (unlistenMode) unlistenMode();
    };
  }, []);

  const loadConfig = async () => {
    try {
      const configStr = await invoke<string>("get_config");
      const parsed = JSON.parse(configStr);
      setConfig(parsed);
    } catch (err) {
      console.error("Failed to load config in ScreenshotPage", err);
    }
  };

  const loadWindowRects = async () => {
    try {
      const raw = await invoke<string>("get_window_rects");
      const rects: Array<{ x: number; y: number; w: number; h: number }> = JSON.parse(raw);
      setWindowRects(rects);
    } catch (_) {
      setWindowRects([]);
    }
  };

  const loadFullscreen = async () => {
    try {
      // Clear old screenshot immediately so canvas reflects a fresh state
      imageRef.current = null;
      setImgSrc("");
      setRect({ x: 0, y: 0, w: 0, h: 0 });
      setHasSelected(false);
      setIsDragging(false);
      setTranslatedResult(null);

      // Re-detect window bounds for current mouse position
      loadWindowRects();

      setDbgStatus(prev => ({ ...prev, errorMsg: "", imageLoaded: false }));
      const base64 = await invoke<string>("get_fullscreen_image");
      if (!base64 || base64.length === 0) {
        throw new Error("截屏Base64数据为空");
      }
      const dataUrl = "data:image/png;base64," + base64;
      const img = new Image();
      img.src = dataUrl;
      img.onload = () => {
        imageRef.current = img;
        setImgSrc(dataUrl);
        setDbgStatus({
          imageLoaded: true,
          imageWidth: img.naturalWidth,
          imageHeight: img.naturalHeight,
          screenshotBytes: Math.round(base64.length * 0.75),
          errorMsg: ""
        });
        initCanvas(img);
      };
      img.onerror = () => {
        throw new Error("HTML Image 元素解码 Base64 截图字节流失败");
      };
    } catch (err: any) {
      const msg = err.toString();
      setDbgStatus(prev => ({ ...prev, errorMsg: msg, imageLoaded: false }));
      message.error("加载截屏图像失败: " + msg);
    }
  };

  const initCanvas = (img: HTMLImageElement) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    setRect({ x: 0, y: 0, w: 0, h: 0 });
    setHasSelected(false);
  };

  const snap = (val: number, refs: number[]) => {
    const dist = 15;
    for (const r of refs) {
      if (Math.abs(val - r) < dist) return r;
    }
    return val;
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (e.button === 2) {
      e.preventDefault();
      cancelScreenshot();
      return;
    }
    if (hasSelectedRef.current) {
      const cx = e.clientX, cy = e.clientY;
      const { x, y, w, h } = rectRef.current;
      const inside = cx >= x && cx <= x + w && cy >= y && cy <= y + h;
      if (!inside) {
        // Click outside → auto copy to clipboard and exit
        confirmScreenshot("copy");
      } else {
        // Click inside → start dragging the selection
        setIsDragging(true);
        setDragStart({ x: cx, y: cy });
      }
      return;
    }
    setIsSelecting(true);
    setStartPos({ x: e.clientX, y: e.clientY });
    rectRef.current = { x: e.clientX, y: e.clientY, w: 0, h: 0 };
    setRect({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
    setHasSelected(false);
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    // Update mouse coordinate tracker directly via DOM reference to ensure zero-lag performance
    if (mouseTrackerRef.current) {
      mouseTrackerRef.current.style.left = `${e.clientX + 16}px`;
      mouseTrackerRef.current.style.top = `${e.clientY + 20}px`;
      mouseTrackerRef.current.textContent = `${e.clientX}, ${e.clientY}${
        hasSelectedRef.current ? ` | ${rectRef.current.w}×${rectRef.current.h}` : ""
      }`;
    }

    if (isDragging) {
      const dx = e.clientX - dragStart.x;
      const dy = e.clientY - dragStart.y;
      setDragStart({ x: e.clientX, y: e.clientY });
      
      rectRef.current = {
        x: Math.max(0, rectRef.current.x + dx),
        y: Math.max(0, rectRef.current.y + dy),
        w: rectRef.current.w,
        h: rectRef.current.h,
      };
      
      draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h);
      return;
    }
    if (!isSelecting) return;
    // Build snap reference points from window rects
    const snapX: number[] = [];
    const snapY: number[] = [];
    for (const wr of windowRects) {
      snapX.push(wr.x, wr.x + wr.w);
      snapY.push(wr.y, wr.y + wr.h);
    }
    let cx = snap(e.clientX, snapX);
    let cy = snap(e.clientY, snapY);
    const x = Math.min(startPos.x, cx);
    const y = Math.min(startPos.y, cy);
    const w = Math.abs(startPos.x - cx);
    const h = Math.abs(startPos.y - cy);
    
    rectRef.current = { x, y, w, h };
    draw(x, y, w, h);
  };

  const handleMouseUp = () => {
    if (isDragging) {
      setIsDragging(false);
      // Sync final dragged coordinates to React state to display the toolbar
      setRect({ ...rectRef.current });
      return;
    }
    if (!isSelecting) return;
    setIsSelecting(false);
    
    // Sync final selected coordinates to React state to display the toolbar
    setRect({ ...rectRef.current });
    
    if (rectRef.current.w > 5 && rectRef.current.h > 5) {
      setHasSelected(true);
    } else {
      setHasSelected(false);
    }
  };

  const draw = (rx: number, ry: number, rw: number, rh: number, translatedImg?: HTMLImageElement) => {
    const canvas = canvasRef.current;
    if (!canvas || !imageRef.current) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(imageRef.current, 0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    
    // Draw detected window bounds as subtle snap hints
    if (windowRects.length > 0) {
      ctx.strokeStyle = "rgba(82, 196, 26, 0.35)";
      ctx.lineWidth = 1;
      ctx.setLineDash([4, 4]);
      for (const wr of windowRects) {
        ctx.strokeRect(wr.x, wr.y, wr.w, wr.h);
      }
      ctx.setLineDash([]);
    }
    
    if (rw > 0 && rh > 0) {
      ctx.clearRect(rx, ry, rw, rh);
      if (translatedImg) {
        ctx.drawImage(translatedImg, rx, ry, rw, rh);
      } else {
        ctx.drawImage(imageRef.current, rx, ry, rw, rh, rx, ry, rw, rh);
      }
      ctx.strokeStyle = "#1677ff";
      ctx.lineWidth = 2;
      ctx.strokeRect(rx, ry, rw, rh);
      ctx.fillStyle = "rgba(22, 119, 255, 0.85)";
      ctx.font = "12px sans-serif";
      const text = `${rw} x ${rh}`;
      const textWidth = ctx.measureText(text).width;
      const tipY = ry - 22 >= 0 ? ry - 22 : ry + rh + 4;
      ctx.fillRect(rx, tipY, textWidth + 12, 20);
      ctx.fillStyle = "#ffffff";
      ctx.fillText(text, rx + 6, tipY + 14);
    }
  };

  const captureRegionBase64 = async (): Promise<string> => {
    const dpr = window.devicePixelRatio || 1;
    const physicalX = Math.round(rectRef.current.x * dpr);
    const physicalY = Math.round(rectRef.current.y * dpr);
    const physicalW = Math.round(rectRef.current.w * dpr);
    const physicalH = Math.round(rectRef.current.h * dpr);
    return await invoke<string>("capture_region", {
      x: physicalX, y: physicalY, w: physicalW, h: physicalH,
    });
  };

  // --- Translate: send region to server, overlay translated result ---
  const handleTranslate = async () => {
    const serverUrl = configRef.current.serverUrl || "https://ocr.yousn.me";
    const token = configRef.current.clientToken || "";
    try {
      setIsTranslating(true);
      message.loading({ content: "正在请求翻译重绘...", key: "translate", duration: 0 });
      const base64 = await captureRegionBase64();
      const resp = await fetch(`${serverUrl.replace(/\/$/, "")}/api/translate`, {
        method: "POST",
        headers: { "x-api-key": token },
        body: (() => {
          const byteChars = atob(base64);
          const byteNums = new Array(byteChars.length);
          for (let i = 0; i < byteChars.length; i++) byteNums[i] = byteChars.charCodeAt(i);
          const byteArr = new Uint8Array(byteNums);
          const blob = new Blob([byteArr], { type: "image/png" });
          const fd = new FormData();
          fd.append("image", blob, "region.png");
          return fd;
        })(),
      });
      if (!resp.ok) {
        const errText = await resp.text();
        throw new Error(errText || `HTTP ${resp.status}`);
      }
      const resultBlob = await resp.blob();
      const resultUrl = URL.createObjectURL(resultBlob);
      const overlayImg = new Image();
      overlayImg.src = resultUrl;
      overlayImg.onload = () => {
        // Redraw canvas with translated image overlay in the selected region
        draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h, overlayImg);
        
        // Store result for later copy/save via confirmScreenshot
        const reader = new FileReader();
        reader.onloadend = async () => {
          const resultBase64 = (reader.result as string).split(",")[1];
          setTranslatedResult(resultBase64);
          
          try {
            // Automatically copy translated image to clipboard
            await invoke("copy_image_to_clipboard", { imageBase64: resultBase64 });
            
            // Automatically pin the translation at the exact physical screen position!
            const dpr = window.devicePixelRatio || 1;
            const winPos = await getCurrentWindow().outerPosition();
            const physicalX = Math.round(rectRef.current.x * dpr) + winPos.x;
            const physicalY = Math.round(rectRef.current.y * dpr) + winPos.y;
            const physicalW = Math.round(rectRef.current.w * dpr);
            const physicalH = Math.round(rectRef.current.h * dpr);

            await invoke("create_pin_window", { 
              imageBase64: resultBase64,
              x: physicalX,
              y: physicalY,
              w: physicalW,
              h: physicalH
            });
            
            message.success({ content: "翻译并贴图完成，已复制到剪贴板！", key: "translate" });
            
            // Auto close/hide screenshot window
            await invoke("cancel_screenshot");
          } catch (pinErr: any) {
            message.error({ content: `自动贴图或复制失败: ${pinErr.toString()}`, key: "translate" });
          }
        };
        reader.readAsDataURL(resultBlob);
        setIsTranslating(false);
      };
      overlayImg.onerror = () => {
        throw new Error("翻译结果图片解码失败");
      };
    } catch (e: any) {
      message.error({ content: `翻译失败: ${e.message}`, key: "translate" });
      setIsTranslating(false);
    }
  };
  const [translatedResult, setTranslatedResult] = useState<string | null>(null);

  // --- OCR: send region to server, copy text to clipboard ---
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
        body: (() => {
          const byteChars = atob(base64);
          const byteNums = new Array(byteChars.length);
          for (let i = 0; i < byteChars.length; i++) byteNums[i] = byteChars.charCodeAt(i);
          const byteArr = new Uint8Array(byteNums);
          const blob = new Blob([byteArr], { type: "image/png" });
          const fd = new FormData();
          fd.append("image", blob, "region.png");
          return fd;
        })(),
      });
      if (!resp.ok) {
        const errText = await resp.text();
        throw new Error(errText || `HTTP ${resp.status}`);
      }
      const data = await resp.json();
      if (data.status === "success" && data.ocr && data.ocr.length > 0) {
        const texts = data.ocr.map((item: any) => item.text).join("\n");
        await navigator.clipboard.writeText(texts);
        message.success({ content: `识别到 ${data.ocr.length} 条文本，已复制到剪贴板`, key: "ocr" });
      } else {
        message.info({ content: "未识别到任何文字", key: "ocr" });
      }
    } catch (e: any) {
      message.error({ content: `OCR 失败: ${e.message}`, key: "ocr" });
    } finally {
      setIsOCRing(false);
    }
  };

  // --- Pin: create floating window with selected region ---
  const handlePin = async () => {
    try {
      const dpr = window.devicePixelRatio || 1;
      const winPos = await getCurrentWindow().outerPosition();
      const physicalX = Math.round(rectRef.current.x * dpr) + winPos.x;
      const physicalY = Math.round(rectRef.current.y * dpr) + winPos.y;
      const physicalW = Math.round(rectRef.current.w * dpr);
      const physicalH = Math.round(rectRef.current.h * dpr);

      const base64 = await captureRegionBase64();
      await invoke("create_pin_window", { 
        imageBase64: base64,
        x: physicalX,
        y: physicalY,
        w: physicalW,
        h: physicalH
      });
      message.success("已创建贴图窗口");
      await invoke("cancel_screenshot");
    } catch (e: any) {
      message.error("贴图失败: " + e.toString());
    }
  };

  const cancelScreenshot = async () => {
    try {
      await invoke("cancel_screenshot");
    } catch (err) {
      console.error("Failed to cancel screenshot:", err);
    }
  };

  const confirmScreenshot = async (action: "copy" | "save" | "both") => {
    try {
      let base64: string;
      // If we have a translated result and action is copy/both, use translated image
      if (translatedResult && (action === "copy" || action === "both")) {
        base64 = translatedResult;
      } else {
        base64 = await captureRegionBase64();
      }

      if (action === "copy" || action === "both") {
        await invoke("copy_image_to_clipboard", { imageBase64: base64 });
        message.success("图片已成功复制至剪贴板");
      }
      if (action === "save") {
        try {
          const savePath = await invoke<string>("save_image_to_file", { imageBase64: base64 });
          message.success(`图片成功保存至: ${savePath}`);
        } catch (saveErr) {
          if (saveErr === "用户取消了保存") {
            message.info("已取消保存");
            return;
          } else {
            throw saveErr;
          }
        }
      }

      // Just hide — no popup window, no animation
      await invoke("cancel_screenshot");
    } catch (e: any) {
      message.error("截图操作失败: " + e.toString());
    }
  };

  return (
    <div
      style={{
        position: "relative",
        width: "100vw",
        height: "100vh",
        overflow: "hidden",
        userSelect: "none"
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        cancelScreenshot();
      }}
    >
      {/* Mouse coordinate tracker */}
      <div
        ref={mouseTrackerRef}
        style={{
          position: "absolute", top: -100, left: -100, zIndex: 9999,
          background: "rgba(0, 0, 0, 0.75)", color: "#fff",
          padding: "2px 8px", borderRadius: "4px",
          fontSize: "11px", fontFamily: "Consolas, Monaco, monospace",
          pointerEvents: "none", whiteSpace: "nowrap",
          lineHeight: "18px"
        }}
      >
        0, 0
      </div>

      {/* Load error fallback */}
      {!dbgStatus.imageLoaded && dbgStatus.errorMsg && (
        <div style={{
          position: "absolute", top: "50%", left: "50%",
          transform: "translate(-50%, -50%)",
          background: "rgba(0, 0, 0, 0.9)", color: "#ffffff",
          padding: "28px 36px", borderRadius: "12px",
          textAlign: "center", border: "2px solid #ff4d4f",
          zIndex: 10000, maxWidth: "80%",
          boxShadow: "0 8px 32px rgba(0,0,0,0.5)"
        }}>
          <h3 style={{ color: "#ff4d4f", margin: "0 0 12px 0", fontSize: "16px" }}>截图图像加载失败</h3>
          <p style={{ margin: "0 0 20px 0", fontSize: "13px", opacity: 0.85, wordBreak: "break-all" }}>{dbgStatus.errorMsg}</p>
          <Button type="primary" danger onClick={cancelScreenshot}>关闭截图</Button>
        </div>
      )}

      {/* Translation loading: spinning overlay with light gray bg */}
      {isTranslating && (
        <div style={{
          position: "absolute", top: 0, left: 0, width: "100vw", height: "100vh",
          zIndex: 200, background: "rgba(200, 200, 210, 0.55)",
          display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center",
        }}>
          <div style={{
            width: 48, height: 48, borderRadius: "50%",
            border: "4px solid #e0e0e0", borderTopColor: "#1677ff",
            animation: "spin 0.8s linear infinite",
          }} />
          <div style={{
            marginTop: 16, color: "#333", fontSize: 13,
            fontFamily: "'Inter', sans-serif", fontWeight: 500,
          }}>
            正在翻译重绘中…
          </div>
        </div>
      )}

      <canvas
        ref={canvasRef}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        style={{ position: "absolute", top: 0, left: 0, zIndex: 10, cursor: "crosshair" }}
      />

      {hasSelected && !isSelecting && (
        <div
          style={{
            position: "absolute",
            top: rect.y + rect.h + 8 + 36 > window.innerHeight ? rect.y - 44 : rect.y + rect.h + 8,
            left: Math.max(8, Math.min(rect.x + rect.w - 480, window.innerWidth - 496)),
            zIndex: 100,
            background: "#ffffff",
            padding: "6px 10px",
            borderRadius: 8,
            boxShadow: "0 2px 12px rgba(0, 0, 0, 0.12)",
            border: "1px solid #e8e8e8"
          }}
          onContextMenu={(e) => e.stopPropagation()}
        >
          <Space size="small" wrap>
            <Button size="small" icon={<TranslationOutlined />} type="primary" ghost
              onClick={handleTranslate} loading={isTranslating}
            >翻译 (Ctrl+Q)</Button>
            <Button size="small" icon={<PushpinOutlined />} onClick={handlePin}>贴图</Button>
            <Button size="small" icon={<ScanOutlined />} onClick={handleOCR} loading={isOCRing}>识字</Button>
            <Button size="small" icon={<CopyOutlined />} onClick={() => confirmScreenshot("copy")}>复制</Button>
            <Button size="small" icon={<SaveOutlined />} onClick={() => confirmScreenshot("save")}>保存</Button>
            <Button size="small" type="primary" icon={<CheckOutlined />} onClick={() => confirmScreenshot("both")}>完成</Button>
            <Button size="small" icon={<CloseOutlined />} onClick={cancelScreenshot} danger />
          </Space>
        </div>
      )}
    </div>
  );
}
