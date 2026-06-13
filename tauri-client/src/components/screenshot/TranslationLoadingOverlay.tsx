import React from "react";

interface TranslationLoadingOverlayProps {
  rect: { x: number; y: number; w: number; h: number };
}

export default function TranslationLoadingOverlay({ rect }: TranslationLoadingOverlayProps) {
  if (rect.w <= 0 || rect.h <= 0) return null;
  const isSmall = rect.w < 80 || rect.h < 40;
  const spinnerSize = Math.max(12, Math.min(32, Math.round(Math.min(rect.w, rect.h) * 0.6)));
  const spinnerBorder = Math.max(2, Math.round(spinnerSize / 10));
  return (
    <>
      <div style={{ position: "absolute", top: rect.y, left: rect.x, width: rect.w, height: rect.h, zIndex: 200, background: "rgba(240, 240, 245, 0.75)", border: "2px dashed #1677ff", boxSizing: "border-box" }} />
      <div style={{ position: "absolute", left: rect.x + rect.w / 2, top: rect.y + rect.h / 2, transform: "translate(-50%, -50%)", zIndex: 201, display: "flex", flexDirection: "column", alignItems: "center", pointerEvents: "none" }}>
        <div style={{ width: spinnerSize, height: spinnerSize, minWidth: spinnerSize, minHeight: spinnerSize, flex: `0 0 ${spinnerSize}px`, borderRadius: "50%", border: `${spinnerBorder}px solid #e0e0e0`, borderTopColor: "#1677ff", animation: "spin 0.8s linear infinite", boxSizing: "border-box" }} />
        {!isSmall && <div style={{ marginTop: 8, color: "#1677ff", fontSize: 12, fontFamily: "'Inter', sans-serif", fontWeight: 500, whiteSpace: "nowrap", textShadow: "0 1px 2px rgba(255,255,255,0.8)" }}>翻译中...</div>}
      </div>
    </>
  );
}
