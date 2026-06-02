import type { OcrBlock } from "../types/screenshot";
import { getBoundsCenterY, getBoundsHeight, getOcrBlockBounds } from "./blockGeometry";
import { filterUsefulOcrBlocks } from "./blockFilters";
import { restoreCollapsedUiTextSpacing } from "./textSpacing";

const median = (values: number[]) => {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
};

export const buildVirtualOcrLines = (blocks: OcrBlock[]) => {
  const items = filterUsefulOcrBlocks(blocks)
    .map((block) => ({ block, bounds: getOcrBlockBounds(block) }))
    .sort((a, b) => getBoundsCenterY(a.bounds) - getBoundsCenterY(b.bounds) || a.bounds.minX - b.bounds.minX);

  const medianHeight = median(items.map((item) => getBoundsHeight(item.bounds))) || 12;
  const rowTolerance = Math.max(6, medianHeight * 0.65);
  const rows: typeof items[] = [];

  for (const item of items) {
    const centerY = getBoundsCenterY(item.bounds);
    const row = rows.find((candidate) => {
      const rowCenter = candidate.reduce((sum, entry) => sum + getBoundsCenterY(entry.bounds), 0) / candidate.length;
      return Math.abs(rowCenter - centerY) <= rowTolerance;
    });
    if (row) row.push(item);
    else rows.push([item]);
  }

  return rows.map((row) => {
    const sorted = [...row].sort((a, b) => a.bounds.minX - b.bounds.minX);
    const minX = Math.min(...sorted.map((item) => item.bounds.minX));
    const minY = Math.min(...sorted.map((item) => item.bounds.minY));
    const maxX = Math.max(...sorted.map((item) => item.bounds.maxX));
    const maxY = Math.max(...sorted.map((item) => item.bounds.maxY));
    const text = restoreCollapsedUiTextSpacing(sorted.map((item) => item.block.text.trim()).join(" "));
    return {
      text,
      confidence: sorted.reduce((sum, item) => sum + item.block.confidence, 0) / sorted.length,
      box_coords: [[minX, minY], [maxX, minY], [maxX, maxY], [minX, maxY]] as [number, number][],
    } satisfies OcrBlock;
  }).filter((block) => block.text);
};
