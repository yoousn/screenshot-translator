import { Button, Empty, Space, Tooltip } from "antd";
import { AppstoreOutlined, CheckOutlined, CloseOutlined, DesktopOutlined } from "@ant-design/icons";

export type RecordingPickerMode = "window" | "display";

export type RecordingPickerTarget = {
  id: string;
  title: string;
  exeName?: string;
  processPath?: string;
  iconDataUrl?: string | null;
  x: number;
  y: number;
  w: number;
  h: number;
};

type RecordingTargetPickerProps = {
  mode: RecordingPickerMode;
  targets: RecordingPickerTarget[];
  selectedTargetId: string | null;
  busy?: boolean;
  onSelect: (targetId: string) => void;
  onConfirm: () => void;
  onCancel: () => void;
};

const titleByMode: Record<RecordingPickerMode, string> = {
  window: "选择窗口录制",
  display: "选择显示器录制",
};

const subtitleByMode: Record<RecordingPickerMode, string> = {
  window: "点击窗口名称预览蓝框，确认后进入录制控制条。",
  display: "点击显示器预览蓝框，确认后进入录制控制条。",
};

function TargetIcon({ target, mode }: { target: RecordingPickerTarget; mode: RecordingPickerMode }) {
  if (target.iconDataUrl) {
    return <img src={target.iconDataUrl} alt="" style={{ width: 22, height: 22, borderRadius: 6, objectFit: "cover" }} />;
  }
  const fallback = (target.exeName || target.title || "?").trim().slice(0, 1).toUpperCase();
  if (mode === "display") return <DesktopOutlined style={{ fontSize: 18, color: "#1677ff" }} />;
  return (
    <span style={{ width: 22, height: 22, borderRadius: 7, display: "inline-flex", alignItems: "center", justifyContent: "center", background: "#eef4ff", color: "#1677ff", fontSize: 12, fontWeight: 800 }}>
      {fallback || <AppstoreOutlined />}
    </span>
  );
}

export default function RecordingTargetPicker({
  mode,
  targets,
  selectedTargetId,
  busy = false,
  onSelect,
  onConfirm,
  onCancel,
}: RecordingTargetPickerProps) {
  return (
    <div style={{ maxWidth: "min(760px, calc(100vw - 24px))", padding: 10, borderRadius: 18, border: "1px solid rgba(226,232,240,0.95)", background: "rgba(255,255,255,0.97)", boxShadow: "0 16px 42px rgba(15,23,42,0.16)", backdropFilter: "blur(14px)", color: "#0f172a" }}>
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", marginBottom: 8 }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontWeight: 800, fontSize: 13 }}>{titleByMode[mode]}</div>
          <div style={{ color: "#64748b", fontSize: 12, marginTop: 2 }}>{subtitleByMode[mode]}</div>
        </div>
        <Space size={6}>
          <Tooltip title="取消 Esc">
            <Button size="small" icon={<CloseOutlined />} onClick={onCancel} disabled={busy} />
          </Tooltip>
          <Tooltip title="确认录制目标">
            <Button size="small" type="primary" icon={<CheckOutlined />} onClick={onConfirm} disabled={!selectedTargetId || busy} loading={busy}>
              确认
            </Button>
          </Tooltip>
        </Space>
      </div>
      {targets.length === 0 ? (
        <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={mode === "window" ? "没有检测到可录制窗口" : "没有检测到显示器"} />
      ) : (
        <div style={{ display: "flex", gap: 8, overflowX: "auto", paddingBottom: 2, maxWidth: "100%" }}>
          {targets.map((target) => {
            const active = selectedTargetId === target.id;
            return (
              <button
                key={target.id}
                type="button"
                onClick={() => onSelect(target.id)}
                style={{
                  minWidth: mode === "display" ? 180 : 230,
                  maxWidth: 280,
                  height: 54,
                  padding: "7px 9px",
                  borderRadius: 12,
                  border: active ? "1px solid #1677ff" : "1px solid #e2e8f0",
                  background: active ? "#eff6ff" : "#fff",
                  color: "#0f172a",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  textAlign: "left",
                  boxShadow: active ? "0 0 0 3px rgba(22,119,255,0.12)" : "none",
                }}
              >
                <TargetIcon target={target} mode={mode} />
                <span style={{ minWidth: 0, display: "block" }}>
                  <span style={{ display: "block", fontSize: 12, fontWeight: 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{target.title}</span>
                  <span style={{ display: "block", fontSize: 11, color: "#64748b", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {mode === "display" ? `${target.w} x ${target.h}` : target.exeName || target.processPath || "Window"}
                  </span>
                </span>
                <span style={{ marginLeft: "auto", width: 8, height: 8, borderRadius: 999, background: active ? "#1677ff" : "transparent" }} />
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
