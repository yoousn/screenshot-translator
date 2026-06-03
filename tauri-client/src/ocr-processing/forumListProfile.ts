import type { OcrBlock } from "../types/screenshot";
import { getBoundsHeight, getOcrBlockBounds } from "./blockGeometry";
import { restoreCollapsedUiTextSpacing } from "./textSpacing";

type LineItem = {
  block: OcrBlock;
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
  height: number;
};

const median = (values: number[]) => {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
};

const makeBlock = (items: LineItem[], text: string): OcrBlock => ({
  text,
  confidence: items.reduce((sum, item) => sum + item.block.confidence, 0) / Math.max(1, items.length),
  box_coords: [
    [Math.min(...items.map((item) => item.minX)), Math.min(...items.map((item) => item.minY))],
    [Math.max(...items.map((item) => item.maxX)), Math.min(...items.map((item) => item.minY))],
    [Math.max(...items.map((item) => item.maxX)), Math.max(...items.map((item) => item.maxY))],
    [Math.min(...items.map((item) => item.minX)), Math.max(...items.map((item) => item.maxY))],
  ],
});

export const normalizeForumListText = (text: string) => {
  let next = restoreCollapsedUiTextSpacing(text)
    .replace(/\bOpen\s*Al\b/g, "OpenAI")
    .replace(/\bA\s*Pls\b/g, "APIs")
    .replace(/\bAl-generated\b/g, "AI-generated")
    .replace(/\bChatGPTApps\b/g, "ChatGPT Apps")
    .replace(/\bSDKmcp\b/g, "SDK mcp")
    .replace(/\bCant\b/g, "Can't");

  next = next
    .replace(/^[■□▪▫◾◼●○•·]\s*/u, "")
    .replace(/^[1I丨]\s+(?=(?:Codex|API|Feedback|ChatGPT|OpenAI|MCP|GPT)\b)/i, "")
    .replace(/\s+/g, " ")
    .trim();

  return next;
};

const tagTokenPattern = /^[a-z0-9][a-z0-9+._-]*$/i;
const metadataCategoryPattern = /^(?:API|Codex|Feedback|Announcements|ChatGPT Apps SDK|Codex CLI)$/i;

const isForumMetadataLine = (text: string) => {
  const normalized = normalizeForumListText(text);
  if (!normalized) return false;

  const firstCommaFree = normalized.split(",")[0].trim();
  const parts = firstCommaFree.split(/\s+/);
  const category = parts.length >= 3 && `${parts[0]} ${parts[1]} ${parts[2]}`.toLowerCase() === "chatgpt apps sdk"
    ? "ChatGPT Apps SDK"
    : parts.length >= 2 && `${parts[0]} ${parts[1]}`.toLowerCase() === "codex cli"
      ? "Codex CLI"
      : parts[0];
  if (!metadataCategoryPattern.test(category)) return false;

  const rest = normalized.slice(category.length).trim();
  if (!rest) return true;
  return rest
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean)
    .every((item) => tagTokenPattern.test(item));
};

const startsLikeListNumber = (text: string) => /^\d+[.)]\s+/.test(text.trim());
const hasSentenceEnding = (text: string) => /[.!?。！？]["')\]]?$/.test(text.trim());

const shouldMergeWrappedTitle = (previous: LineItem, current: LineItem, medianHeight: number) => {
  const previousText = previous.block.text.trim();
  const currentText = current.block.text.trim();
  if (!previousText || !currentText) return false;
  if (isForumMetadataLine(previousText) || isForumMetadataLine(currentText)) return false;
  if (startsLikeListNumber(currentText)) return false;

  const verticalGap = Math.max(0, current.minY - previous.maxY);
  const leftAligned = Math.abs(current.minX - previous.minX) <= Math.max(18, medianHeight * 0.9);
  const shortContinuation = currentText.length <= 40 && currentText.split(/\s+/).length <= 4;
  const previousLooksOpen = previousText.length >= 28 && !hasSentenceEnding(previousText);

  return leftAligned && verticalGap <= Math.max(8, medianHeight * 0.75) && shortContinuation && previousLooksOpen;
};

export const applyForumListProfile = (blocks: OcrBlock[]) => {
  const items = blocks
    .map((block) => {
      const bounds = getOcrBlockBounds(block);
      return {
        block: { ...block, text: normalizeForumListText(block.text) },
        ...bounds,
        height: getBoundsHeight(bounds),
      };
    })
    .filter((item) => item.block.text)
    .sort((a, b) => a.minY - b.minY || a.minX - b.minX);

  const medianHeight = median(items.map((item) => item.height)) || 14;
  const merged: LineItem[] = [];

  for (const item of items) {
    const previous = merged[merged.length - 1];
    if (previous && shouldMergeWrappedTitle(previous, item, medianHeight)) {
      const text = normalizeForumListText(`${previous.block.text} ${item.block.text}`);
      merged[merged.length - 1] = {
        block: makeBlock([previous, item], text),
        minX: Math.min(previous.minX, item.minX),
        minY: Math.min(previous.minY, item.minY),
        maxX: Math.max(previous.maxX, item.maxX),
        maxY: Math.max(previous.maxY, item.maxY),
        height: Math.max(previous.height, item.height),
      };
    } else {
      merged.push(item);
    }
  }

  return merged.map((item) => item.block);
};
