import { CheckOutlined, CloseOutlined, PauseOutlined, CaretRightOutlined } from "@ant-design/icons";
import { Button, Space } from "antd";

export type RecordingOverlayStatus = "countdown" | "recording" | "paused" | "saving";

type RecordingControlHudProps = {
  status: RecordingOverlayStatus;
  elapsedText: string;
  countdown: number | null;
  busy: boolean;
  onPause: () => void;
  onResume: () => void;
  onSave: () => void;
  onCancel: () => void;
};

export default function RecordingControlHud({
  status,
  elapsedText,
  countdown,
  busy,
  onPause,
  onResume,
  onSave,
  onCancel,
}: RecordingControlHudProps) {
  const isPaused = status === "paused";
  const isCounting = status === "countdown";

  return (
    <div style={{ width: "100vw", height: "100vh", display: "flex", alignItems: "flex-start", justifyContent: "flex-start", pointerEvents: "auto", background: "transparent", padding: 2, boxSizing: "border-box" }}>
      <style>{`
        @keyframes ysn-rec-pulse { 0%, 100% { transform: scale(1); opacity: 1; } 50% { transform: scale(1.32); opacity: .62; } }
      `}</style>
      <div style={{ display: "inline-flex", alignItems: "center", maxWidth: "calc(100vw - 4px)", height: 44, padding: "4px 8px", borderRadius: 999, background: "rgba(255,255,255,0.96)", border: "1px solid rgba(226,232,240,0.95)", boxShadow: "0 10px 28px rgba(15,23,42,0.18)", color: "#111827", boxSizing: "border-box" }}>
        <Space size={7} align="center" wrap={false}>
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6, height: 28, padding: "0 10px", borderRadius: 999, background: isPaused ? "#fffbeb" : "#fff1f2", color: isPaused ? "#d97706" : "#e11d48", fontSize: 12, fontWeight: 850, letterSpacing: 0.2 }}>
            <span style={{ width: 7, height: 7, borderRadius: 999, background: isPaused ? "#f59e0b" : "#ef3348", boxShadow: "0 0 0 3px rgba(239,51,72,0.14)", animation: isPaused ? "none" : "ysn-rec-pulse 1.2s infinite" }} />
            {isCounting && countdown !== null ? countdown : isPaused ? "已暂停" : "录制中"}
          </span>
          <span style={{ minWidth: 48, fontFamily: "Consolas, Monaco, monospace", fontSize: 14, fontWeight: 850 }}>{elapsedText}</span>
          {status !== "saving" && (
            <Button data-no-drag="true" size="small" type="default" icon={isPaused ? <CaretRightOutlined /> : <PauseOutlined />} disabled={busy || isCounting} onClick={isPaused ? onResume : onPause} style={{ height: 30, borderRadius: 999, fontWeight: 750 }}>
              {isPaused ? "\u7ee7\u7eed" : "\u6682\u505c"}
            </Button>
          )}
          <Button data-no-drag="true" size="small" type="primary" icon={<CheckOutlined />} loading={busy && status === "saving"} disabled={isCounting} onClick={onSave} style={{ height: 30, borderRadius: 999, fontWeight: 750, boxShadow: "0 8px 18px rgba(37,99,235,0.24)" }}>
            {"\u4fdd\u5b58"}
          </Button>
          <Button data-no-drag="true" size="small" danger icon={<CloseOutlined />} disabled={busy && status === "saving"} onClick={onCancel} style={{ height: 30, borderRadius: 999, background: "rgba(255,255,255,0.96)", fontWeight: 750 }}>
            {"\u53d6\u6d88"}
          </Button>
        </Space>
      </div>
    </div>
  );
}
