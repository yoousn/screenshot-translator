import type { Annotation, AnnotationTool, Point, Rect } from "../types/screenshot";

export type AnnotationResizeHandle = "n" | "s" | "e" | "w" | "nw" | "ne" | "sw" | "se";
export type AnnotationHit = { index: number; action: "move" | "resize"; handle?: AnnotationResizeHandle; cursor: string };

export const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(value, max));

export const normalizedRectFromPoints = (start: Point, end: Point, selection: Rect): Rect => {
  const x1 = clamp(start.x, selection.x, selection.x + selection.w);
  const y1 = clamp(start.y, selection.y, selection.y + selection.h);
  const x2 = clamp(end.x, selection.x, selection.x + selection.w);
  const y2 = clamp(end.y, selection.y, selection.y + selection.h);
  return { x: Math.min(x1, x2), y: Math.min(y1, y2), w: Math.abs(x2 - x1), h: Math.abs(y2 - y1) };
};

export const makeLineAnnotation = (
  tool: AnnotationTool,
  start: Point,
  end: Point,
  selection: Rect,
  color: string,
  size: number,
): Annotation => ({
  type: tool,
  rect: normalizedRectFromPoints(start, end, selection),
  color,
  size,
  points: [
    { x: clamp(start.x, selection.x, selection.x + selection.w), y: clamp(start.y, selection.y, selection.y + selection.h) },
    { x: clamp(end.x, selection.x, selection.x + selection.w), y: clamp(end.y, selection.y, selection.y + selection.h) },
  ],
});

export const makeTextAnnotation = (point: Point, text: string, color: string, baseSize: number): Annotation => {
  const fontSize = Math.max(14, baseSize + 14);
  return {
    type: "text",
    rect: { x: point.x, y: point.y, w: Math.max(48, text.length * fontSize * 0.72 + 12), h: fontSize + 8 },
    text,
    color,
    size: fontSize,
  };
};

export const isDraggableAnnotation = (annotation: Annotation) => annotation.type === "rect" || annotation.type === "circle" || annotation.type === "text";


const resizeCursors: Record<AnnotationResizeHandle, string> = {
  nw: "nwse-resize",
  ne: "nesw-resize",
  sw: "nesw-resize",
  se: "nwse-resize",
  n: "ns-resize",
  s: "ns-resize",
  w: "ew-resize",
  e: "ew-resize",
};

const getRectResizeHandle = (rect: Rect, point: Point, tolerance: number): AnnotationResizeHandle | null => {
  const left = rect.x;
  const right = rect.x + rect.w;
  const top = rect.y;
  const bottom = rect.y + rect.h;
  const nearX = point.x >= left - tolerance && point.x <= right + tolerance;
  const nearY = point.y >= top - tolerance && point.y <= bottom + tolerance;
  const nearLeft = Math.abs(point.x - left) <= tolerance && nearY;
  const nearRight = Math.abs(point.x - right) <= tolerance && nearY;
  const nearTop = Math.abs(point.y - top) <= tolerance && nearX;
  const nearBottom = Math.abs(point.y - bottom) <= tolerance && nearX;

  if (nearLeft && nearTop) return "nw";
  if (nearRight && nearTop) return "ne";
  if (nearLeft && nearBottom) return "sw";
  if (nearRight && nearBottom) return "se";
  if (nearTop) return "n";
  if (nearBottom) return "s";
  if (nearLeft) return "w";
  if (nearRight) return "e";
  return null;
};

const isInsideRect = (rect: Rect, point: Point) => (
  point.x >= rect.x && point.x <= rect.x + rect.w && point.y >= rect.y && point.y <= rect.y + rect.h
);

const isInsideEllipse = (rect: Rect, point: Point) => {
  const rx = Math.max(1, rect.w / 2);
  const ry = Math.max(1, rect.h / 2);
  const cx = rect.x + rx;
  const cy = rect.y + ry;
  const nx = (point.x - cx) / rx;
  const ny = (point.y - cy) / ry;
  return nx * nx + ny * ny <= 1;
};

export const hitAnnotationDetailed = (annotations: Annotation[], point: Point, fallbackSize: number): AnnotationHit | null => {
  for (let index = annotations.length - 1; index >= 0; index--) {
    const annotation = annotations[index];
    const rect = annotation.rect;
    const tolerance = Math.max(6, Math.min(12, annotation.size || fallbackSize));

    if (annotation.type === "rect" || annotation.type === "circle") {
      const handle = getRectResizeHandle(rect, point, tolerance);
      if (handle) return { index, action: "resize", handle, cursor: resizeCursors[handle] };
      const inside = annotation.type === "circle" ? isInsideEllipse(rect, point) : isInsideRect(rect, point);
      if (inside) return { index, action: "move", cursor: "move" };
      continue;
    }

    if (annotation.type === "text" && isInsideRect(rect, point)) {
      return { index, action: "move", cursor: "text" };
    }
  }
  return null;
};

export const hitAnnotation = (annotations: Annotation[], point: Point, fallbackSize: number) => {
  for (let index = annotations.length - 1; index >= 0; index--) {
    const annotation = annotations[index];
    const rect = annotation.rect;
    const tolerance = Math.max(8, annotation.size || fallbackSize);
    if (point.x >= rect.x - tolerance && point.x <= rect.x + rect.w + tolerance && point.y >= rect.y - tolerance && point.y <= rect.y + rect.h + tolerance) {
      return index;
    }
  }
  return null;
};

export const moveAnnotation = (annotation: Annotation, dx: number, dy: number, selection: Rect): Annotation => {
  const minDx = selection.x - annotation.rect.x;
  const maxDx = selection.x + selection.w - (annotation.rect.x + annotation.rect.w);
  const minDy = selection.y - annotation.rect.y;
  const maxDy = selection.y + selection.h - (annotation.rect.y + annotation.rect.h);
  const boundedDx = clamp(dx, minDx, maxDx);
  const boundedDy = clamp(dy, minDy, maxDy);
  return {
    ...annotation,
    rect: { ...annotation.rect, x: annotation.rect.x + boundedDx, y: annotation.rect.y + boundedDy },
    points: annotation.points?.map((item) => ({ x: item.x + boundedDx, y: item.y + boundedDy })),
  };
};

export const resizeAnnotation = (
  annotation: Annotation,
  handle: AnnotationResizeHandle,
  dx: number,
  dy: number,
  selection: Rect,
): Annotation => {
  const original = annotation.rect;
  let x1 = original.x;
  let y1 = original.y;
  let x2 = original.x + original.w;
  let y2 = original.y + original.h;

  if (handle.includes("e")) x2 += dx;
  if (handle.includes("w")) x1 += dx;
  if (handle.includes("s")) y2 += dy;
  if (handle.includes("n")) y1 += dy;

  x1 = clamp(x1, selection.x, selection.x + selection.w);
  x2 = clamp(x2, selection.x, selection.x + selection.w);
  y1 = clamp(y1, selection.y, selection.y + selection.h);
  y2 = clamp(y2, selection.y, selection.y + selection.h);

  const minSize = Math.max(8, annotation.size || 4);
  if (Math.abs(x2 - x1) < minSize) {
    if (handle.includes("w")) x1 = x2 - Math.sign(x2 - x1 || 1) * minSize;
    else x2 = x1 + Math.sign(x2 - x1 || 1) * minSize;
  }
  if (Math.abs(y2 - y1) < minSize) {
    if (handle.includes("n")) y1 = y2 - Math.sign(y2 - y1 || 1) * minSize;
    else y2 = y1 + Math.sign(y2 - y1 || 1) * minSize;
  }

  const nextRect = {
    x: clamp(Math.min(x1, x2), selection.x, selection.x + selection.w),
    y: clamp(Math.min(y1, y2), selection.y, selection.y + selection.h),
    w: Math.min(Math.abs(x2 - x1), selection.w),
    h: Math.min(Math.abs(y2 - y1), selection.h),
  };

  return { ...annotation, rect: nextRect };
};

