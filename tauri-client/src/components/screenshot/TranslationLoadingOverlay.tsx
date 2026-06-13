import React from "react";

interface TranslationLoadingOverlayProps {
  rect: { x: number; y: number; w: number; h: number };
}

export default function TranslationLoadingOverlay({ rect }: TranslationLoadingOverlayProps) {
  if (rect.w <= 0 || rect.h <= 0) return null;
  const showLabel = rect.w >= 86 && rect.h >= 58;
  const labelBlockHeight = showLabel ? 24 : 0;
  const availableSpinnerSpace = Math.max(12, Math.min(rect.w - 12, rect.h - labelBlockHeight - 12));
  const spinnerSize = Math.max(12, Math.min(28, Math.round(availableSpinnerSpace * 0.58)));
  const spinnerBorder = Math.max(2, Math.round(spinnerSize / 10));

  return (
    <div
      style={{
        position: "absolute",
        top: rect.y,
        left: rect.x,
        width: rect.w,
        height: rect.h,
        zIndex: 200,
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        gap: showLabel ? 6 : 0,
        padding: 6,
        overflow: "hidden",
        pointerEvents: "none",
        background: "rgba(240, 240, 245, 0.75)",
        border: "2px dashed #1677ff",
        boxSizing: "border-box",
      }}
    >
        <div style={{ width: spinnerSize, height: spinnerSize, minWidth: spinnerSize, minHeight: spinnerSize, flex: `0 0 ${spinnerSize}px`, borderRadius: "50%", border: `${spinnerBorder}px solid #e0e0e0`, borderTopColor: "#1677ff", animation: "spin 0.8s linear infinite", boxSizing: "border-box" }} />
        {showLabel && <div style={{ color: "#1677ff", fontSize: 12, lineHeight: "16px", fontFamily: "'Inter', sans-serif", fontWeight: 500, whiteSpace: "nowrap", textShadow: "0 1px 2px rgba(255,255,255,0.8)" }}>翻译中...</div>}
    </div>
  );
}
