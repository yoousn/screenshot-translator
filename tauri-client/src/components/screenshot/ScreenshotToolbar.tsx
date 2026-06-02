import React from "react";
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
  isScrollCapturing?: boolean;
  canUndo: boolean;
  canRedo: boolean;
  onSetEditing: (editing: boolean) => void;
  onSelectMove: () => void;
  onSetAnnotationTool: (tool: AnnotationTool) => void;
  onSetAnnotationColor: (color: string) => void;
  onSetAnnotationSize: (size: number) => void;
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

const tools: ToolbarTool[] = [
  { key: "rect", tip: "矩形标注 1", icon: <BorderOutlined /> },
  { key: "circle", tip: "圆形标注 2", icon: <span style={{ fontSize: 22, lineHeight: 1 }}>○</span> },
  { key: "arrow", tip: "箭头标注 3", icon: <ArrowUpOutlined rotate={45} style={{ fontSize: 18 }} /> },
  { key: "brush", tip: "画笔 4", icon: <EditOutlined style={{ fontSize: 18 }} /> },
  { key: "text", tip: "文字标注 5 / T", icon: <span style={{ fontWeight: 800, fontSize: 19 }}>T</span> },
  { key: "mosaic", tip: "马赛克 6", icon: <span style={{ fontSize: 20, lineHeight: 1 }}>▦</span> },
];

const squareButtonStyle: React.CSSProperties = { width: 36, height: 36, padding: 0, fontSize: 18 };
const iconButtonStyle: React.CSSProperties = { width: 42, height: 36, padding: 0 };

const getToolHint = (annotationTool: AnnotationTool | null) => {
  if (annotationTool === "text") return "点击选区添加文字；点击已有文字可编辑。";
  if (annotationTool === "rect" || annotationTool === "circle") return "矩形和圆形可拖动移动，边缘可调整大小。";
  if (annotationTool === "mosaic") return "拖动需要打码的区域；可用撤销删除。";
  return "拖动绘制标注；可用撤销删除。";
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
  isScrollCapturing = false,
  canUndo,
  canRedo,
  onSetEditing,
  onSelectMove,
  onSetAnnotationTool,
  onSetAnnotationColor,
  onSetAnnotationSize,
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
  const normalizedGap = Math.max(0, Math.min(16, Number(buttonGap) || 0));
  const isBusy = isTranslating || isOCRing || isScrollCapturing;

  return (
    <div ref={containerRef} style={style} onContextMenu={(event) => event.stopPropagation()}>
      <Space size={[normalizedGap, 6]} wrap style={{ display: "inline-flex", maxWidth: "100%", whiteSpace: "normal", alignItems: "center" }}>
        <Tooltip title="移动/调整选区">
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
            items: [{ key: "result", label: "查看翻译结果", disabled: !canShowTranslateResult }],
            onClick: ({ key }) => {
              if (key === "result") onShowTranslateResult();
            },
          }}
        >
          <Button.Group>
            <Tooltip title="截图翻译并重绘 Ctrl+Q">
              <Button size="middle" style={iconButtonStyle} type="primary" ghost onClick={(event) => { event.stopPropagation(); onTranslate(); }} loading={isTranslating} disabled={isOCRing} icon={<span style={{ fontSize: 13, fontWeight: 800 }}>A/文</span>} />
            </Tooltip>
            <Button size="middle" style={{ width: 24, height: 36, padding: 0 }} disabled={isTranslating || isOCRing} icon={<DownOutlined style={{ fontSize: 10 }} />} />
          </Button.Group>
        </Dropdown>

        <Tooltip title="OCR 识字 Ctrl+D">
          <Button size="middle" style={squareButtonStyle} icon={<ScanOutlined />} onClick={onOCR} loading={isOCRing} disabled={isTranslating || isScrollCapturing} />
        </Tooltip>
        {onScrollCapture && (
          <Tooltip title="滚动截图">
            <Button size="middle" style={squareButtonStyle} icon={<span style={{ fontSize: 13, fontWeight: 800 }}>SCR</span>} onClick={onScrollCapture} loading={isScrollCapturing} disabled={isTranslating || isOCRing} />
          </Tooltip>
        )}
        {onRecording && (
          <Dropdown
            trigger={["click"]}
            menu={{
              items: [
                { key: "region", label: "区域录制" },
                { key: "window", label: "窗口录制" },
                { key: "display", label: "显示器录制" },
              ],
              onClick: ({ key }) => onRecording(key as "region" | "window" | "display"),
            }}
          >
            <Button.Group>
              <Tooltip title="录制选区">
                <Button size="middle" style={iconButtonStyle} icon={<VideoCameraOutlined />} disabled={isBusy} />
              </Tooltip>
              <Button size="middle" style={{ width: 24, height: 36, padding: 0 }} disabled={isBusy} icon={<DownOutlined style={{ fontSize: 10 }} />} />
            </Button.Group>
          </Dropdown>
        )}
        <Tooltip title="钉图"><Button size="middle" style={squareButtonStyle} icon={<PushpinOutlined />} onClick={onPin} /></Tooltip>
        <Tooltip title="撤销 Ctrl+Z"><Button size="middle" style={squareButtonStyle} disabled={!canUndo} icon={<UndoOutlined />} onClick={onUndo} /></Tooltip>
        <Tooltip title="恢复 Ctrl+Y / Ctrl+Shift+Z"><Button size="middle" style={squareButtonStyle} disabled={!canRedo} icon={<RedoOutlined />} onClick={onRedo} /></Tooltip>
        <Tooltip title="保存 Ctrl+S"><Button size="middle" style={squareButtonStyle} icon={<SaveOutlined />} onClick={onSave} /></Tooltip>
        <Tooltip title="取消 Esc"><Button size="middle" style={{ ...iconButtonStyle, color: "#ef4444", borderColor: "transparent", background: "transparent", fontSize: 20, borderRadius: 10, boxShadow: "none" }} icon={<CloseOutlined />} onClick={onCancel} /></Tooltip>
        <Tooltip title="完成并复制 Ctrl+C"><Button size="middle" style={{ ...iconButtonStyle, color: "#16a34a", borderColor: "transparent", background: "transparent", fontSize: 20, borderRadius: 10, boxShadow: "none" }} icon={<CheckOutlined />} onClick={onCopy} /></Tooltip>
      </Space>
      {isEditing && annotationTool && (
        <div style={{ marginTop: 8, display: "flex", alignItems: "center", flexWrap: "wrap", gap: 8, color: "#334155", fontSize: 12, background: "#f8fafc", border: "1px solid #e2e8f0", borderRadius: 10, padding: "6px 8px" }}>
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
