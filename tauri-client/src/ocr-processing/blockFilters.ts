import type { OcrBlock } from "../types/screenshot";
import { getBoundsHeight, getBoundsWidth, getOcrBlockBounds } from "./blockGeometry";

const iconOnlyPattern = /^[oO○●•·■□▪▫◾◼◆◇×+*☆★←→①②③④⑤⑥⑦⑧⑨⑩]$/u;

export const isUsefulOcrBlock = (block: OcrBlock) => {
  const text = block.text.trim();
  if (!text) return false;
  if (block.box_coords.length < 4) return false;

  const bounds = getOcrBlockBounds(block);
  const width = getBoundsWidth(bounds);
  const height = getBoundsHeight(bounds);

  if (iconOnlyPattern.test(text) && block.confidence < 0.92) return false;
  if (iconOnlyPattern.test(text) && width <= height * 1.8) return false;
  if (text.length === 1 && block.confidence < 0.55 && width <= height * 1.6) return false;

  return true;
};

export const filterUsefulOcrBlocks = (blocks: OcrBlock[]) => blocks.filter(isUsefulOcrBlock);
