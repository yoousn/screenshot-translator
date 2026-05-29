import React, { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export default function PinPage() {
  const [imgSrc, setImgSrc] = useState<string>("");

  useEffect(() => {
    const label = getCurrentWindow().label;
    const data = localStorage.getItem(label);
    if (data) {
      // it should be base64
      setImgSrc(`data:image/png;base64,${data}`);
    }

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
    }

    window.addEventListener("mousedown", handleMouseDown);
    window.addEventListener("dblclick", handleDoubleClick);
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("contextmenu", handleContextMenu);

    return () => {
      window.removeEventListener("mousedown", handleMouseDown);
      window.removeEventListener("dblclick", handleDoubleClick);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("contextmenu", handleContextMenu);
    };
  }, []);

  return (
    <div style={{ width: "100vw", height: "100vh", overflow: "hidden", display: "flex", justifyContent: "center", alignItems: "center" }}>
      {imgSrc && <img src={imgSrc} alt="Pinned" style={{ width: "100%", height: "100%", objectFit: "contain", cursor: "move" }} draggable={false} />}
    </div>
  );
}
