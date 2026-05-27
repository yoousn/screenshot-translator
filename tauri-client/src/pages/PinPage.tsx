import React, { useEffect, useRef, useState } from "react";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

export default function PinPage() {
  const [imgSrc, setImgSrc] = useState<string>("");
  const [scale, setScale] = useState(1);
  const imgRef = useRef<HTMLImageElement | null>(null);
  const winRef = useRef(getCurrentWindow());

  useEffect(() => {
    // Make background transparent
    document.body.style.setProperty("background-color", "transparent", "important");
    document.documentElement.style.setProperty("background-color", "transparent", "important");

    const handleImageData = (base64: string) => {
      const dataUrl = "data:image/png;base64," + base64;
      setImgSrc(dataUrl);

      // Auto-size window to image dimensions in physical pixels
      const img = new Image();
      img.onload = async () => {
        try {
          imgRef.current = img;
          const factor = await winRef.current.scaleFactor();
          const screenW = window.screen.width * factor;
          const screenH = window.screen.height * factor;
          const maxW = Math.min(img.naturalWidth, screenW * 0.8);
          const maxH = Math.min(img.naturalHeight, screenH * 0.8);
          const ratio = Math.min(maxW / img.naturalWidth, maxH / img.naturalHeight, 1);
          setScale(ratio);
          
          await winRef.current.setSize(
            new PhysicalSize(
              Math.round(img.naturalWidth * ratio),
              Math.round(img.naturalHeight * ratio)
            )
          );
        } catch (e) {
          console.error("Failed to size pin window:", e);
        }
      };
      img.src = dataUrl;
    };

    // 1. Instantly pull image data from Rust using the window label (defeats races!)
    const loadCachedData = async () => {
      try {
        const base64 = await invoke<string>("get_pin_image", { label: winRef.current.label });
        handleImageData(base64);
      } catch (err) {
        console.warn("Direct cache pull failed, waiting for event fallback:", err);
      }
    };
    loadCachedData();

    // 2. Listen for pin image data from Rust as a concurrent fallback
    let unlistenData: (() => void) | null = null;
    const setupListener = async () => {
      try {
        const unsub = await listen<string>("pin-image-data", (event) => {
          handleImageData(event.payload);
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
      if ((e.ctrlKey || e.metaKey) && (e.key === "c" || e.key === "C")) {
        e.preventDefault();
        handleCopy();
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
      // Clean up cache to prevent memory leak
      invoke("delete_pin_image", { label: winRef.current.label }).catch(() => {});
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
          alt="钉图"
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
          等待钉图数据...
        </div>
      )}
    </div>
  );
}
