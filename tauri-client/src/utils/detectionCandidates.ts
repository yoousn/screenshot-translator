import type { Rect } from "../types/screenshot";
import { clamp } from "./annotationGeometry";

export const rectSignature = (rect: Rect) => `${Math.round(rect.x / 3)}:${Math.round(rect.y / 3)}:${Math.round(rect.w / 3)}:${Math.round(rect.h / 3)}`;

export const sortDetectionCandidates = (candidates: Rect[], mx: number, my: number) => {
  const seen = new Set<string>();
  const unique = candidates.filter((candidate) => {
    const key = rectSignature(candidate);
    if (seen.has(key)) return false;
    seen.add(key);
    return candidate.w >= 12 && candidate.h >= 12;
  });
  return unique.sort((a, b) => {
    const priority = (rect: Rect) => rect.kind === "control" ? 0 : rect.kind === "window" ? 1 : 2;
    const areaA = a.w * a.h;
    const areaB = b.w * b.h;
    const centerA = Math.hypot(mx - (a.x + a.w / 2), my - (a.y + a.h / 2));
    const centerB = Math.hypot(mx - (b.x + b.w / 2), my - (b.y + b.h / 2));
    return priority(a) - priority(b) || areaA - areaB || centerA - centerB;
  });
};

const getPixel = (imageData: ImageData, x: number, y: number) => {
  const px = clamp(Math.round(x), 0, imageData.width - 1);
  const py = clamp(Math.round(y), 0, imageData.height - 1);
  const idx = (py * imageData.width + px) * 4;
  const data = imageData.data;
  return [data[idx], data[idx + 1], data[idx + 2]];
};

const pixelDiff = (a: number[], b: number[]) => (
  Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2])
) / 3;

const verticalEdgeScore = (imageData: ImageData, x: number, y: number, span: number) => {
  let score = 0;
  let count = 0;
  for (let yy = y - span; yy <= y + span; yy += 8) {
    if (yy <= 1 || yy >= imageData.height - 2) continue;
    score += pixelDiff(getPixel(imageData, x, yy), getPixel(imageData, x - 1, yy));
    count += 1;
  }
  return count ? score / count : 0;
};

const horizontalEdgeScore = (imageData: ImageData, x: number, y: number, span: number) => {
  let score = 0;
  let count = 0;
  for (let xx = x - span; xx <= x + span; xx += 8) {
    if (xx <= 1 || xx >= imageData.width - 2) continue;
    score += pixelDiff(getPixel(imageData, xx, y), getPixel(imageData, xx, y - 1));
    count += 1;
  }
  return count ? score / count : 0;
};

const findVisualBoundary = (
  imageData: ImageData,
  mx: number,
  my: number,
  direction: "left" | "right" | "top" | "bottom",
  span: number,
  threshold: number,
) => {
  const step = direction === "left" || direction === "top" ? -2 : 2;
  const horizontal = direction === "top" || direction === "bottom";
  const max = horizontal ? imageData.height - 2 : imageData.width - 2;
  let pos = horizontal ? my : mx;
  for (pos += step; pos > 2 && pos < max; pos += step) {
    const score = horizontal
      ? horizontalEdgeScore(imageData, mx, pos, span)
      : verticalEdgeScore(imageData, pos, my, span);
    if (score >= threshold) return pos;
  }
  return null;
};

export const getVisualRectsAt = (imageData: ImageData | null, mx: number, my: number, sensitivityInput: number): Rect[] => {
  if (!imageData) return [];
  const width = imageData.width;
  const height = imageData.height;
  if (mx < 0 || my < 0 || mx >= width || my >= height) return [];

  const sensitivity = clamp(sensitivityInput || 3, 1, 5);
  const thresholdOffset = (3 - sensitivity) * 4;
  const attempts = [
    { span: 128, threshold: 18 + thresholdOffset },
    { span: 96, threshold: 16 + thresholdOffset },
    { span: 64, threshold: 20 + thresholdOffset },
    { span: 36, threshold: 26 + thresholdOffset },
  ];

  const matches: Rect[] = [];
  for (const attempt of attempts) {
    const left = findVisualBoundary(imageData, mx, my, "left", attempt.span, attempt.threshold);
    const right = findVisualBoundary(imageData, mx, my, "right", attempt.span, attempt.threshold);
    const top = findVisualBoundary(imageData, mx, my, "top", attempt.span, attempt.threshold);
    const bottom = findVisualBoundary(imageData, mx, my, "bottom", attempt.span, attempt.threshold);
    if (left === null || right === null || top === null || bottom === null) continue;
    const rect = {
      x: clamp(Math.min(left, right), 0, width - 1),
      y: clamp(Math.min(top, bottom), 0, height - 1),
      w: Math.max(1, Math.abs(right - left)),
      h: Math.max(1, Math.abs(bottom - top)),
      kind: "visual" as const,
    };
    const area = rect.w * rect.h;
    const screenArea = width * height;
    const minW = sensitivity >= 4 ? 56 : 80;
    const minH = sensitivity >= 4 ? 28 : 40;
    if (rect.w >= minW && rect.h >= minH && area < screenArea * 0.9) {
      const cursorMarginX = Math.min(mx - rect.x, rect.x + rect.w - mx);
      const cursorMarginY = Math.min(my - rect.y, rect.y + rect.h - my);
      const cursorTooCloseToEdge = cursorMarginX < 3 || cursorMarginY < 3;
      if (!cursorTooCloseToEdge || sensitivity >= 5) matches.push(rect);
    }
  }
  return sortDetectionCandidates(matches, mx, my);
};

export const getDetectionCandidatesAt = (
  mx: number,
  my: number,
  windowRects: Rect[],
  imageData: ImageData | null,
  visualEnabled: boolean,
  sensitivityInput: number,
) => {
  const sensitivity = clamp(sensitivityInput || 3, 1, 5);
  const candidates: Rect[] = [];
  for (const candidate of windowRects) {
    if (mx >= candidate.x && mx <= candidate.x + candidate.w && my >= candidate.y && my <= candidate.y + candidate.h) {
      candidates.push(candidate);
    }
  }
  if (visualEnabled && (candidates.length === 0 || sensitivity >= 4)) candidates.push(...getVisualRectsAt(imageData, mx, my, sensitivity));
  return sortDetectionCandidates(candidates, mx, my);
};
