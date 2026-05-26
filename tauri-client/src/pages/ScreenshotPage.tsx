import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button, Space, message } from "antd";
import { CopyOutlined, SaveOutlined, CloseOutlined, CheckOutlined } from "@ant-design/icons";

export default function ScreenshotPage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [imgSrc, setImgSrc] = useState<string>("");
  const [isSelecting, setIsSelecting] = useState(false);
  const [startPos, setStartPos] = useState({ x: 0, y: 0 });
  const [rect, setRect] = useState({ x: 0, y: 0, w: 0, h: 0 });
  const [hasSelected, setHasSelected] = useState(false);

  useEffect(() => {
    loadFullscreen();
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") cancelScreenshot();
      if (e.key === "Enter" && hasSelected) confirmScreenshot("copy");
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [hasSelected, rect]);

  const loadFullscreen = async () => {
    try {
      const base64 = await invoke<string>("get_fullscreen_image");
      setImgSrc("data:image/png;base64," + base64);
    } catch (err) {
      console.error("加载截屏失败", err);
    }
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (e.button === 2) { // 右键取消
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
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    
    const img = new Image();
    img.src = imgSrc;
    img.onload = () => {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
      
      // 绘制半透明蒙版层
      ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      
      // 掏空选中区域
      ctx.clearRect(rx, ry, rw, rh);
      ctx.drawImage(img, rx, ry, rw, rh, rx, ry, rw, rh);
      
      // 绘制选区细边框
      ctx.strokeStyle = "#1677ff";
      ctx.lineWidth = 1.5;
      ctx.strokeRect(rx, ry, rw, rh);
      
      // 绘制选区分辨率大小提示
      ctx.fillStyle = "rgba(22, 119, 255, 0.85)";
      ctx.font = "12px sans-serif";
      const text = `${rw} x ${rh}`;
      const textWidth = ctx.measureText(text).width;
      ctx.fillRect(rx, ry - 22 >= 0 ? ry - 22 : ry, textWidth + 10, 18);
      ctx.fillStyle = "#ffffff";
      ctx.fillText(text, rx + 5, (ry - 22 >= 0 ? ry - 9 : ry + 13));
    };
  };

  const cancelScreenshot = async () => {
    await getCurrentWindow().hide();
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
      }
      if (action === "save") {
        await invoke("save_image_to_file", { imageBase64: base64 });
      }

      // 像主窗口发送事件以拉起测试界面
      const win = getCurrentWindow();
      await win.emitTo("main", "screenshot-captured", base64);
      await win.hide();
    } catch (e) {
      message.error("截图裁剪失败: " + e);
    }
  };

  return (
    <div style={{ position: "relative", width: "100vw", height: "100vh", overflow: "hidden", userSelect: "none" }}>
      <img
        src={imgSrc}
        alt="fullscreen"
        style={{ position: "absolute", top: 0, left: 0, width: "100%", height: "100%", pointerEvents: "none" }}
        onLoad={() => {
          const canvas = canvasRef.current;
          if (canvas) {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
            const ctx = canvas.getContext("2d");
            if (ctx) {
              const img = new Image();
              img.src = imgSrc;
              img.onload = () => {
                ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
                ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
                ctx.fillRect(0, 0, canvas.width, canvas.height);
              };
            }
          }
        }}
      />
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
