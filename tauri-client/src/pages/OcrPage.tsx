import React, { useEffect, useRef, useState } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";
import OcrResultWindow, { type OcrResultContextMenu } from "../components/ocr/OcrResultWindow";
import { useI18n } from "../i18n";

type OcrResultNormalizationSummary = {
  rawCount: number;
  usefulCount: number;
  virtualLineCount: number;
  droppedCount: number;
  routeMissingScripts?: string[];
};

interface OcrWindowPayload {
  text: string;
  previewBase64: string;
  title?: string;
  normalizationSummary?: OcrResultNormalizationSummary;
}

export default function OcrPage() {
  const { text: dictionary } = useI18n();
  const labels = dictionary.ocrResult;
  const winRef = useRef(getCurrentWindow());
  const [text, setText] = useState("");
  const textRef = useRef("");
  const [previewBase64, setPreviewBase64] = useState("");
  const [title, setTitle] = useState(labels.defaultTitle);
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);
  const [contextMenu, setContextMenu] = useState<OcrResultContextMenu>(null);
  const [normalizationSummary, setNormalizationSummary] = useState<OcrResultNormalizationSummary | null>(null);

  useEffect(() => {
    setTitle((current) => current || labels.defaultTitle);
  }, [labels.defaultTitle]);

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
        setTitle(payload.title || labels.defaultTitle);
        setNormalizationSummary(payload.normalizationSummary || null);
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
        closeWindow();
        return;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "c") {
        const target = event.target as HTMLElement | null;
        const hasSelectedText = window.getSelection()?.toString();
        const currentText = textRef.current;
        if (!hasSelectedText && target?.tagName !== "TEXTAREA" && currentText) {
          event.preventDefault();
          copyAndClose();
        }
      }
    };

    const handleContextMenu = (event: MouseEvent) => {
      event.preventDefault();
      setContextMenu({ x: Math.min(event.clientX, window.innerWidth - 128), y: Math.min(event.clientY, window.innerHeight - 76) });
    };
    const handleClick = () => setContextMenu(null);

    window.addEventListener("mouseenter", focusWindow);
    window.addEventListener("mousemove", focusWindow);
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
  }, [labels.defaultTitle]);

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
      message.error(labels.pinToggleFailed);
    }
  };

  const updateText = (nextText: string) => {
    textRef.current = nextText;
    setText(nextText);
  };

  const copyAndClose = async () => {
    try {
      await navigator.clipboard.writeText(textRef.current);
      setContextMenu(null);
      await winRef.current.close();
    } catch (error) {
      message.error(labels.copyFailed);
    }
  };

  return (
    <OcrResultWindow
      title={title}
      text={text}
      previewBase64={previewBase64}
      alwaysOnTop={alwaysOnTop}
      contextMenu={contextMenu}
      onTextChange={updateText}
      onMouseDown={startDragging}
      onToggleAlwaysOnTop={toggleAlwaysOnTop}
      onClose={closeWindow}
      onCopyAndClose={copyAndClose}
      normalizationSummary={normalizationSummary}
    />
  );
}
