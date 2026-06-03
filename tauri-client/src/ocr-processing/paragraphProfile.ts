import type { OcrBlock } from "../types/screenshot";
import { getBoundsHeight, getBoundsWidth, getOcrBlockBounds } from "./blockGeometry";
import { restoreCollapsedUiTextSpacing } from "./textSpacing";

type ParagraphLine = {
  block: OcrBlock;
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
  width: number;
  height: number;
};

const cjkPattern = /[\u3040-\u30ff\u3400-\u9fff\uf900-\ufaff\uac00-\ud7af]/u;
const latinWordPattern = /[A-Za-z]{2,}/g;
const prosePunctuationPattern = /[,.!?;:，。！？；：、]/u;

const median = (values: number[]) => {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
};

const countMatches = (text: string, pattern: RegExp) => [...text.matchAll(pattern)].length;

const isMostlyCjk = (text: string) => {
  const compact = text.replace(/\s+/g, "");
  if (!compact) return false;
  const cjkCount = [...compact].filter((char) => cjkPattern.test(char)).length;
  return cjkCount / compact.length >= 0.45;
};

const looksLikeDenseProseLine = (text: string) => {
  const trimmed = text.trim();
  if (!trimmed) return false;
  if (/^(?:[A-Za-z0-9._+-]+,\s*){2,}[A-Za-z0-9._+-]+$/.test(trimmed)) return false;
  if (/^(?:Codex|API|Feedback|ChatGPT Apps SDK)\b/i.test(trimmed) && trimmed.length <= 48) return false;

  const latinWords = countMatches(trimmed, latinWordPattern);
  const cjkChars = [...trimmed].filter((char) => cjkPattern.test(char)).length;

  return (
    trimmed.length >= 34 ||
    latinWords >= 5 ||
    cjkChars >= 15 ||
    (trimmed.length >= 24 && prosePunctuationPattern.test(trimmed))
  );
};

const makeBlock = (items: ParagraphLine[], text: string): OcrBlock => ({
  text,
  confidence: items.reduce((sum, item) => sum + item.block.confidence, 0) / Math.max(1, items.length),
  box_coords: [
    [Math.min(...items.map((item) => item.minX)), Math.min(...items.map((item) => item.minY))],
    [Math.max(...items.map((item) => item.maxX)), Math.min(...items.map((item) => item.minY))],
    [Math.max(...items.map((item) => item.maxX)), Math.max(...items.map((item) => item.maxY))],
    [Math.min(...items.map((item) => item.minX)), Math.max(...items.map((item) => item.maxY))],
  ],
});

const joinParagraphText = (items: ParagraphLine[]) => {
  const mostlyCjkParagraph = isMostlyCjk(items.map((item) => item.block.text).join(""));
  const joined = items
    .map((item) => item.block.text.trim())
    .filter(Boolean)
    .join(mostlyCjkParagraph ? "" : " ");
  return mostlyCjkParagraph ? joined.replace(/\s+/g, " ").trim() : restoreCollapsedUiTextSpacing(joined);
};

const canMergeAdjacent = (
  previous: ParagraphLine,
  current: ParagraphLine,
  groupMinX: number,
  groupMaxWidth: number,
  medianHeight: number,
) => {
  const verticalGap = Math.max(0, current.minY - previous.maxY);
  const leftAligned = Math.abs(current.minX - groupMinX) <= Math.max(24, medianHeight * 1.35);
  const previousWidth = Math.max(previous.width, groupMaxWidth);
  const currentNotTiny = current.width >= Math.max(80, previousWidth * 0.34);
  const tightLineGap = verticalGap <= Math.max(12, medianHeight * 1.05);
  return leftAligned && currentNotTiny && tightLineGap;
};

const shouldFlushAsParagraph = (items: ParagraphLine[]) => {
  if (items.length < 3) return false;
  const text = items.map((item) => item.block.text.trim()).join(" ");
  const denseLineCount = items.filter((item) => looksLikeDenseProseLine(item.block.text)).length;
  const totalChars = text.replace(/\s+/g, "").length;
  return denseLineCount >= Math.max(3, Math.ceil(items.length * 0.65)) && totalChars >= 90;
};

export const applyParagraphProfile = (blocks: OcrBlock[]) => {
  const lines = blocks
    .map((block) => {
      const bounds = getOcrBlockBounds(block);
      return {
        block,
        ...bounds,
        width: getBoundsWidth(bounds),
        height: getBoundsHeight(bounds),
      };
    })
    .filter((item) => item.block.text.trim())
    .sort((a, b) => a.minY - b.minY || a.minX - b.minX);

  const medianHeight = median(lines.map((line) => line.height)) || 14;
  const output: OcrBlock[] = [];
  let group: ParagraphLine[] = [];

  const flush = () => {
    if (shouldFlushAsParagraph(group)) {
      output.push(makeBlock(group, joinParagraphText(group)));
    } else {
      output.push(...group.map((item) => item.block));
    }
    group = [];
  };

  for (const line of lines) {
    const previous = group[group.length - 1];
    const groupMinX = group.length ? Math.min(...group.map((item) => item.minX)) : line.minX;
    const groupMaxWidth = group.length ? Math.max(...group.map((item) => item.width)) : line.width;
    if (
      previous &&
      looksLikeDenseProseLine(previous.block.text) &&
      looksLikeDenseProseLine(line.block.text) &&
      canMergeAdjacent(previous, line, groupMinX, groupMaxWidth, medianHeight)
    ) {
      group.push(line);
      continue;
    }

    flush();
    group.push(line);
  }

  flush();
  return output;
};
