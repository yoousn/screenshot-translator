import type { MouseEvent } from "react";
import { AudioOutlined, CaretRightOutlined, CloseOutlined, CopyOutlined, FolderOpenOutlined, PauseOutlined } from "@ant-design/icons";
import { Button, Tooltip } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type RecordingOverlayStatus = "ready" | "countdown" | "recording" | "paused" | "saving" | "saved";

type RecordingControlHudProps = {
  status: RecordingOverlayStatus;
  elapsedText: string;
  countdown: number | null;
  busy: boolean;
  sessionReady: boolean;
  hasSavedVideo: boolean;
  audioLabel: string;
  onToggleRecord: () => void;
  onPause: () => void;
  onResume: () => void;
  onOpenFolder: () => void;
  onCopy: () => void;
  onCancel: () => void;
};

const tooltipProps = {
  placement: "top" as const,
  mouseEnterDelay: 0.55,
  mouseLeaveDelay: 0.04,
  overlayStyle: { pointerEvents: "none" as const, fontSize: 12 },
  getPopupContainer: () => document.body,
};

const dragHandle = (side: "left" | "right") => {
  const startDrag = (event: MouseEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    getCurrentWindow().setFocus().catch(() => {});
    getCurrentWindow().startDragging().catch(() => {});
  };

  return (
    <div
      className="ysn-rec-drag-handle ysn-rec-drag-region"
      data-tauri-drag-region
      role="button"
      tabIndex={0}
      aria-label={`${side} drag handle`}
      onMouseDown={startDrag}
      style={{
        width: 26,
        height: 38,
        display: "grid",
        gridTemplateColumns: "repeat(2, 4px)",
        gridTemplateRows: "repeat(3, 4px)",
        alignContent: "center",
        justifyContent: "center",
        gap: 3,
        borderRadius: 10,
        background: "transparent",
        cursor: "grab",
        transition: "background 140ms ease, transform 140ms ease",
      }}
    >
      {Array.from({ length: 6 }).map((_, index) => (
        <span key={index} data-tauri-drag-region style={{ width: 4, height: 4, borderRadius: 999, background: "#94a3b8" }} />
      ))}
    </div>
  );
};

export default function RecordingControlHud({
  status,
  elapsedText,
  countdown,
  busy,
  sessionReady,
  hasSavedVideo,
  audioLabel,
  onToggleRecord,
  onPause,
  onResume,
  onOpenFolder,
  onCopy,
  onCancel,
}: RecordingControlHudProps) {
  const isReady = status === "ready" || status === "saved";
  const isRecording = status === "recording" || status === "countdown";
  const isPaused = status === "paused";
  const isSaving = status === "saving";
  const recordColor = isRecording ? "#ef4444" : isPaused ? "#f59e0b" : "#2563eb";
  const recordTitle = !sessionReady ? "录制准备中" : status === "saved" ? "录制已保存" : isRecording || isPaused ? "停止并保存" : "开始录制";
  const countdownText = countdown !== null ? String(countdown).padStart(2, "0") : "00";
  const toolBorderColor = isRecording ? "rgba(239,68,68,0.34)" : isPaused ? "rgba(245,158,11,0.36)" : "rgba(226,232,240,0.95)";
  const canUseControls = sessionReady && !isSaving;

  return (
    <div className="ysn-rec-shell" style={{ width: "100vw", height: "100vh", display: "flex", alignItems: "center", justifyContent: "center", background: "rgba(0,0,0,0.005)", padding: 8, boxSizing: "border-box" }}>
      <style>{`
        @keyframes ysn-rec-pulse { 0%, 100% { transform: scale(1); opacity: 1; } 50% { transform: scale(1.22); opacity: .68; } }
        .ysn-rec-tool button, .ysn-rec-drag-handle { pointer-events: auto; }
        .ysn-rec-tool button:not(:disabled):hover { transform: translateY(-1px); }
        .ysn-rec-drag-region, .ysn-rec-drag-region * { -webkit-app-region: drag; app-region: drag; }
        .ysn-rec-action, .ysn-rec-action * { -webkit-app-region: no-drag; app-region: no-drag; }
        .ysn-rec-drag-handle:hover { background: rgba(148, 163, 184, 0.14) !important; }
        .ysn-rec-drag-handle:active { cursor: grabbing; transform: scale(0.96); }
      `}</style>
      <div className="ysn-rec-tool" style={{ pointerEvents: "auto", height: 52, maxWidth: "calc(100vw - 12px)", display: "inline-flex", alignItems: "center", gap: 8, padding: "7px 12px", borderRadius: 999, background: "rgba(255,255,255,0.97)", border: `1px solid ${toolBorderColor}`, boxShadow: "none", color: "#0f172a", boxSizing: "border-box", backdropFilter: "blur(16px)", WebkitBackdropFilter: "blur(16px)", transition: "border-color 180ms ease" }}>
        {dragHandle("left")}
        <Tooltip {...tooltipProps} title={recordTitle}>
          <Button className="ysn-rec-action" data-no-drag="true" type="text" loading={isSaving || (busy && status === "countdown")} disabled={!canUseControls || (busy && !isRecording && !isPaused)} onClick={onToggleRecord} style={{ width: 36, height: 36, minWidth: 36, borderRadius: 999, padding: 0, display: "inline-flex", alignItems: "center", justifyContent: "center", background: `${recordColor}14`, border: `1px solid ${recordColor}33` }}>
            <span style={{ width: isRecording || isPaused ? 14 : 15, height: isRecording || isPaused ? 14 : 15, borderRadius: isRecording || isPaused ? 5 : 999, background: recordColor, boxShadow: `0 0 0 5px ${recordColor}18`, animation: isRecording ? "ysn-rec-pulse 1.15s infinite" : "none" }} />
          </Button>
        </Tooltip>
        <Tooltip {...tooltipProps} title={isPaused ? "继续" : "暂停"}>
          <Button className="ysn-rec-action" data-no-drag="true" type="text" icon={isPaused ? <CaretRightOutlined /> : <PauseOutlined />} disabled={!canUseControls || busy || isReady || status === "countdown"} onClick={isPaused ? onResume : onPause} style={{ width: 36, height: 36, minWidth: 36, borderRadius: 999, color: isPaused ? "#2563eb" : "#334155", background: isPaused ? "#dbeafe" : "#f8fafc" }} />
        </Tooltip>
        <span style={{ minWidth: 82, fontFamily: "Consolas, Monaco, monospace", fontSize: 14, fontWeight: 850, textAlign: "center", color: isPaused ? "#b45309" : isRecording ? "#dc2626" : "#0f172a" }}>
          {status === "countdown" && countdown !== null ? `00:00:${countdownText}` : elapsedText}
        </span>
        <Tooltip {...tooltipProps} title={audioLabel}>
          <span className="ysn-rec-action" style={{ height: 30, display: "inline-flex", alignItems: "center", gap: 6, padding: "0 10px", borderRadius: 999, background: "#f8fafc", color: "#334155", fontSize: 12, fontWeight: 750, whiteSpace: "nowrap" }}>
            <AudioOutlined />
            {audioLabel}
          </span>
        </Tooltip>
        <span style={{ width: 1, height: 24, background: "#e2e8f0" }} />
        <Tooltip {...tooltipProps} title="打开视频目录">
          <Button className="ysn-rec-action" data-no-drag="true" type="text" icon={<FolderOpenOutlined />} onClick={onOpenFolder} style={{ width: 36, height: 36, minWidth: 36, borderRadius: 999, color: "#2563eb", background: "#eff6ff" }} />
        </Tooltip>
        <Tooltip {...tooltipProps} title="关闭 / 取消">
          <button className="ysn-rec-action" data-no-drag="true" type="button" onMouseDown={(e) => { e.stopPropagation(); e.preventDefault(); }} onClick={(e) => { e.stopPropagation(); e.preventDefault(); console.log('[window-trace] close-button click fired'); onCancel(); }} style={{ width: 36, height: 36, minWidth: 36, borderRadius: 999, color: "#dc2626", background: "#fef2f2", border: "none", cursor: "pointer", display: "inline-flex", alignItems: "center", justifyContent: "center", padding: 0 }}>
            <CloseOutlined />
          </button>
        </Tooltip>
        <Tooltip {...tooltipProps} title={hasSavedVideo ? "复制视频" : "保存后可复制"}>
          <Button className="ysn-rec-action" data-no-drag="true" type="text" icon={<CopyOutlined />} disabled={!hasSavedVideo || busy || isSaving} onClick={onCopy} style={{ width: 36, height: 36, minWidth: 36, borderRadius: 999, color: hasSavedVideo ? "#0f766e" : "#94a3b8", background: hasSavedVideo ? "#ccfbf1" : "#f1f5f9" }} />
        </Tooltip>
        {dragHandle("right")}
      </div>
    </div>
  );
}
