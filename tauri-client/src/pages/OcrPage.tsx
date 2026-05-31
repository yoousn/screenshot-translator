import React, { useEffect, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Button, Input, Tooltip, message } from "antd";
import { CloseOutlined, CopyOutlined, PushpinOutlined } from "@ant-design/icons";

interface OcrWindowPayload {
  text: string;
  previewBase64: string;
  title?: string;
}

export default function OcrPage() {
  const winRef = useRef(getCurrentWindow());
  const [text, setText] = useState("");
  const textRef = useRef("");
  const [previewBase64, setPreviewBase64] = useState("");
  const [title, setTitle] = useState("OCR 识字结果");
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  useEffect(() => {
    const win = winRef.current;
    const label = win.label;
    let unlistenFn: (() => void) | null = null;

    listen<string>(`ocr-result-${label}`, (event) => {
      try {
        const payload = JSON.parse(event.payload) as OcrWindowPayload;
        const nextText = payload.text || "";
        textRef.current = nextText;
        setText(nextText);
        setPreviewBase64(payload.previewBase64 || "");
        setTitle(payload.title || "OCR 识字结果");
      } catch (error) {
        console.error("Failed to parse OCR payload", error);
      }
    }).then((unsub) => {
      unlistenFn = unsub;
      emit(`ocr-ready-${label}`).catch(() => {});
    });

    const focusWindow = () => {
      win.setFocus().catch(() => {});
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        win.close().catch(() => {});
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c") {
        const target = event.target as HTMLElement | null;
        const hasSelectedText = window.getSelection()?.toString();
        const currentText = textRef.current;
        if (!hasSelectedText && target?.tagName !== "TEXTAREA" && currentText) {
          event.preventDefault();
          navigator.clipboard.writeText(currentText).catch(() => {});
        }
      }
    };

    window.addEventListener("mouseenter", focusWindow);
    window.addEventListener("mousemove", focusWindow);
    const handleContextMenu = (event: MouseEvent) => {
      event.preventDefault();
      setContextMenu({ x: event.clientX, y: event.clientY });
    };
    const handleClick = () => setContextMenu(null);

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("contextmenu", handleContextMenu);
    window.addEventListener("click", handleClick);

    return () => {
      if (unlistenFn) unlistenFn();
      window.removeEventListener("mouseenter", focusWindow);
      window.removeEventListener("mousemove", focusWindow);
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("contextmenu", handleContextMenu);
      window.removeEventListener("click", handleClick);
    };
  }, []);

  const startDragging = async (event: React.MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement;
    if (target.closest("[data-no-drag='true']")) return;
    await winRef.current.startDragging();
  };

  const closeWindow = () => {
    winRef.current.close().catch(() => {});
  };

  const toggleAlwaysOnTop = async () => {
    const next = !alwaysOnTop;
    setAlwaysOnTop(next);
    try {
      await winRef.current.setAlwaysOnTop(next);
    } catch (error) {
      setAlwaysOnTop(!next);
      message.error("置顶切换失败");
    }
  };

  const copyText = async () => {
    try {
      await navigator.clipboard.writeText(textRef.current);
      message.success("OCR 文本已复制");
      setContextMenu(null);
    } catch (error) {
      message.error("复制失败");
    }
  };

  return (
    <div
      onMouseDown={startDragging}
      style={{
        width: "100vw",
        height: "100vh",
        display: "flex",
        flexDirection: "column",
        background: "#ffffff",
        border: "1px solid #e5e7eb",
        boxSizing: "border-box",
        overflow: "hidden",
        userSelect: "auto",
      }}
    >
      <div
        style={{
          height: 40,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "0 8px 0 12px",
          borderBottom: "1px solid #f0f0f0",
          background: "#f8fafc",
          cursor: "move",
          flex: "0 0 auto",
          userSelect: "none",
        }}
      >
        <div style={{ fontSize: 13, fontWeight: 600, color: "#1f2937" }}>OCR 识字结果</div>
        <div data-no-drag="true" style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          <Tooltip title={alwaysOnTop ? "取消置顶" : "置顶窗口"}>
            <Button
              size="small"
              type={alwaysOnTop ? "primary" : "text"}
              icon={<PushpinOutlined />}
              onClick={toggleAlwaysOnTop}
            />
          </Tooltip>
          <Tooltip title="关闭">
            <Button size="small" type="text" danger icon={<CloseOutlined />} onClick={closeWindow} />
          </Tooltip>
        </div>
      </div>

      {contextMenu && (
        <div data-no-drag="true" style={{ position: "absolute", left: contextMenu.x, top: contextMenu.y, zIndex: 20, background: "#fff", border: "1px solid #ddd", borderRadius: 8, boxShadow: "0 8px 24px rgba(0,0,0,0.18)", padding: 4, minWidth: 96 }} onMouseDown={(event) => event.stopPropagation()}>
          <button style={{ width: "100%", padding: "6px 10px", border: 0, background: "transparent", textAlign: "left", cursor: text ? "pointer" : "not-allowed", opacity: text ? 1 : 0.45 }} onClick={copyText} disabled={!text}>复制</button>
          <button style={{ width: "100%", padding: "6px 10px", border: 0, background: "transparent", textAlign: "left", cursor: "pointer", color: "#cf1322" }} onClick={closeWindow}>关闭</button>
        </div>
      )}

      <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, minHeight: 0, flex: 1 }}>
        <div data-no-drag="true" style={{ minHeight: 0, flex: 1, cursor: "auto" }}>
          <Input.TextArea
            value={text}
            onChange={(event) => { textRef.current = event.target.value; setText(event.target.value); }}
            placeholder="未识别到文字"
            style={{ height: "100%", resize: "none", fontSize: 13, lineHeight: 1.55 }}
          />
        </div>

        {previewBase64 && (
          <div
            data-no-drag="true"
            style={{
              height: 96,
              border: "1px solid #f0f0f0",
              borderRadius: 6,
              overflow: "hidden",
              background: "#fafafa",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              flex: "0 0 auto",
              cursor: "auto",
            }}
          >
            <img
              src={`data:image/png;base64,${previewBase64}`}
              alt="OCR preview"
              draggable={false}
              style={{ maxWidth: "100%", maxHeight: "100%", objectFit: "contain" }}
            />
          </div>
        )}

        <div data-no-drag="true" style={{ display: "flex", justifyContent: "flex-end", cursor: "auto" }}>
          <Button size="small" type="primary" icon={<CopyOutlined />} onClick={copyText} disabled={!text}>
            复制文本
          </Button>
        </div>
      </div>
    </div>
  );
}
