import React from "react";

interface DelayedCountdownOverlayProps {
  countdown: number;
  title: string;
  onCancel: () => void;
}

export default function DelayedCountdownOverlay({ countdown, title, onCancel }: DelayedCountdownOverlayProps) {
  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 9999,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "rgba(15, 23, 42, 0.28)",
        backdropFilter: "blur(2px)",
        pointerEvents: "auto",
      }}
      onClick={onCancel}
    >
      <div
        style={{
          minWidth: 220,
          padding: "26px 34px",
          borderRadius: 24,
          textAlign: "center",
          color: "#fff",
          background: "rgba(15, 23, 42, 0.86)",
          boxShadow: "0 24px 80px rgba(0,0,0,0.28)",
          border: "1px solid rgba(255,255,255,0.18)",
        }}
      >
        <div style={{ fontSize: 14, letterSpacing: 2, opacity: 0.82 }}>{title}</div>
        <div style={{ fontSize: 86, lineHeight: 1, fontWeight: 800, margin: "10px 0 12px" }}>{countdown}</div>
        <div style={{ fontSize: 13, opacity: 0.72 }}>点击任意位置取消</div>
      </div>
    </div>
  );
}
