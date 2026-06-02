import type { OcrBlock } from "../types/screenshot";

export const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

export const median = (values: number[]) => {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
};

export const getBlockBounds = (block: OcrBlock) => {
  const xs = block.box_coords.map((point) => point[0]);
  const ys = block.box_coords.map((point) => point[1]);
  return {
    minX: Math.min(...xs),
    maxX: Math.max(...xs),
    minY: Math.min(...ys),
    maxY: Math.max(...ys),
  };
};

export const isLikelyVerticalText = (block: OcrBlock) => {
  const bounds = getBlockBounds(block);
  const width = Math.max(1, bounds.maxX - bounds.minX);
  const height = Math.max(1, bounds.maxY - bounds.minY);
  const compactText = block.text.replace(/\s+/g, "");
  return height / width > 2.6 && compactText.length >= 3 && !/[A-Za-z0-9]{2,}/.test(compactText);
};

export const isLikelyRtlText = (text: string) => /[\u0590-\u08FF]/.test(text);
