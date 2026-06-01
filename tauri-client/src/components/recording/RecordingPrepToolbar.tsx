import { Button, Select, Space } from "antd";

type RecordingMode = "region" | "window" | "display";

type RecordingTarget = {
  id: string;
  title: string;
  x: number;
  y: number;
  w: number;
  h: number;
};

type RecordingPrepToolbarProps = {
  mode: RecordingMode;
  windowTargets: RecordingTarget[];
  displayTargets: RecordingTarget[];
  selectedWindowTargetId: string | null;
  selectedDisplayTargetId: string | null;
  fps: number;
  resolution: string;
  audioMode: string;
  countdownSeconds: number;
  audioOptions: Array<{ label: string; value: string; disabled?: boolean }>;
  busy: boolean;
  onSelectWindowTarget: (targetId: string) => void;
  onSelectDisplayTarget: (targetId: string) => void;
  onFpsChange: (fps: number) => void;
  onResolutionChange: (resolution: string) => void;
  onAudioModeChange: (audioMode: string) => void;
  onCountdownChange: (seconds: number) => void;
  onStart: () => void;
  onCancel: () => void;
};

const modeLabel: Record<RecordingMode, string> = {
  region: "\u533a\u57df\u5f55\u5236",
  window: "\u7a97\u53e3\u5f55\u5236",
  display: "\u663e\u793a\u5668\u5f55\u5236",
};

export default function RecordingPrepToolbar({
  mode,
  windowTargets,
  displayTargets,
  selectedWindowTargetId,
  selectedDisplayTargetId,
  fps,
  resolution,
  audioMode,
  countdownSeconds,
  audioOptions,
  busy,
  onSelectWindowTarget,
  onSelectDisplayTarget,
  onFpsChange,
  onResolutionChange,
  onAudioModeChange,
  onCountdownChange,
  onStart,
  onCancel,
}: RecordingPrepToolbarProps) {
  return (
    <Space size={[8, 8]} wrap style={{ maxWidth: "100%", padding: "8px 10px", borderRadius: 16, background: "rgba(255,255,255,0.96)", border: "1px solid rgba(226,232,240,0.95)", boxShadow: "0 12px 32px rgba(15,23,42,0.18)", color: "#111827", backdropFilter: "blur(12px)", boxSizing: "border-box" }}>
      <span style={{ display: "inline-flex", alignItems: "center", gap: 7, height: 30, padding: "0 10px", borderRadius: 999, background: "#fff1f2", color: "#e11d48", fontWeight: 800, letterSpacing: 0.2 }}>
        <span style={{ width: 8, height: 8, borderRadius: 999, background: "#ef4444", boxShadow: "0 0 0 4px rgba(239,68,68,0.14)" }} />
        {modeLabel[mode]}
      </span>
      {mode === "window" && (
        <Select size="small" value={selectedWindowTargetId || undefined} disabled={busy} style={{ width: 240, maxWidth: "calc(100vw - 48px)" }} placeholder="\u9009\u62e9\u7a97\u53e3" onChange={onSelectWindowTarget} options={windowTargets.map((item) => ({ label: item.title, value: item.id }))} />
      )}
      {mode === "display" && (
        <Select size="small" value={selectedDisplayTargetId || undefined} disabled={busy} style={{ width: 190, maxWidth: "calc(100vw - 48px)" }} placeholder="\u9009\u62e9\u663e\u793a\u5668" onChange={onSelectDisplayTarget} options={displayTargets.map((item) => ({ label: item.title, value: item.id }))} />
      )}
      <Select size="small" value={fps} disabled={busy} style={{ width: 92 }} onChange={onFpsChange} options={[{ label: "30 FPS", value: 30 }, { label: "60 FPS", value: 60 }]} />
      <Select size="small" value={resolution} disabled={busy} style={{ width: 96 }} onChange={onResolutionChange} options={[{ label: "480P", value: "480p" }, { label: "720P", value: "720p" }, { label: "1080P", value: "1080p" }, { label: "\u539f\u753b", value: "original" }]} />
      <Select size="small" value={audioMode} disabled={busy} style={{ width: 220, maxWidth: "calc(100vw - 48px)" }} onChange={onAudioModeChange} options={audioOptions} />
      <Select size="small" value={countdownSeconds} disabled={busy} style={{ width: 86 }} onChange={onCountdownChange} options={[{ label: "0s", value: 0 }, { label: "1s", value: 1 }, { label: "3s", value: 3 }, { label: "5s", value: 5 }]} />
      <Button size="small" type="primary" danger loading={busy} onClick={onStart} style={{ height: 30, borderRadius: 999, fontWeight: 700, boxShadow: "0 8px 18px rgba(239,68,68,0.24)" }}>
        {"\u5f00\u59cb"}
      </Button>
      <Button size="small" danger disabled={busy} onClick={onCancel} style={{ height: 30, borderRadius: 999, fontWeight: 700, background: "rgba(255,255,255,0.96)" }}>
        {"\u53d6\u6d88"}
      </Button>
    </Space>
  );
}
