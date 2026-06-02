import type { OcrBlock } from "../types/screenshot";
import type { BlockBounds } from "./types";

export const getOcrBlockBounds = (block: OcrBlock): BlockBounds => {
  const xs = block.box_coords.map((point) => point[0]);
  const ys = block.box_coords.map((point) => point[1]);
  return { minX: Math.min(...xs), minY: Math.min(...ys), maxX: Math.max(...xs), maxY: Math.max(...ys) };
};

export const getBoundsWidth = (bounds: BlockBounds) => Math.max(1, bounds.maxX - bounds.minX);
export const getBoundsHeight = (bounds: BlockBounds) => Math.max(1, bounds.maxY - bounds.minY);
export const getBoundsCenterY = (bounds: BlockBounds) => (bounds.minY + bounds.maxY) / 2;
