import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";
import { Button, Space, message } from "antd";
import { CopyOutlined, SaveOutlined, CloseOutlined, CheckOutlined } from "@ant-design/icons";

export default function ScreenshotPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [imgSrc, setImgSrc] = useState<string>("");
  const [isSelecting, setIsSelecting] = useState(false);
  const [startPos, setStartPos] = useState({ x: 0, y: 0 });
  const [rect, setRect] = useState({ x: 0, y: 0, w: 0, h: 0 });
  const [hasSelected, setHasSelected] = useState(false);

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

  useEffect(() => {
    // 1. Force background to be transparent to prevent default solid white background
    const origBodyBg = document.body.style.backgroundColor;
    const origHtmlBg = document.documentElement.style.backgroundColor;
    
    document.body.style.setProperty("background-color", "transparent", "important");
    document.documentElement.style.setProperty("background-color", "transparent", "important");

    // 2. Load the initial screenshot
    loadFullscreen();

    // 3. Listen for subsequent screenshot updates
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

    // 4. Global keyboard listener for Esc and Enter
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        cancelScreenshot();
      }
      if (e.key === "Enter" && hasSelectedRef.current) {
        confirmScreenshot("copy");
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.backgroundColor = origBodyBg;
      document.documentElement.style.backgroundColor = origHtmlBg;
      window.removeEventListener("keydown", handleKeyDown);
      if (unlistenEvent) {
        unlistenEvent();
      }
    };
  }, []);

  const loadFullscreen = async () => {
    try {
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
        
        // Setup canvas sizing and draw initial dark overlay
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
    
    // Reset selection state when image reloads
    setRect({ x: 0, y: 0, w: 0, h: 0 });
    setHasSelected(false);
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (e.button === 2) { // Right click cancels screenshot
      e.preventDefault();
      cancelScreenshot();
      return;
    }
    setIsSelecting(true);
    setStartPos({ x: e.clientX, y: e.clientY });
    setRect({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
    setHasSelected(false);
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!isSelecting) return;
    const x = Math.min(startPos.x, e.clientX);
    const y = Math.min(startPos.y, e.clientY);
    const w = Math.abs(startPos.x - e.clientX);
    const h = Math.abs(startPos.y - e.clientY);
    setRect({ x, y, w, h });
    draw(x, y, w, h);
  };

  const handleMouseUp = () => {
    if (!isSelecting) return;
    setIsSelecting(false);
    if (rect.w > 5 && rect.h > 5) {
      setHasSelected(true);
    } else {
      setHasSelected(false);
    }
  };

  const draw = (rx: number, ry: number, rw: number, rh: number) => {
    const canvas = canvasRef.current;
    if (!canvas || !imageRef.current) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    
    // Smooth frame rendering from cached Image object
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(imageRef.current, 0, 0, canvas.width, canvas.height);
    
    // Dark mask layer
    ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    
    if (rw > 0 && rh > 0) {
      // Clear mask inside selection
      ctx.clearRect(rx, ry, rw, rh);
      ctx.drawImage(imageRef.current, rx, ry, rw, rh, rx, ry, rw, rh);
      
      // Selection blue borders
      ctx.strokeStyle = "#1677ff";
      ctx.lineWidth = 2;
      ctx.strokeRect(rx, ry, rw, rh);
      
      // Coordinate display label
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

  const cancelScreenshot = async () => {
    const mainWin = new WebviewWindow("main");
    await mainWin.show();
    await mainWin.setFocus();
    
    const win = getCurrentWindow();
    await win.hide();
  };

  const confirmScreenshot = async (action: "copy" | "save" | "both") => {
    try {
      const dpr = window.devicePixelRatio || 1;
      const physicalX = Math.round(rect.x * dpr);
      const physicalY = Math.round(rect.y * dpr);
      const physicalW = Math.round(rect.w * dpr);
      const physicalH = Math.round(rect.h * dpr);

      const base64 = await invoke<string>("capture_region", {
        x: physicalX,
        y: physicalY,
        w: physicalW,
        h: physicalH,
      });

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
            return; // Exit early, keep screenshot window open
          } else {
            throw saveErr;
          }
        }
      }

      // Show the main window and deliver data
      const mainWin = new WebviewWindow("main");
      await mainWin.show();
      await mainWin.setFocus();

      const win = getCurrentWindow();
      await win.emitTo("main", "screenshot-captured", base64);
      await win.hide();
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
      {/* High-visibility Debug Info HUD */}
      <div style={{
        position: "absolute",
        top: 12,
        left: 12,
        zIndex: 9999,
        background: "rgba(0, 0, 0, 0.8)",
        color: "#00ff00",
        padding: "8px 14px",
        borderRadius: "6px",
        fontSize: "11px",
        fontFamily: "Consolas, Monaco, monospace",
        pointerEvents: "none",
        border: "1px solid #00ff00",
        lineHeight: "1.5",
        boxShadow: "0 4px 12px rgba(0,0,0,0.5)"
      }}>
        <div style={{ fontWeight: "bold", borderBottom: "1px solid #00ff00", marginBottom: "4px", paddingBottom: "2px" }}>HUD DEBUG</div>
        <div>imageLoaded: {dbgStatus.imageLoaded ? "true" : "false"}</div>
        <div>imageSize: {dbgStatus.imageWidth} x {dbgStatus.imageHeight}</div>
        <div>screenshotBytes: {dbgStatus.screenshotBytes} bytes</div>
        {dbgStatus.errorMsg ? (
          <div style={{ color: "#ff4d4f", marginTop: "4px", fontWeight: "bold" }}>Error: {dbgStatus.errorMsg}</div>
        ) : (
          <div style={{ color: "#00ff00", marginTop: "4px" }}>Status: Normal</div>
        )}
      </div>

      {/* Load error screen fallback placeholder */}
      {!dbgStatus.imageLoaded && dbgStatus.errorMsg && (
        <div style={{
          position: "absolute",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          background: "rgba(0, 0, 0, 0.9)",
          color: "#ffffff",
          padding: "28px 36px",
          borderRadius: "12px",
          textAlign: "center",
          border: "2px solid #ff4d4f",
          zIndex: 10000,
          maxWidth: "80%",
          boxShadow: "0 8px 32px rgba(0,0,0,0.5)"
        }}>
          <h3 style={{ color: "#ff4d4f", margin: "0 0 12px 0", fontSize: "16px" }}>截图图像加载失败</h3>
          <p style={{ margin: "0 0 20px 0", fontSize: "13px", opacity: 0.85, wordBreak: "break-all" }}>{dbgStatus.errorMsg}</p>
          <Button type="primary" danger onClick={cancelScreenshot}>关闭截图</Button>
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
            left: Math.max(8, rect.x + rect.w - 240),
            zIndex: 100,
            background: "#ffffff",
            padding: "4px 8px",
            borderRadius: 6,
            boxShadow: "0 2px 10px rgba(0, 0, 0, 0.15)",
            border: "1px solid #f0f0f0"
          }}
          onContextMenu={(e) => e.stopPropagation()} // Prevent cancelling when right-clicking on menu itself
        >
          <Space size="small">
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
