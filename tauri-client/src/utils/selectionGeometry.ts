import type { Rect } from "../types/screenshot";

export const getHandleAt = (rect: Rect, hasSelected: boolean, mx: number, my: number, isClick = false) => {
  if (!hasSelected) return null;
  const { x, y, w, h } = rect;
  const tolerance = 8;
  const points = {
    nw: { x, y, cursor: "nwse-resize" },
    ne: { x: x + w, y, cursor: "nesw-resize" },
    sw: { x, y: y + h, cursor: "nesw-resize" },
    se: { x: x + w, y: y + h, cursor: "nwse-resize" },
    n: { x: x + w / 2, y, cursor: "ns-resize" },
    s: { x: x + w / 2, y: y + h, cursor: "ns-resize" },
    w: { x, y: y + h / 2, cursor: "ew-resize" },
    e: { x: x + w, y: y + h / 2, cursor: "ew-resize" },
  };

  for (const [key, point] of Object.entries(points)) {
    if (Math.abs(mx - point.x) <= tolerance && Math.abs(my - point.y) <= tolerance) return { handle: key, cursor: point.cursor };
  }

  if (mx >= x && mx <= x + w && my >= y && my <= y + h) return { handle: "move", cursor: "move" };

  if (isClick) {
    let nearestKey = "se";
    let minDistance = Infinity;
    let nearestCursor = "nwse-resize";
    for (const [key, point] of Object.entries(points)) {
      const dist = Math.hypot(mx - point.x, my - point.y);
      if (dist < minDistance) {
        minDistance = dist;
        nearestKey = key;
        nearestCursor = point.cursor;
      }
    }
    return { handle: nearestKey, cursor: nearestCursor };
  }

  return null;
};

export const isPointInSelection = (rect: Rect, hasSelected: boolean, x: number, y: number) => (
  hasSelected && x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
);
