import type { OcrBlock } from "../types/screenshot";
import type { RenderBlock } from "./types";
import { getBlockBounds, isLikelyRtlText, isLikelyVerticalText, median } from "./geometry";

export const buildRenderBlocks = (blocks: OcrBlock[], translations: string[]): RenderBlock[] => {
  const items = blocks
    .map((block, index) => ({ block, translation: translations[index] || block.text, bounds: getBlockBounds(block) }))
    .filter((item) => item.block.box_coords.length >= 4)
    .sort((a, b) => (a.bounds.minY + a.bounds.maxY) / 2 - (b.bounds.minY + b.bounds.maxY) / 2 || a.bounds.minX - b.bounds.minX);

  const heights = items.map((item) => Math.max(1, item.bounds.maxY - item.bounds.minY));
  const rowTolerance = Math.max(8, median(heights) * 0.65);
  const rows: typeof items[] = [];

  for (const item of items) {
    if (isLikelyVerticalText(item.block)) {
      rows.push([item]);
      continue;
    }

    const centerY = (item.bounds.minY + item.bounds.maxY) / 2;
    const row = rows.find((candidate) => {
      if (candidate.some((entry) => isLikelyVerticalText(entry.block))) return false;
      const rowCenterY = candidate.reduce((sum, entry) => sum + (entry.bounds.minY + entry.bounds.maxY) / 2, 0) / candidate.length;
      return Math.abs(rowCenterY - centerY) <= rowTolerance;
    });

    if (row) row.push(item);
    else rows.push([item]);
  }

  return rows.flatMap((row) => {
    const sortedRow = [...row].sort((a, b) => a.bounds.minX - b.bounds.minX);
    const rowHeights = sortedRow.map((item) => Math.max(1, item.bounds.maxY - item.bounds.minY));
    const rowHeight = Math.max(1, median(rowHeights));
    const groups: typeof sortedRow[] = [];

    for (const item of sortedRow) {
      const previousGroup = groups[groups.length - 1];
      const previous = previousGroup?.[previousGroup.length - 1];
      const gap = previous ? item.bounds.minX - previous.bounds.maxX : Infinity;
      const shouldMerge = previous && !isLikelyVerticalText(item.block) && !isLikelyVerticalText(previous.block) && gap <= Math.max(18, rowHeight * 1.8);
      if (shouldMerge) previousGroup.push(item);
      else groups.push([item]);
    }

    return groups.map((group) => {
      const text = group.map((item) => item.block.text).join(" ").replace(/\s+/g, " ").trim();
      const translated = group.map((item) => item.translation).join(" ").replace(/\s+/g, " ").trim();
      return {
        text,
        translated,
        minX: Math.min(...group.map((item) => item.bounds.minX)),
        minY: Math.min(...group.map((item) => item.bounds.minY)),
        maxX: Math.max(...group.map((item) => item.bounds.maxX)),
        maxY: Math.max(...group.map((item) => item.bounds.maxY)),
        direction: isLikelyRtlText(translated || text) ? "rtl" : "ltr",
      } satisfies RenderBlock;
    });
  });
};
