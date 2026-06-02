import { CheckCircleOutlined } from "@ant-design/icons";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect } from "react";

const NOTICE_DURATION_MS = 1800;

const getNoticeText = () => {
  const params = new URLSearchParams(window.location.search);
  return params.get("text") || "Recording saved";
};

export default function RecordingNoticePage() {
  const text = getNoticeText();

  useEffect(() => {
    const timer = window.setTimeout(() => {
      getCurrentWindow().close().catch(() => {});
    }, NOTICE_DURATION_MS);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        pointerEvents: "none",
        background: "transparent",
      }}
    >
      <div
        style={{
          height: 42,
          maxWidth: "calc(100vw - 16px)",
          display: "inline-flex",
          alignItems: "center",
          gap: 8,
          padding: "0 14px",
          borderRadius: 999,
          background: "rgba(255,255,255,0.96)",
          border: "1px solid rgba(187,247,208,0.95)",
          color: "#14532d",
          fontSize: 13,
          fontWeight: 800,
          whiteSpace: "nowrap",
          boxSizing: "border-box",
          backdropFilter: "blur(12px)",
          WebkitBackdropFilter: "blur(12px)",
        }}
      >
        <CheckCircleOutlined style={{ color: "#16a34a" }} />
        <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{text}</span>
      </div>
    </div>
  );
}
