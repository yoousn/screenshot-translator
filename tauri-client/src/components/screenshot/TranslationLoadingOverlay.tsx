import React from "react";

interface TranslationLoadingOverlayProps {
  rect: { x: number; y: number; w: number; h: number };
}

export default function TranslationLoadingOverlay({ rect }: TranslationLoadingOverlayProps) {
  if (rect.w <= 0 || rect.h <= 0) return null;
  const borderWidth = rect.w < 42 || rect.h < 42 ? 1 : 2;
  const innerInset = Math.min(6, Math.max(2, Math.floor(Math.min(rect.w, rect.h) / 8)));
  const innerWidth = Math.max(0, rect.w - borderWidth * 2 - innerInset * 2);
  const innerHeight = Math.max(0, rect.h - borderWidth * 2 - innerInset * 2);
  const showLabel = innerWidth >= 96 && innerHeight >= 58;
  const labelHeight = showLabel ? 14 : 0;
  const gap = showLabel ? Math.min(4, Math.max(2, Math.floor(innerHeight / 18))) : 0;
  const spinnerBox = Math.max(8, Math.min(innerWidth, innerHeight - labelHeight - gap));
  const spinnerSize = Math.max(8, Math.min(showLabel ? 22 : 26, Math.round(spinnerBox)));
  const spinnerBorder = Math.max(2, Math.min(3, Math.round(spinnerSize / 9)));

  return (
    <div
      style={{
        position: "absolute",
        top: rect.y,
        left: rect.x,
        width: rect.w,
        height: rect.h,
        zIndex: 200,
        overflow: "hidden",
        pointerEvents: "none",
        background: "rgba(240, 240, 245, 0.75)",
        border: `${borderWidth}px dashed #1677ff`,
        boxSizing: "border-box",
        clipPath: "inset(0 round 2px)",
      }}
    >
      <div
        style={{
          position: "absolute",
          inset: innerInset,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          gap,
          minWidth: 0,
          minHeight: 0,
          overflow: "hidden",
          boxSizing: "border-box",
        }}
      >
        <div style={{ width: spinnerSize, height: spinnerSize, minWidth: spinnerSize, minHeight: spinnerSize, flex: `0 0 ${spinnerSize}px`, borderRadius: "50%", border: `${spinnerBorder}px solid #e0e0e0`, borderTopColor: "#1677ff", animation: "spin 0.8s linear infinite", boxSizing: "border-box" }} />
        {showLabel && <div style={{ color: "#1677ff", fontSize: 11, lineHeight: `${labelHeight}px`, fontFamily: "'Inter', sans-serif", fontWeight: 500, whiteSpace: "nowrap", textShadow: "0 1px 2px rgba(255,255,255,0.8)" }}>翻译中...</div>}
      </div>
    </div>
  );
}
