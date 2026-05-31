import React from "react";
import { Button, Space } from "antd";
import { CloseOutlined, CopyOutlined } from "@ant-design/icons";
import type { Rect, TranslatePair } from "../../types/screenshot";

interface TranslatePanelProps {
  rect: Rect;
  pairs: TranslatePair[];
  onClose: () => void;
}

const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(value, max));

export default function TranslatePanel({ rect, pairs, onClose }: TranslatePanelProps) {
  const panelWidth = 350;
  const panelGap = 12;
  const rightLeft = rect.x + rect.w + panelGap;
  const leftLeft = rect.x - panelWidth - panelGap;
  const hasRightSpace = rightLeft + panelWidth <= window.innerWidth - 8;
  const hasLeftSpace = leftLeft >= 8;
  const left = hasRightSpace ? rightLeft : hasLeftSpace ? leftLeft : clamp(rightLeft, 8, window.innerWidth - panelWidth - 8);

  return (
    <div
      style={{ position: "absolute", top: Math.max(8, Math.min(rect.y, window.innerHeight - 360)), left, width: panelWidth, maxHeight: "80vh", overflowY: "auto", zIndex: 120, background: "#fff", padding: 12, borderRadius: 10, boxShadow: "0 6px 24px rgba(0, 0, 0, 0.18)", border: "1px solid #e8e8e8" }}
      onMouseDown={(event) => event.stopPropagation()}
      onContextMenu={(event) => event.stopPropagation()}
    >
      <div style={{ marginBottom: 12, fontWeight: "bold", fontSize: 14 }}>翻译结果</div>
      <div style={{ display: "flex", flexDirection: "column", gap: 10, marginBottom: 12 }}>
        {pairs.map((pair, index) => (
          <div key={index} style={{ padding: 8, background: "#f5f5f5", borderRadius: 6, fontSize: 12 }}>
            <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 8, marginBottom: 6 }}>
              <div style={{ color: "#8c8c8c", lineHeight: 1.45, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>{pair.o}</div>
              <Button size="small" onClick={() => navigator.clipboard.writeText(pair.o)}>复制原文</Button>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr auto", gap: 8 }}>
              <div style={{ color: "#1f1f1f", fontWeight: "bold", lineHeight: 1.45, whiteSpace: "pre-wrap", wordBreak: "break-word" }}>{pair.t}</div>
              <Button size="small" type="primary" ghost onClick={() => navigator.clipboard.writeText(pair.t)}>复制译文</Button>
            </div>
          </div>
        ))}
      </div>
      <Space size="small">
        <Button size="small" type="primary" icon={<CopyOutlined />} onClick={() => navigator.clipboard.writeText(pairs.map((pair) => pair.t).join("\n"))}>复制全部译文</Button>
        <Button size="small" icon={<CloseOutlined />} onClick={onClose}>关闭</Button>
      </Space>
    </div>
  );
}
