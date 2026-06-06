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
  diagnostics?: any;
}

export default function OcrPage() {
  const { text: dictionary } = useI18n();
  const labels = dictionary.ocrResult;
  const winRef = useRef(getCurrentWindow());
  const loadingText = "\u6b63\u5728\u52a0\u8f7d OCR \u7ed3\u679c...";
  const [text, setText] = useState(loadingText);
  const textRef = useRef(loadingText);
  const [previewBase64, setPreviewBase64] = useState("");
  const [title, setTitle] = useState(labels.defaultTitle);
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);
  const [contextMenu, setContextMenu] = useState<OcrResultContextMenu>(null);
  const [normalizationSummary, setNormalizationSummary] = useState<OcrResultNormalizationSummary | null>(null);
  const [diagnostics, setDiagnostics] = useState<any>(null);

  useEffect(() => {
    setTitle((current) => current || labels.defaultTitle);
  }, [labels.defaultTitle]);

  useEffect(() => {
    const win = winRef.current;
    const label = win.label;
    let unlistenFn: (() => void) | null = null;

    const payloadKey = `ysn-ocr-result-${label}`;
    const applyPayload = (payload: OcrWindowPayload) => {
      const nextText = payload.text || "\u672a\u8bc6\u522b\u5230\u6587\u5b57\u3002\u8bf7\u91cd\u65b0\u6846\u9009\u66f4\u6e05\u6670\u7684\u533a\u57df\uff0c\u6216\u5230\u8bc6\u5b57\u6a21\u578b\u9875\u5237\u65b0\u6a21\u578b\u72b6\u6001\u3002";
      textRef.current = nextText;
      setText(nextText);
      setPreviewBase64(payload.previewBase64 || "");
      setTitle(payload.title || labels.defaultTitle);
      setNormalizationSummary(payload.normalizationSummary || null);
      setDiagnostics(payload.diagnostics || null);
    };

    const applyPayloadJson = (payloadJson: string | null) => {
      if (!payloadJson) return false;
      try {
        applyPayload(JSON.parse(payloadJson) as OcrWindowPayload);
        return true;
      } catch (error) {
        console.error("Failed to parse OCR payload", error);
        return false;
      }
    };

    applyPayloadJson(window.localStorage.getItem(payloadKey));

    listen<string>(`ocr-result-${label}`, (event) => {
      if (applyPayloadJson(event.payload)) {
        window.localStorage.removeItem(payloadKey);
      }
    }).then((unsub) => {
      unlistenFn = unsub;
      emit(`ocr-ready-${label}`).catch(() => {});
    });

    const fallbackTimer = window.setTimeout(() => {
      if (textRef.current === loadingText) {
        setText("OCR \u7ed3\u679c\u7a97\u53e3\u5df2\u6253\u5f00\uff0c\u4f46\u7ed3\u679c\u8fd8\u6ca1\u9001\u8fbe\u3002\u8bf7\u91cd\u8bd5 Ctrl+D\uff1b\u5982\u679c\u7ee7\u7eed\u5931\u8d25\uff0c\u53bb\u8bc6\u5b57\u6a21\u578b\u9875\u70b9\u5237\u65b0\u3002");
      }
    }, 3000);

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
      window.clearTimeout(fallbackTimer);
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
      diagnostics={diagnostics}
    />
  );
}
