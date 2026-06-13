import React from "react";
import { Button, Space } from "antd";
import { CloseOutlined, CopyOutlined } from "@ant-design/icons";
import type { Rect, TranslatePair } from "../../types/screenshot";

interface TranslatePanelProps {
  rect: Rect;
  pairs: TranslatePair[];
  onClose: () => void;
  diagnostics?: any;
}

const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(value, max));

export default function TranslatePanel({ rect, pairs, onClose, diagnostics }: TranslatePanelProps) {
  const panelWidth = Math.min(350, Math.max(260, window.innerWidth - 16));
  const panelGap = 12;
  const rightLeft = rect.x + rect.w + panelGap;
  const leftLeft = rect.x - panelWidth - panelGap;
  const hasRightSpace = rightLeft + panelWidth <= window.innerWidth - 8;
  const hasLeftSpace = leftLeft >= 8;
  const left = hasRightSpace ? rightLeft : hasLeftSpace ? leftLeft : clamp(rightLeft, 8, Math.max(8, window.innerWidth - panelWidth - 8));
  const pipelineDiagnostics = diagnostics?.pipelineDiagnostics;
  const rawOcr = pipelineDiagnostics?.rawOcr;
  const normalizedOcr = pipelineDiagnostics?.normalizedOcr;
  const translationDecision = pipelineDiagnostics?.translationDecision;
  const translationService = pipelineDiagnostics?.translationService;
  const translationResult = pipelineDiagnostics?.translationResult;

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

      {diagnostics && (
        <div style={{ marginTop: 12, borderTop: "1px solid #f0f0f0", paddingTop: 8, fontSize: 11, color: "#8c8c8c" }}>
          <details style={{ cursor: "pointer" }}>
            <summary style={{ outline: "none", color: "#595959", userSelect: "none" }}>
              性能诊断 (耗时 {diagnostics.totalMs}ms)
            </summary>
            <div style={{ marginTop: 6, display: "flex", flexDirection: "column", gap: 3, cursor: "default", fontFamily: "Consolas, monospace" }} onMouseDown={(e) => e.stopPropagation()}>
              <div>总耗时: {diagnostics.totalMs}ms</div>
              <div>截图加载: {diagnostics.captureMs}ms</div>
              {diagnostics.textSource?.usable ? (
                <>
                  <div style={{ color: "#389e0d" }}>文本源 (UIA): 命中 ({diagnostics.textSource.elapsedMs}ms)</div>
                  <div>UIA 匹配: {diagnostics.textSource.matchedRawCount}/{diagnostics.textSource.rawCount}</div>
                </>
              ) : (
                <>
                  <div style={{ color: "#d46b08" }}>文本源 (UIA): 未命中 ({diagnostics.textSource?.status || "empty"})</div>
                  {diagnostics.localTimings?.ocrMs > 0 && (
                    <div style={{ color: "#096dd9" }}>OCR 识别: {diagnostics.localTimings.ocrMs}ms ({diagnostics.localTimings.source})</div>
                  )}
                </>
              )}
              {diagnostics.localTimings?.translationMs > 0 && (
                <div>网络翻译: {diagnostics.localTimings.translationMs}ms ({diagnostics.usedChannel})</div>
              )}
              {diagnostics.serverTimings?.provider_ms > 0 && (
                <div style={{ paddingLeft: 8 }}>- 服务端: {diagnostics.serverTimings.provider_ms}ms (RTT)</div>
              )}
              {diagnostics.localTimings?.renderMs > 0 && (
                <div>渲染回填: {diagnostics.localTimings.renderMs}ms</div>
              )}
              {pipelineDiagnostics && (
                <div style={{ marginTop: 6, paddingTop: 6, borderTop: "1px solid #f0f0f0", display: "flex", flexDirection: "column", gap: 3 }}>
                  <div style={{ color: diagnostics.status === "error" ? "#cf1322" : "#595959" }}>
                    阶段: {pipelineDiagnostics.stage}{pipelineDiagnostics.error ? ` | ${pipelineDiagnostics.error}` : ""}
                  </div>
                  <div>OCR: raw {rawOcr?.count ?? 0}, normalized {normalizedOcr?.count ?? 0}, avg {normalizedOcr?.avgConfidence ?? rawOcr?.avgConfidence ?? "-"}</div>
                  <div>翻译判定: 需译 {translationDecision?.requiresTranslation ?? 0}, 保留 {translationDecision?.preservedByPolicy ?? 0}, 请求 {translationDecision?.queuedForService ?? 0}</div>
                  <div>服务返回: {translationResult?.returnedTranslations ?? 0}, 空 {translationResult?.emptyTranslations ?? 0}, 原文 {translationResult?.unchangedTranslations ?? 0}</div>
                  <div>服务请求: {translationService?.requestedBlocks ?? 0} 行, 去重后 {translationService?.dedupedBlocks ?? 0} 行</div>
                </div>
              )}
            </div>
          </details>
        </div>
      )}
    </div>
  );
}
