import { useEffect, useState, type CSSProperties } from "react";
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

const ICON_FRAME_STYLE: CSSProperties = {
  width: 26,
  height: 26,
  borderRadius: 8,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  flex: "0 0 auto",
  background: "linear-gradient(180deg, rgba(248,250,252,0.96), rgba(241,245,249,0.92))",
  border: "1px solid rgba(226,232,240,0.92)",
  boxShadow: "inset 0 1px 0 rgba(255,255,255,0.72), 0 1px 2px rgba(15,23,42,0.08)",
  overflow: "hidden",
};

const ICON_IMAGE_STYLE: CSSProperties = {
  width: 20,
  height: 20,
  display: "block",
  objectFit: "contain",
  filter: "drop-shadow(0 1px 1px rgba(15,23,42,0.18))",
};

function ProcessedTargetIconImage({ src }: { src: string }) {
  const [processedSrc, setProcessedSrc] = useState(src);

  useEffect(() => {
    let cancelled = false;
    const image = new Image();
    image.decoding = "async";

    image.onload = () => {
      if (cancelled) return;

      try {
        const sourceSize = Math.max(image.width, image.height, 1);
        const workCanvas = document.createElement("canvas");
        workCanvas.width = sourceSize;
        workCanvas.height = sourceSize;
        const workCtx = workCanvas.getContext("2d", { willReadFrequently: true });
        if (!workCtx) {
          setProcessedSrc(src);
          return;
        }

        workCtx.clearRect(0, 0, sourceSize, sourceSize);
        workCtx.drawImage(image, 0, 0, sourceSize, sourceSize);

        const imageData = workCtx.getImageData(0, 0, sourceSize, sourceSize);
        const { data } = imageData;
        const visited = new Uint8Array(sourceSize * sourceSize);
        const queueX = new Int32Array(sourceSize * sourceSize);
        const queueY = new Int32Array(sourceSize * sourceSize);
        let head = 0;
        let tail = 0;

        const isEdgeMatte = (x: number, y: number) => {
          const idx = (y * sourceSize + x) * 4;
          const alpha = data[idx + 3];
          if (alpha < 12) return false;
          const r = data[idx];
          const g = data[idx + 1];
          const b = data[idx + 2];
          return r >= 238 && g >= 238 && b >= 238;
        };

        const push = (x: number, y: number) => {
          const pos = y * sourceSize + x;
          if (visited[pos] || !isEdgeMatte(x, y)) return;
          visited[pos] = 1;
          queueX[tail] = x;
          queueY[tail] = y;
          tail++;
        };

        for (let x = 0; x < sourceSize; x++) {
          push(x, 0);
          push(x, sourceSize - 1);
        }
        for (let y = 1; y < sourceSize - 1; y++) {
          push(0, y);
          push(sourceSize - 1, y);
        }

        while (head < tail) {
          const x = queueX[head];
          const y = queueY[head];
          head++;
          const idx = (y * sourceSize + x) * 4;
          data[idx + 3] = 0;
          if (x > 0) push(x - 1, y);
          if (x + 1 < sourceSize) push(x + 1, y);
          if (y > 0) push(x, y - 1);
          if (y + 1 < sourceSize) push(x, y + 1);
        }

        workCtx.putImageData(imageData, 0, 0);

        let minX = sourceSize;
        let minY = sourceSize;
        let maxX = -1;
        let maxY = -1;
        for (let y = 0; y < sourceSize; y++) {
          for (let x = 0; x < sourceSize; x++) {
            const idx = (y * sourceSize + x) * 4;
            if (data[idx + 3] < 20) continue;
            minX = Math.min(minX, x);
            minY = Math.min(minY, y);
            maxX = Math.max(maxX, x);
            maxY = Math.max(maxY, y);
          }
        }

        if (maxX < minX || maxY < minY) {
          setProcessedSrc(src);
          return;
        }

        const boundsWidth = maxX - minX + 1;
        const boundsHeight = maxY - minY + 1;
        const outputSize = 64;
        const padding = 6;
        const scale = Math.min(
          (outputSize - padding * 2) / boundsWidth,
          (outputSize - padding * 2) / boundsHeight,
        );
        const drawWidth = boundsWidth * scale;
        const drawHeight = boundsHeight * scale;
        const dx = (outputSize - drawWidth) / 2;
        const dy = (outputSize - drawHeight) / 2;

        const outputCanvas = document.createElement("canvas");
        outputCanvas.width = outputSize;
        outputCanvas.height = outputSize;
        const outputCtx = outputCanvas.getContext("2d");
        if (!outputCtx) {
          setProcessedSrc(src);
          return;
        }
        outputCtx.clearRect(0, 0, outputSize, outputSize);
        outputCtx.imageSmoothingEnabled = true;
        outputCtx.imageSmoothingQuality = "high";
        outputCtx.drawImage(
          workCanvas,
          minX,
          minY,
          boundsWidth,
          boundsHeight,
          dx,
          dy,
          drawWidth,
          drawHeight,
        );

        setProcessedSrc(outputCanvas.toDataURL("image/png"));
      } catch {
        setProcessedSrc(src);
      }
    };

    image.onerror = () => {
      if (!cancelled) setProcessedSrc(src);
    };

    image.src = src;
    setProcessedSrc(src);

    return () => {
      cancelled = true;
    };
  }, [src]);

  return <img src={processedSrc} alt="" draggable={false} style={ICON_IMAGE_STYLE} />;
}

function TargetIcon({ target, mode }: { target: RecordingPickerTarget; mode: RecordingPickerMode }) {
  if (target.iconDataUrl) {
    return (
      <span style={ICON_FRAME_STYLE}>
        <ProcessedTargetIconImage src={target.iconDataUrl} />
      </span>
    );
  }
  const fallback = (target.exeName || target.title || "?").trim().slice(0, 1).toUpperCase();
  if (mode === "display") {
    return (
      <span style={ICON_FRAME_STYLE}>
        <DesktopOutlined style={{ fontSize: 18, color: "#1677ff" }} />
      </span>
    );
  }
  return (
    <span style={{ ...ICON_FRAME_STYLE, color: "#1677ff", fontSize: 12, fontWeight: 800 }}>
      {fallback || <AppstoreOutlined style={{ fontSize: 16 }} />}
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
