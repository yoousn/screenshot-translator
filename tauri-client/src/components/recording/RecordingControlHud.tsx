import { AudioOutlined, CloseOutlined, CopyOutlined, FolderOpenOutlined, PauseOutlined, CaretRightOutlined } from "@ant-design/icons";
import { Button, Tooltip } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type RecordingOverlayStatus = "ready" | "countdown" | "recording" | "paused" | "saving" | "saved";

type RecordingControlHudProps = {
  status: RecordingOverlayStatus;
  elapsedText: string;
  countdown: number | null;
  busy: boolean;
  hasSavedVideo: boolean;
  audioLabel: string;
  onToggleRecord: () => void;
  onPause: () => void;
  onResume: () => void;
  onOpenFolder: () => void;
  onCopy: () => void;
  onCancel: () => void;
};

const dragHandle = (side: "left" | "right") => (
  <button
    className="ysn-rec-drag-handle"
    type="button"
    aria-label={`${side} drag handle`}
    onMouseDown={(event) => {
      event.preventDefault();
      getCurrentWindow().startDragging().catch(() => {});
    }}
    style={{
      width: 22,
      height: 34,
      border: 0,
      padding: 0,
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
      <span key={index} style={{ width: 4, height: 4, borderRadius: 999, background: "#94a3b8" }} />
    ))}
  </button>
);

export default function RecordingControlHud({
  status,
  elapsedText,
  countdown,
  busy,
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
  const recordTitle = status === "saved" ? "录制已保存" : isRecording || isPaused ? "停止并保存" : "开始录制";
  const countdownText = countdown !== null ? String(countdown).padStart(2, "0") : "00";
  const toolBorderColor = isRecording ? "rgba(239,68,68,0.34)" : isPaused ? "rgba(245,158,11,0.36)" : "rgba(226,232,240,0.95)";
  const toolShadow = "none";

  return (
    <div style={{ width: "100vw", height: "100vh", display: "flex", alignItems: "center", justifyContent: "center", pointerEvents: "none", background: "transparent", padding: 2, boxSizing: "border-box" }}>
      <style>{`
        @keyframes ysn-rec-pulse { 0%, 100% { transform: scale(1); opacity: 1; } 50% { transform: scale(1.22); opacity: .68; } }
        .ysn-rec-tool button { pointer-events: auto; }
        .ysn-rec-tool button:not(:disabled):hover { transform: translateY(-1px); }
        .ysn-rec-drag-handle:hover { background: rgba(148, 163, 184, 0.14) !important; }
        .ysn-rec-drag-handle:active { cursor: grabbing; transform: scale(0.96); }
      `}</style>
      <div className="ysn-rec-tool" style={{ pointerEvents: "auto", height: 48, maxWidth: "calc(100vw - 8px)", display: "inline-flex", alignItems: "center", gap: 8, padding: "6px 10px", borderRadius: 999, background: "rgba(255,255,255,0.96)", border: `1px solid ${toolBorderColor}`, boxShadow: toolShadow, color: "#0f172a", boxSizing: "border-box", backdropFilter: "blur(14px)", WebkitBackdropFilter: "blur(14px)", transition: "border-color 180ms ease, box-shadow 180ms ease" }}>
        {dragHandle("left")}
        <Tooltip title={recordTitle}>
          <Button data-no-drag="true" type="text" loading={isSaving || busy && status === "countdown"} disabled={isSaving || busy && !isRecording && !isPaused} onClick={onToggleRecord} style={{ width: 34, height: 34, minWidth: 34, borderRadius: 999, padding: 0, display: "inline-flex", alignItems: "center", justifyContent: "center", background: `${recordColor}14`, border: `1px solid ${recordColor}33` }}>
            <span style={{ width: isRecording || isPaused ? 14 : 15, height: isRecording || isPaused ? 14 : 15, borderRadius: isRecording || isPaused ? 5 : 999, background: recordColor, boxShadow: `0 0 0 5px ${recordColor}18`, animation: isRecording ? "ysn-rec-pulse 1.15s infinite" : "none" }} />
          </Button>
        </Tooltip>
        <Tooltip title={isPaused ? "继续" : "暂停"}>
          <Button data-no-drag="true" type="text" icon={isPaused ? <CaretRightOutlined /> : <PauseOutlined />} disabled={busy || isSaving || isReady || status === "countdown"} onClick={isPaused ? onResume : onPause} style={{ width: 34, height: 34, minWidth: 34, borderRadius: 999, color: isPaused ? "#2563eb" : "#334155", background: isPaused ? "#dbeafe" : "#f8fafc" }} />
        </Tooltip>
        <span style={{ minWidth: 76, fontFamily: "Consolas, Monaco, monospace", fontSize: 14, fontWeight: 850, textAlign: "center", color: isPaused ? "#b45309" : isRecording ? "#dc2626" : "#0f172a" }}>
          {status === "countdown" && countdown !== null ? `00:00:${countdownText}` : elapsedText}
        </span>
        <Tooltip title={audioLabel}>
          <span style={{ height: 30, display: "inline-flex", alignItems: "center", gap: 6, padding: "0 10px", borderRadius: 999, background: "#f8fafc", color: "#334155", fontSize: 12, fontWeight: 750, whiteSpace: "nowrap" }}>
            <AudioOutlined />
            {audioLabel}
          </span>
        </Tooltip>
        <span style={{ width: 1, height: 24, background: "#e2e8f0" }} />
        <Tooltip title="打开视频目录">
          <Button data-no-drag="true" type="text" icon={<FolderOpenOutlined />} onClick={onOpenFolder} style={{ width: 34, height: 34, minWidth: 34, borderRadius: 999, color: "#2563eb", background: "#eff6ff" }} />
        </Tooltip>
        <Tooltip title="关闭 / 取消">
          <Button data-no-drag="true" type="text" icon={<CloseOutlined />} onClick={onCancel} style={{ width: 34, height: 34, minWidth: 34, borderRadius: 999, color: "#dc2626", background: "#fef2f2" }} />
        </Tooltip>
        <Tooltip title={hasSavedVideo ? "复制视频" : "保存后可复制"}>
          <Button data-no-drag="true" type="text" icon={<CopyOutlined />} disabled={!hasSavedVideo || busy || isSaving} onClick={onCopy} style={{ width: 34, height: 34, minWidth: 34, borderRadius: 999, color: hasSavedVideo ? "#0f766e" : "#94a3b8", background: hasSavedVideo ? "#ccfbf1" : "#f1f5f9" }} />
        </Tooltip>
        {dragHandle("right")}
      </div>
    </div>
  );
}
