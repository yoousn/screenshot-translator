import React, { useEffect, useState, useRef } from "react";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import { emit, listen } from "@tauri-apps/api/event";

export default function PinPage() {
  const [imgSrc, setImgSrc] = useState<string>("");
  const [opacity, setOpacity] = useState<number>(1);
  const opacityRef = useRef(1);

  useEffect(() => {
    const label = getCurrentWindow().label;
    
    // Listen for image via event (handles large images)
    let unlistenFn: (() => void) | null = null;
    listen<string>(`pin-image-${label}`, (event) => {
      setImgSrc(`data:image/png;base64,${event.payload}`);
    }).then(unsub => { unlistenFn = unsub; });
    emit(`pin-ready-${label}`).catch(() => {});

    // Allow dragging the window
    const handleMouseDown = async (e: MouseEvent) => {
      if (e.button === 0) {
        // Left click to drag
        await getCurrentWindow().startDragging();
      }
    };
    
    // Close on double click or Escape
    const handleDoubleClick = () => getCurrentWindow().close();
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") getCurrentWindow().close();
    };
    const handleContextMenu = (e: MouseEvent) => {
        e.preventDefault();
        getCurrentWindow().close();
    };

    const handleWheel = async (e: WheelEvent) => {
      e.preventDefault();
      if (e.altKey || e.shiftKey) {
        // 调节透明度
        const delta = e.deltaY > 0 ? -0.1 : 0.1;
        opacityRef.current = Math.max(0.1, Math.min(1.0, opacityRef.current + delta));
        setOpacity(opacityRef.current);
      } else {
        // 调节尺寸缩放
        const delta = e.deltaY > 0 ? 0.9 : 1.1; // 向下滚动缩小10%，向上滚动放大10%
        const win = getCurrentWindow();
        try {
          const currentSize = await win.outerSize();
          const newW = Math.max(50, Math.round(currentSize.width * delta));
          const newH = Math.max(50, Math.round(currentSize.height * delta));
          await win.setSize(new PhysicalSize(newW, newH));
        } catch (err) {
          console.error("Failed to resize window", err);
        }
      }
    };

    window.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("dblclick", handleDoubleClick);
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("contextmenu", handleContextMenu);
    window.addEventListener("wheel", handleWheel, { passive: false });

    return () => {
      if (unlistenFn) unlistenFn();
      window.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("dblclick", handleDoubleClick);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("contextmenu", handleContextMenu);
      window.removeEventListener("wheel", handleWheel);
    };
  }, []);

  return (
    <div style={{ width: "100vw", height: "100vh", overflow: "hidden", display: "flex", justifyContent: "center", alignItems: "center", opacity: opacity }}>
      {imgSrc && <img src={imgSrc} alt="Pinned" style={{ width: "100%", height: "100%", objectFit: "contain", cursor: "move", userSelect: "none" }} draggable={false} />}
      
      {/* 操作提示小浮层，仅在调整透明度时或鼠标悬浮时隐约可见，这里为了极简暂不额外实现复杂的 UI */}
      {opacity < 1 && (
        <div style={{
          position: "absolute", top: 4, right: 4, background: "rgba(0,0,0,0.5)", color: "white", padding: "2px 6px", borderRadius: 4, fontSize: 10, pointerEvents: "none"
        }}>
          {Math.round(opacity * 100)}%
        </div>
      )}
    </div>
  );
}
