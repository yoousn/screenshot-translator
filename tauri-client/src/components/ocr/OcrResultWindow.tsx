import React from "react";
import { Button, Input, Tooltip } from "antd";
import { CloseOutlined, CopyOutlined, PushpinOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";

export type OcrResultContextMenu = { x: number; y: number } | null;

type OcrResultNormalizationSummary = {
  rawCount: number;
  usefulCount: number;
  virtualLineCount: number;
  droppedCount: number;
  routeMissingScripts?: string[];
};

interface OcrResultWindowProps {
  title: string;
  text: string;
  previewBase64: string;
  alwaysOnTop: boolean;
  contextMenu: OcrResultContextMenu;
  onTextChange: (text: string) => void;
  onMouseDown: (event: React.MouseEvent<HTMLElement>) => void;
  onToggleAlwaysOnTop: () => void;
  onClose: () => void;
  onCopyAndClose: () => void;
  normalizationSummary?: OcrResultNormalizationSummary | null;
}

export default function OcrResultWindow({
  title,
  text,
  previewBase64,
  alwaysOnTop,
  contextMenu,
  onTextChange,
  onMouseDown,
  onToggleAlwaysOnTop,
  onClose,
  onCopyAndClose,
  normalizationSummary,
}: OcrResultWindowProps) {
  const { text: dictionary } = useI18n();
  const labels = dictionary.ocrResult;

  return (
    <div
      onMouseDown={onMouseDown}
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
        <div style={{ fontSize: 13, fontWeight: 600, color: "#1f2937" }}>{title}</div>
        <div data-no-drag="true" style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
          <Tooltip title={alwaysOnTop ? labels.unpinWindow : labels.pinWindow}>
            <Button size="small" type={alwaysOnTop ? "primary" : "text"} icon={<PushpinOutlined />} onClick={onToggleAlwaysOnTop} />
          </Tooltip>
          <Tooltip title={labels.close}>
            <Button size="small" type="text" danger icon={<CloseOutlined />} onClick={onClose} />
          </Tooltip>
        </div>
      </div>

      {contextMenu && (
        <div data-no-drag="true" style={{ position: "absolute", left: Math.max(8, contextMenu.x), top: Math.max(8, contextMenu.y), zIndex: 20, background: "#fff", border: "1px solid #ddd", borderRadius: 8, boxShadow: "0 8px 24px rgba(0,0,0,0.18)", padding: 4, minWidth: 116 }} onMouseDown={(event) => event.stopPropagation()}>
          <button style={{ width: "100%", padding: "6px 10px", border: 0, background: "transparent", textAlign: "left", cursor: text ? "pointer" : "not-allowed", opacity: text ? 1 : 0.45 }} onClick={onCopyAndClose} disabled={!text}>{labels.copyAndClose}</button>
          <button style={{ width: "100%", padding: "6px 10px", border: 0, background: "transparent", textAlign: "left", cursor: "pointer", color: "#cf1322" }} onClick={onClose}>{labels.close}</button>
        </div>
      )}

      <div style={{ padding: 12, display: "flex", flexDirection: "column", gap: 10, minHeight: 0, flex: 1 }}>
        <div data-no-drag="true" style={{ minHeight: 0, flex: 1, cursor: "auto" }}>
    
      {normalizationSummary && (
        <div
          data-no-drag="true"
          style={{
            display: "flex",
            gap: 8,
            flexWrap: "wrap",
            padding: "6px 10px",
            borderBottom: "1px solid #eef2f7",
            background: "#fbfdff",
            color: "#64748b",
            fontSize: 12,
          }}
        >
          <span>{labels.rawBlocks || "Raw"} {normalizationSummary.rawCount}</span>
          <span>{labels.usefulBlocks || "Useful"} {normalizationSummary.usefulCount}</span>
          <span>{labels.virtualLines || "Lines"} {normalizationSummary.virtualLineCount}</span>
          {normalizationSummary.droppedCount > 0 && <span>{labels.droppedBlocks || "Dropped"} {normalizationSummary.droppedCount}</span>}
          {(normalizationSummary.routeMissingScripts?.length || 0) > 0 && <span>{labels.missingScripts || "Missing scripts"} {normalizationSummary.routeMissingScripts?.join(", ")}</span>}
        </div>
      )}
      <Input.TextArea
            value={text}
            onChange={(event) => onTextChange(event.target.value)}
            placeholder={labels.noTextRecognized}
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
            <img src={`data:image/png;base64,${previewBase64}`} alt={labels.previewAlt} draggable={false} style={{ maxWidth: "100%", maxHeight: "100%", objectFit: "contain" }} />
          </div>
        )}

        <div data-no-drag="true" style={{ display: "flex", justifyContent: "flex-end", cursor: "auto" }}>
          <Button size="small" type="primary" icon={<CopyOutlined />} onClick={onCopyAndClose} disabled={!text}>
            {labels.copyAndClose}
          </Button>
        </div>
      </div>
    </div>
  );
}
