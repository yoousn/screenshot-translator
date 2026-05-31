import React from "react";
import { Button, Dropdown, InputNumber, Space, Tooltip } from "antd";
import {
  ArrowUpOutlined,
  BorderOutlined,
  CheckOutlined,
  CloseOutlined,
  DownOutlined,
  EditOutlined,
  PushpinOutlined,
  RedoOutlined,
  SaveOutlined,
  ScanOutlined,
  UndoOutlined,
} from "@ant-design/icons";
import type { AnnotationTool } from "../../types/screenshot";

interface ScreenshotToolbarProps {
  containerRef: React.RefObject<HTMLDivElement | null>;
  style: React.CSSProperties;
  annotationTool: AnnotationTool | null;
  annotationColor: string;
  annotationSize: number;
  isEditing: boolean;
  isTranslating: boolean;
  isOCRing: boolean;
  canUndo: boolean;
  canRedo: boolean;
  onSetEditing: (editing: boolean) => void;
  onSetAnnotationTool: (tool: AnnotationTool) => void;
  onSetAnnotationColor: (color: string) => void;
  onSetAnnotationSize: (size: number) => void;
  onTranslate: () => void;
  onShowTranslateResult: () => void;
  canShowTranslateResult: boolean;
  onOCR: () => void;
  onPin: () => void;
  onUndo: () => void;
  onRedo: () => void;
  onSave: () => void;
  onCancel: () => void;
  onCopy: () => void;
}

const tools: Array<{ key: AnnotationTool; tip: string; icon: React.ReactNode }> = [
  { key: "rect", tip: "方框", icon: <BorderOutlined /> },
  { key: "circle", tip: "圆形", icon: <span style={{ fontSize: 22, lineHeight: 1 }}>○</span> },
  { key: "arrow", tip: "箭头", icon: <ArrowUpOutlined rotate={45} style={{ fontSize: 18 }} /> },
  { key: "brush", tip: "画笔", icon: <EditOutlined style={{ fontSize: 18 }} /> },
  { key: "mosaic", tip: "马赛克", icon: <span style={{ fontSize: 20, lineHeight: 1 }}>▦</span> },
  { key: "text", tip: "文字", icon: <span style={{ fontWeight: 800, fontSize: 19 }}>T</span> },
];

const squareButtonStyle: React.CSSProperties = { width: 36, height: 36, padding: 0, fontSize: 18 };

const getToolHint = (annotationTool: AnnotationTool | null) => {
  if (annotationTool === "text") return "点击选区添加文字，点击文字可编辑";
  if (annotationTool === "rect" || annotationTool === "circle") return "拖动可移动已画标注";
  return "画错可撤销";
};

export default function ScreenshotToolbar({
  containerRef,
  style,
  annotationTool,
  annotationColor,
  annotationSize,
  isEditing,
  isTranslating,
  isOCRing,
  canUndo,
  canRedo,
  onSetEditing,
  onSetAnnotationTool,
  onSetAnnotationColor,
  onSetAnnotationSize,
  onTranslate,
  onShowTranslateResult,
  canShowTranslateResult,
  onOCR,
  onPin,
  onUndo,
  onRedo,
  onSave,
  onCancel,
  onCopy,
}: ScreenshotToolbarProps) {
  return (
    <div ref={containerRef} style={style} onContextMenu={(event) => event.stopPropagation()}>
      <Space size={6} style={{ display: "inline-flex", flexWrap: "nowrap", whiteSpace: "nowrap", alignItems: "center" }}>
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
            items: [{ key: "result", label: "翻译结果", disabled: !canShowTranslateResult }],
            onClick: ({ key }) => {
              if (key === "result") onShowTranslateResult();
            },
          }}
        >
          <Button.Group>
            <Tooltip title="翻译并重绘">
              <Button size="middle" style={{ width: 42, height: 36, padding: 0 }} type="primary" ghost onClick={(event) => { event.stopPropagation(); onTranslate(); }} loading={isTranslating} disabled={isOCRing} icon={<span style={{ fontSize: 13, fontWeight: 800 }}>A/译</span>} />
            </Tooltip>
            <Button size="middle" style={{ width: 24, height: 36, padding: 0 }} disabled={isTranslating || isOCRing} icon={<DownOutlined style={{ fontSize: 10 }} />} />
          </Button.Group>
        </Dropdown>
        <Tooltip title="OCR 识字"><Button size="middle" style={squareButtonStyle} icon={<ScanOutlined />} onClick={onOCR} loading={isOCRing} disabled={isTranslating} /></Tooltip>
        <Tooltip title="贴图"><Button size="middle" style={squareButtonStyle} icon={<PushpinOutlined />} onClick={onPin} /></Tooltip>
        <Tooltip title="撤销 Ctrl+Z"><Button size="middle" style={squareButtonStyle} disabled={!canUndo} icon={<UndoOutlined />} onClick={onUndo} /></Tooltip>
        <Tooltip title="恢复 Ctrl+Y / Ctrl+Shift+Z"><Button size="middle" style={squareButtonStyle} disabled={!canRedo} icon={<RedoOutlined />} onClick={onRedo} /></Tooltip>
        <Tooltip title="保存"><Button size="middle" style={squareButtonStyle} icon={<SaveOutlined />} onClick={onSave} /></Tooltip>
        <Tooltip title="取消"><Button size="middle" style={{ width: 42, height: 36, padding: 0, color: "#ef4444", borderColor: "transparent", background: "transparent", fontSize: 20, borderRadius: 10, boxShadow: "none" }} icon={<CloseOutlined />} onClick={onCancel} /></Tooltip>
        <Tooltip title="完成并复制"><Button size="middle" style={{ width: 42, height: 36, padding: 0, color: "#16a34a", borderColor: "transparent", background: "transparent", fontSize: 20, borderRadius: 10, boxShadow: "none" }} icon={<CheckOutlined />} onClick={onCopy} /></Tooltip>
      </Space>
      {isEditing && (
        <div style={{ marginTop: 8, display: "flex", alignItems: "center", gap: 8, color: "#ffffff", fontSize: 12, textShadow: "0 1px 2px rgba(0,0,0,0.45)" }}>
          <span>大小</span>
          <InputNumber size="small" min={1} max={48} value={annotationSize} onChange={(value) => onSetAnnotationSize(Number(value || 1))} style={{ width: 74 }} />
          {annotationTool !== "mosaic" && (
            <>
              <span>颜色</span>
              <input type="color" value={annotationColor} onChange={(event) => onSetAnnotationColor(event.target.value)} style={{ width: 30, height: 26, padding: 0, border: "1px solid #d9d9d9", borderRadius: 5, background: "#fff" }} />
            </>
          )}
          <span>{getToolHint(annotationTool)}</span>
        </div>
      )}
    </div>
  );
}
