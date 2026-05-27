import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";

const MIN_SCALE = 0.25;
const MAX_SCALE = 4;
const SCALE_STEP = 0.1;

export default function PinPage() {
  const [src, setSrc] = useState("");
  const [hovered, setHovered] = useState(false);
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const imageBase64Ref = useRef("");
  const naturalSizeRef = useRef({ width: 1, height: 1 });
  const scaleRef = useRef(1);
  const winRef = useRef(getCurrentWindow());

  const resizeWindow = async (scale: number) => {
    const width = Math.max(1, Math.round(naturalSizeRef.current.width * scale));
    const height = Math.max(1, Math.round(naturalSizeRef.current.height * scale));
    await winRef.current.setSize(new PhysicalSize(width, height));
  };

  const setImage = (base64: string) => {
    imageBase64Ref.current = base64;
    const dataUrl = `data:image/png;base64,${base64}`;
    const image = new Image();

    image.onload = async () => {
      naturalSizeRef.current = {
        width: image.naturalWidth,
        height: image.naturalHeight,
      };
      scaleRef.current = 1;
      setSrc(dataUrl);
      await resizeWindow(1);
      await winRef.current.show();
      await winRef.current.setAlwaysOnTop(true);
      await winRef.current.setFocus();
    };

    image.src = dataUrl;
  };

  const scaleBy = (delta: number) => {
    const nextScale = Math.max(MIN_SCALE, Math.min(MAX_SCALE, scaleRef.current + delta));
    if (Math.abs(nextScale - scaleRef.current) < 0.001) return;
    scaleRef.current = nextScale;
    resizeWindow(nextScale).catch(() => {});
  };

  const copyImage = () => {
    if (!imageBase64Ref.current) return;
    invoke("copy_image_to_clipboard", { imageBase64: imageBase64Ref.current }).catch(() => {});
  };

  const closeWindow = () => {
    winRef.current.close().catch(() => {});
  };

  useEffect(() => {
    const win = winRef.current;

    document.body.style.margin = "0";
    document.body.style.overflow = "hidden";
    document.body.style.background = "transparent";
    document.documentElement.style.background = "transparent";

    invoke<string>("get_pin_image", { label: win.label }).then(setImage).catch(() => {});

    let unlisten: (() => void) | null = null;
    listen<string>("pin-image-data", (event) => setImage(event.payload))
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});

    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      scaleBy(event.deltaY < 0 ? SCALE_STEP : -SCALE_STEP);
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeWindow();
        return;
      }

      if (event.key === "+" || event.key === "=") {
        event.preventDefault();
        scaleBy(SCALE_STEP);
        return;
      }

      if (event.key === "-") {
        event.preventDefault();
        scaleBy(-SCALE_STEP);
        return;
      }

      if ((event.ctrlKey || event.metaKey) && event.key === "0") {
        event.preventDefault();
        scaleRef.current = 1;
        resizeWindow(1).catch(() => {});
        return;
      }

      if ((event.ctrlKey || event.metaKey) && (event.key === "c" || event.key === "C")) {
        event.preventDefault();
        copyImage();
      }
    };

    const onDoubleClick = () => closeWindow();

    window.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("dblclick", onDoubleClick);

    return () => {
      window.removeEventListener("wheel", onWheel);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("dblclick", onDoubleClick);
      if (unlisten) unlisten();
      invoke("delete_pin_image", { label: win.label }).catch(() => {});
    };
  }, []);

  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        background: "transparent",
        overflow: "hidden",
        cursor: "move",
        boxSizing: "border-box",
        border: hovered ? "1px solid rgba(0, 0, 0, 0.25)" : "1px solid transparent",
      }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => {
        setHovered(false);
        setMenu(null);
      }}
      onMouseDown={(event) => {
        if (event.button === 0) {
          setMenu(null);
          winRef.current.startDragging().catch(() => {});
        }
      }}
      onContextMenu={(event) => {
        event.preventDefault();
        setHovered(true);
        setMenu({ x: event.clientX, y: event.clientY });
      }}
    >
      {src && (
        <img
          src={src}
          alt="pin"
          draggable={false}
          style={{
            width: "100%",
            height: "100%",
            objectFit: "fill",
            display: "block",
            userSelect: "none",
            pointerEvents: "none",
          }}
        />
      )}

      {hovered && (
        <div
          style={{
            position: "fixed",
            top: 6,
            right: 6,
            display: "flex",
            gap: 6,
            zIndex: 10,
            pointerEvents: "auto",
          }}
        >
          <button type="button" title="Copy" onMouseDown={(event) => event.stopPropagation()} onClick={copyImage} style={{ width: 24, height: 24, borderRadius: "50%", border: "1px solid rgba(0,0,0,0.16)", background: "rgba(255,255,255,0.9)", cursor: "pointer", lineHeight: "20px", padding: 0 }}>⧉</button>
          <button type="button" title="Close" onMouseDown={(event) => event.stopPropagation()} onClick={closeWindow} style={{ width: 24, height: 24, borderRadius: "50%", border: "1px solid rgba(0,0,0,0.16)", background: "rgba(255,255,255,0.9)", cursor: "pointer", lineHeight: "20px", padding: 0 }}>×</button>
        </div>
      )}

      {menu && (
        <div
          style={{
            position: "fixed",
            left: menu.x,
            top: menu.y,
            minWidth: 108,
            background: "#fff",
            border: "1px solid rgba(0,0,0,0.12)",
            boxShadow: "0 4px 12px rgba(0,0,0,0.16)",
            borderRadius: 6,
            overflow: "hidden",
            zIndex: 20,
            fontSize: 13,
          }}
          onMouseDown={(event) => event.stopPropagation()}
        >
          <div style={{ padding: "7px 12px", cursor: "pointer", whiteSpace: "nowrap" }} onClick={() => { copyImage(); setMenu(null); }}>Copy</div>
          <div style={{ padding: "7px 12px", cursor: "pointer", whiteSpace: "nowrap" }} onClick={closeWindow}>Close</div>
        </div>
      )}
    </div>
  );
}
