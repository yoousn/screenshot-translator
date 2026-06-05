import type React from "react";
import type { Rect } from "../types/screenshot";
import { clamp } from "./annotationGeometry";

type ToolbarStyleOptions = {
  rect: Rect;
  toolbarSize: { width: number; height: number };
  fallbackSize: { width: number; height: number };
  viewportWidth: number;
  viewportHeight: number;
  margin: number;
  gap: number;
};

export const getActionToolbarStyle = ({
  rect,
  toolbarSize,
  fallbackSize,
  viewportWidth,
  viewportHeight,
  margin,
  gap,
}: ToolbarStyleOptions): React.CSSProperties => {
  const toolbarWidth = toolbarSize.width || fallbackSize.width;
  const toolbarHeight = toolbarSize.height || fallbackSize.height;
  const maxLeft = Math.max(margin, viewportWidth - toolbarWidth - margin);
  const maxTop = Math.max(margin, viewportHeight - toolbarHeight - margin);
  const hasSpaceBelow = rect.y + rect.h + gap + toolbarHeight <= viewportHeight - margin;
  const topCandidate = hasSpaceBelow ? rect.y + rect.h + gap : rect.y - toolbarHeight - gap;
  const leftCandidate = rect.x + rect.w - toolbarWidth >= margin ? rect.x + rect.w - toolbarWidth : rect.x;

  return {
    position: "absolute",
    top: clamp(topCandidate, margin, maxTop),
    left: clamp(leftCandidate, margin, maxLeft),
    zIndex: 320,
    background: "#fff",
    padding: "6px 10px",
    borderRadius: 8,
    boxShadow: "0 2px 12px rgba(0, 0, 0, 0.12)",
    border: "1px solid #e8e8e8",
    width: "fit-content",
    maxWidth: `calc(100vw - ${margin * 2}px)`,
    boxSizing: "border-box",
    whiteSpace: "normal",
  };
};

export const FLOATING_PANEL_MARGIN = 8;
export const FLOATING_PANEL_GAP = 8;
export const OCR_WINDOW_SIZE = { width: 500, height: 400 };
