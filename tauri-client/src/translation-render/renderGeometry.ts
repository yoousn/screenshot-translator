import type { RenderBlock } from "./types";
import { clamp } from "./geometry";

export type TranslationEraseRegion = {
  eraseX: number;
  eraseY: number;
  eraseRight: number;
  eraseBottom: number;
  drawWidth: number;
  drawHeight: number;
};

export const normalizeRenderedText = (text: string) => text.replace(/\s+/g, " ").trim().toLowerCase();

export const shouldRenderTranslationBlock = (block: RenderBlock) => {
  const text = block.translated || block.text;
  if (!text) return false;
  return normalizeRenderedText(text) !== normalizeRenderedText(block.text);
};

export const buildTranslationEraseRegion = (
  block: RenderBlock,
  imageWidth: number,
  imageHeight: number,
  desiredWidth: number,
  paddingX: number,
  paddingY: number,
): TranslationEraseRegion => {
  const rawWidth = Math.max(1, block.maxX - block.minX);
  const extraWidth = Math.max(0, desiredWidth - rawWidth);
  const eraseX = clamp(Math.round(block.direction === "rtl" ? block.minX - extraWidth - paddingX : block.minX - paddingX), 0, imageWidth - 1);
  const eraseY = clamp(Math.round(block.minY - paddingY), 0, imageHeight - 1);
  const eraseRight = clamp(Math.round(block.direction === "rtl" ? block.maxX + paddingX : block.maxX + extraWidth + paddingX), eraseX + 1, imageWidth);
  const eraseBottom = clamp(Math.round(block.maxY + paddingY), eraseY + 1, imageHeight);
  return {
    eraseX,
    eraseY,
    eraseRight,
    eraseBottom,
    drawWidth: Math.max(1, eraseRight - eraseX - paddingX * 2),
    drawHeight: Math.max(1, eraseBottom - eraseY - paddingY * 2),
  };
};
