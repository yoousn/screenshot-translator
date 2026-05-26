import React, { useEffect, useRef, useState } from "react";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";

export default function PinPage() {
  const [imgSrc, setImgSrc] = useState<string>("");
  const [scale, setScale] = useState(1);
  const imgRef = useRef<HTMLImageElement | null>(null);
  const winRef = useRef(getCurrentWindow());

  useEffect(() => {
    // Make background transparent
    document.body.style.setProperty("background-color", "transparent", "important");
    document.documentElement.style.setProperty("background-color", "transparent", "important");

    // Listen for pin image data from Rust
    let unlistenData: (() => void) | null = null;
    const setupListener = async () => {
      try {
        const unsub = await listen<string>("pin-image-data", (event) => {
          const base64 = event.payload;
          const dataUrl = "data:image/png;base64," + base64;
          setImgSrc(dataUrl);

          // Auto-size window to image dimensions
          const img = new Image();
          img.onload = () => {
            imgRef.current = img;
            const maxW = Math.min(img.naturalWidth, window.screen.width * 0.8);
            const maxH = Math.min(img.naturalHeight, window.screen.height * 0.8);
            const ratio = Math.min(maxW / img.naturalWidth, maxH / img.naturalHeight, 1);
            setScale(ratio);
            winRef.current.setSize(
              new LogicalSize(
                Math.round(img.naturalWidth * ratio),
                Math.round(img.naturalHeight * ratio)
              )
            );
          };
          img.src = dataUrl;
        });
        unlistenData = unsub;
      } catch (err) {
        console.error("Failed to listen pin-image-data", err);
      }
    };
    setupListener();

    // Keyboard listener: Esc to close, + to zoom in, - to zoom out
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        winRef.current.close();
      }
      if (e.key === "+" || e.key === "=") {
        setScale((s) => Math.min(s + 0.1, 3));
      }
      if (e.key === "-") {
        setScale((s) => Math.max(s - 0.1, 0.3));
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    // Wheel zoom
    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      setScale((s) => {
        const newScale = e.deltaY < 0 ? s + 0.1 : s - 0.1;
        return Math.max(0.3, Math.min(3, newScale));
      });
    };
    window.addEventListener("wheel", handleWheel, { passive: false });

    // Double-click to close
    const handleDblClick = () => {
      winRef.current.close();
    };
    window.addEventListener("dblclick", handleDblClick);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("wheel", handleWheel);
      window.removeEventListener("dblclick", handleDblClick);
      if (unlistenData) unlistenData();
    };
  }, []);

  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        overflow: "hidden",
        cursor: "move",
        background: "transparent",
      }}
      onMouseDown={async () => {
        // Tauri v2: start dragging the window on mouse down
        try {
          await winRef.current.startDragging();
        } catch (e) {
          // Silently ignore if dragging not supported
        }
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        winRef.current.close();
      }}
    >
      {imgSrc ? (
        <img
          src={imgSrc}
          alt="贴图"
          style={{
            maxWidth: "100%",
            maxHeight: "100%",
            transform: `scale(${scale})`,
            transition: "transform 0.15s ease",
            pointerEvents: "none",
            userSelect: "none",
            borderRadius: 4,
            boxShadow: "0 2px 16px rgba(0,0,0,0.15)",
          }}
          draggable={false}
        />
      ) : (
        <div
          style={{
            color: "#fff",
            background: "rgba(0,0,0,0.5)",
            padding: "12px 20px",
            borderRadius: 8,
            fontSize: 13,
          }}
        >
          等待贴图数据...
        </div>
      )}
    </div>
  );
}
