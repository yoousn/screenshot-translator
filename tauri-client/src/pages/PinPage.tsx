import React, { useEffect, useState, useRef } from "react";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import { emit, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export default function PinPage() {
  const [imgSrc, setImgSrc] = useState<string>("");
  const [opacity, setOpacity] = useState<number>(1);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const imgBase64Ref = useRef("");
  const opacityRef = useRef(1);

  useEffect(() => {
    const label = getCurrentWindow().label;
    
    // Listen for image via event (handles large images)
    let unlistenFn: (() => void) | null = null;
    listen<string>(`pin-image-${label}`, (event) => {
      imgBase64Ref.current = event.payload;
      setImgSrc(`data:image/png;base64,${event.payload}`);
    }).then(unsub => { unlistenFn = unsub; });
    emit(`pin-ready-${label}`).catch(() => {});

    // Allow dragging the window
    const handleMouseDown = async (e: MouseEvent) => {
      if (e.button === 0) {
        // Left click to drag
        setContextMenu(null);
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
      setContextMenu({ x: Math.min(e.clientX, window.innerWidth - 112), y: Math.min(e.clientY, window.innerHeight - 76) });
    };
    const handleClick = () => setContextMenu(null);

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
    window.addEventListener("click", handleClick);

    return () => {
      if (unlistenFn) unlistenFn();
      window.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("dblclick", handleDoubleClick);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("contextmenu", handleContextMenu);
      window.removeEventListener("wheel", handleWheel);
      window.removeEventListener("click", handleClick);
    };
  }, []);

  const copyPinnedImage = async () => {
    if (!imgBase64Ref.current) return;
    await invoke("copy_image_to_clipboard", { imageBase64: imgBase64Ref.current }).catch(() => {});
    setContextMenu(null);
  };

  return (
    <div style={{ width: "100vw", height: "100vh", overflow: "hidden", display: "flex", justifyContent: "center", alignItems: "center", opacity: opacity }}>
      {imgSrc && <img src={imgSrc} alt="Pinned" style={{ width: "100%", height: "100%", objectFit: "contain", cursor: "move", userSelect: "none" }} draggable={false} />}
      
      {/* 操作提示小浮层，仅在调整透明度时或鼠标悬浮时隐约可见，这里为了极简暂不额外实现复杂的 UI */}
      {contextMenu && (
        <div style={{ position: "absolute", left: Math.max(8, contextMenu.x), top: Math.max(8, contextMenu.y), zIndex: 20, background: "#fff", border: "1px solid #ddd", borderRadius: 8, boxShadow: "0 8px 24px rgba(0,0,0,0.18)", padding: 4, minWidth: 96 }} onMouseDown={(e) => e.stopPropagation()}>
          <button style={{ width: "100%", padding: "6px 10px", border: 0, background: "transparent", textAlign: "left", cursor: "pointer" }} onClick={copyPinnedImage}>复制</button>
          <button style={{ width: "100%", padding: "6px 10px", border: 0, background: "transparent", textAlign: "left", cursor: "pointer", color: "#cf1322" }} onClick={() => getCurrentWindow().close()}>关闭</button>
        </div>
      )}

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
