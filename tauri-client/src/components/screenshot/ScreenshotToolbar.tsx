import React from "react";
import { useI18n } from "../../i18n";
import { Button, Dropdown, InputNumber, Space, Tooltip } from "antd";
import {
  ArrowUpOutlined,
  BorderOutlined,
  CheckOutlined,
  CloseOutlined,
  DownOutlined,
  EditOutlined,
  DragOutlined,
  PushpinOutlined,
  RedoOutlined,
  SaveOutlined,
  ScanOutlined,
  UndoOutlined,
  VideoCameraOutlined,
} from "@ant-design/icons";
import type { AnnotationTool, MarkerShape } from "../../types/screenshot";

interface ScreenshotToolbarProps {
  containerRef: React.RefObject<HTMLDivElement | null>;
  style: React.CSSProperties;
  annotationTool: AnnotationTool | null;
  annotationColor: string;
  annotationSize: number;
  isEditing: boolean;
  isTranslating: boolean;
  isOCRing: boolean;
  isScrollCapturing?: boolean;
  canUndo: boolean;
  canRedo: boolean;
  onSetEditing: (editing: boolean) => void;
  onSelectMove: () => void;
  onSetAnnotationTool: (tool: AnnotationTool) => void;
  onSetAnnotationColor: (color: string) => void;
  onSetAnnotationSize: (size: number) => void;
  markerShape: MarkerShape;
  onSetMarkerShape: (shape: MarkerShape) => void;
  onTranslate: () => void;
  onShowTranslateResult: () => void;
  canShowTranslateResult: boolean;
  onOCR: () => void;
  onScrollCapture?: () => void;
  onRecording?: (mode: "region" | "window" | "display") => void;
  onPin: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onSave: () => void;
  onCancel: () => void;
  onCopy: () => void;
  buttonGap?: number;
}

type ToolbarTool = { key: AnnotationTool; tip: string; icon: React.ReactNode };



const squareButtonStyle: React.CSSProperties = { width: 36, height: 36, padding: 0, fontSize: 18 };
const iconButtonStyle: React.CSSProperties = { width: 42, height: 36, padding: 0 };



export default function ScreenshotToolbar({
  containerRef,
  style,
  annotationTool,
  annotationColor,
  annotationSize,
  isEditing,
  isTranslating,
  isOCRing,
  isScrollCapturing = false,
  canUndo,
  canRedo,
  onSetEditing,
  onSelectMove,
  onSetAnnotationTool,
  onSetAnnotationColor,
  onSetAnnotationSize,
  markerShape,
  onSetMarkerShape,
  onTranslate,
  onShowTranslateResult,
  canShowTranslateResult,
  onOCR,
  onScrollCapture,
  onRecording,
  onPin,
  onUndo,
  onRedo,
  onSave,
  onCancel,
  onCopy,
  buttonGap = 6,
}: ScreenshotToolbarProps) {
  
  const { text } = useI18n();

  const tools: ToolbarTool[] = [
    { key: "rect", tip: text.toolbar.rect, icon: <BorderOutlined /> },
    { key: "circle", tip: text.toolbar.circle, icon: <span style={{ fontSize: 22, lineHeight: 1 }}>○</span> },
    { key: "arrow", tip: text.toolbar.arrow, icon: <ArrowUpOutlined rotate={45} style={{ fontSize: 18 }} /> },
    { key: "brush", tip: text.toolbar.brush, icon: <EditOutlined style={{ fontSize: 18 }} /> },
    { key: "text", tip: text.toolbar.text, icon: <span style={{ fontWeight: 800, fontSize: 19 }}>T</span> },
    { key: "mosaic", tip: text.toolbar.mosaic, icon: <span style={{ fontSize: 20, lineHeight: 1 }}>▦</span> },
    { key: "number", tip: "数字标注", icon: <span style={{ fontSize: 16, fontWeight: 700 }}>①</span> },
  ];

  const getToolHint = (tool: AnnotationTool | null) => {
    if (tool === "text") return text.toolbar.hintText;
    if (tool === "rect" || tool === "circle") return text.toolbar.hintShape;
    if (tool === "mosaic") return text.toolbar.hintMosaic;
    return text.toolbar.hintDefault;
  };

  const normalizedGap = Math.max(0, Math.min(16, Number(buttonGap) || 0));
  const isBusy = isTranslating || isOCRing || isScrollCapturing;

  return (
    <div ref={containerRef} style={style} onContextMenu={(event) => event.stopPropagation()}>
      <Space size={[normalizedGap, 6]} wrap style={{ display: "inline-flex", maxWidth: "100%", whiteSpace: "normal", alignItems: "center" }}>
        <Tooltip title={text.toolbar.move}>
          <Button
            size="middle"
            style={squareButtonStyle}
            type={!isEditing || !annotationTool ? "primary" : "default"}
            icon={<DragOutlined />}
            onClick={() => {
              onSetEditing(false);
              onSelectMove();
            }}
          />
        </Tooltip>
        {tools.map((item) => (
          <Tooltip key={item.key} title={item.tip}>
            <Button
              size="middle"
              style={squareButtonStyle}
              type={annotationTool === item.key ? "primary" : "default"}
              icon={item.icon}
              onClick={() => {
                onSetEditing(true);
                onSetAnnotationTool(item.key);
              }}
            />
          </Tooltip>
        ))}

        <Dropdown
          trigger={["click"]}
          menu={{
            items: [{ key: "result", label: text.toolbar.viewResult, disabled: !canShowTranslateResult }],
            onClick: ({ key }) => {
              if (key === "result") onShowTranslateResult();
            },
          }}
        >
          <Button.Group>
            <Tooltip title={text.toolbar.translate}>
              <Button size="middle" style={iconButtonStyle} type="primary" ghost onClick={(event) => { event.stopPropagation(); onTranslate(); }} loading={isTranslating} disabled={isOCRing} icon={<span style={{ fontSize: 13, fontWeight: 800 }}>A</span>} />
            </Tooltip>
            <Button size="middle" style={{ width: 24, height: 36, padding: 0 }} disabled={isTranslating || isOCRing} icon={<DownOutlined style={{ fontSize: 10 }} />} />
          </Button.Group>
        </Dropdown>

        <Tooltip title={text.toolbar.ocr}>
          <Button size="middle" style={squareButtonStyle} icon={<ScanOutlined />} onClick={onOCR} loading={isOCRing} disabled={isTranslating || isScrollCapturing} />
        </Tooltip>
        {onScrollCapture && (
          <Tooltip title={text.toolbar.scrollCapture}>
            <Button size="middle" style={squareButtonStyle} icon={<span style={{ fontSize: 13, fontWeight: 800 }}>SCR</span>} onClick={onScrollCapture} loading={isScrollCapturing} disabled={isTranslating || isOCRing} />
          </Tooltip>
        )}
        {onRecording && (
          <Dropdown
            trigger={["click"]}
            menu={{
              items: [
                { key: "region", label: text.toolbar.regionRecord },
                { key: "window", label: text.toolbar.windowRecord },
                { key: "display", label: text.toolbar.displayRecord },
              ],
              onClick: ({ key }) => onRecording(key as "region" | "window" | "display"),
            }}
          >
            <Button.Group>
              <Tooltip title={text.toolbar.recordRegion}>
                <Button size="middle" style={iconButtonStyle} icon={<VideoCameraOutlined />} disabled={isBusy} />
              </Tooltip>
              <Button size="middle" style={{ width: 24, height: 36, padding: 0 }} disabled={isBusy} icon={<DownOutlined style={{ fontSize: 10 }} />} />
            </Button.Group>
          </Dropdown>
        )}
        <Tooltip title={text.toolbar.pin}><Button size="middle" style={squareButtonStyle} icon={<PushpinOutlined />} onClick={onPin} /></Tooltip>
        <Tooltip title={text.toolbar.undo}><Button size="middle" style={squareButtonStyle} disabled={!canUndo} icon={<UndoOutlined />} onClick={onUndo} /></Tooltip>
        <Tooltip title={text.toolbar.redo}><Button size="middle" style={squareButtonStyle} disabled={!canRedo} icon={<RedoOutlined />} onClick={onRedo} /></Tooltip>
        <Tooltip title={text.toolbar.save}><Button size="middle" style={squareButtonStyle} icon={<SaveOutlined />} onClick={onSave} /></Tooltip>
        <Tooltip title={text.toolbar.cancel}><Button size="middle" style={{ ...iconButtonStyle, color: "#ef4444", borderColor: "transparent", background: "transparent", fontSize: 20, borderRadius: 10, boxShadow: "none" }} icon={<CloseOutlined />} onClick={onCancel} /></Tooltip>
        <Tooltip title={text.toolbar.copy}><Button size="middle" style={{ ...iconButtonStyle, color: "#16a34a", borderColor: "transparent", background: "transparent", fontSize: 20, borderRadius: 10, boxShadow: "none" }} icon={<CheckOutlined />} onClick={onCopy} /></Tooltip>
      </Space>
      {isEditing && annotationTool && (
        <div style={{ marginTop: 8, display: "flex", alignItems: "center", flexWrap: "wrap", gap: 8, color: "#334155", fontSize: 12, background: "#f8fafc", border: "1px solid #e2e8f0", borderRadius: 10, padding: "6px 8px" }}>
          <span>{text.toolbar.size}</span>
          <InputNumber size="small" min={1} max={48} value={annotationSize} onChange={(value) => onSetAnnotationSize(Number(value || 1))} style={{ width: 74 }} />
          {annotationTool !== "mosaic" && (
            <>
              <span>{text.toolbar.color}</span>
              <input type="color" value={annotationColor} onChange={(event) => onSetAnnotationColor(event.target.value)} style={{ width: 30, height: 26, padding: 0, border: "1px solid #d9d9d9", borderRadius: 5, background: "#fff" }} />
            </>
          )}
          {annotationTool === "number" && (
            <Space size={4} style={{ marginLeft: 4 }}>
              {(["circle", "square", "drop"] as MarkerShape[]).map((shape) => (
                <Button
                  key={shape}
                  size="small"
                  type={markerShape === shape ? "primary" : "default"}
                  style={{ width: 30, height: 24, padding: 0, fontSize: 14 }}
                  onClick={() => onSetMarkerShape(shape)}
                >
                  {shape === "circle" ? "①" : shape === "square" ? "▣" : "◈"}
                </Button>
              ))}
            </Space>
          )}
          <span>{getToolHint(annotationTool)}</span>
        </div>
      )}
    </div>
  );
}
