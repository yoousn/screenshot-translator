import type { OcrBlock } from "../types/screenshot";
import { getBlockBounds } from "./geometry";

type Bounds = {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
};

export type DistributedRenderTranslations = {
  blocks: OcrBlock[];
  translations: string[];
};

const normalizeOneLine = (text: string) => text.replace(/\s+/g, " ").trim();

const getBounds = (block: OcrBlock): Bounds => getBlockBounds(block);

const getCenterY = (bounds: Bounds) => (bounds.minY + bounds.maxY) / 2;

const horizontalOverlapRatio = (outer: Bounds, inner: Bounds) => {
  const overlap = Math.max(0, Math.min(outer.maxX, inner.maxX) - Math.max(outer.minX, inner.minX));
  return overlap / Math.max(1, inner.maxX - inner.minX);
};

const isRenderBlockInsideTranslationBlock = (translationBounds: Bounds, renderBounds: Bounds) => {
  const centerY = getCenterY(renderBounds);
  const verticalSlack = Math.max(4, (renderBounds.maxY - renderBounds.minY) * 0.45);
  return (
    centerY >= translationBounds.minY - verticalSlack
    && centerY <= translationBounds.maxY + verticalSlack
    && horizontalOverlapRatio(translationBounds, renderBounds) >= 0.45
  );
};

const isCjkLike = (text: string) => /[\u3040-\u30ff\u3400-\u9fff\uf900-\ufaff\uac00-\ud7af]/u.test(text);

const findCjkSplitIndex = (text: string, target: number) => {
  const min = Math.max(1, target - 5);
  const max = Math.min(text.length - 1, target + 5);
  for (let index = min; index <= max; index += 1) {
    if (/[，。！？；：、,.!?;:]/u.test(text[index - 1])) return index;
  }
  return Math.max(1, Math.min(text.length - 1, target));
};

const splitCjkTextByWeights = (text: string, weights: number[]) => {
  const output: string[] = [];
  let rest = normalizeOneLine(text);
  let restWeight = weights.reduce((sum, weight) => sum + Math.max(1, weight), 0);

  for (let index = 0; index < weights.length - 1; index += 1) {
    const weight = Math.max(1, weights[index]);
    const target = Math.round((rest.length * weight) / Math.max(1, restWeight));
    const splitIndex = findCjkSplitIndex(rest, target);
    output.push(rest.slice(0, splitIndex).trim());
    rest = rest.slice(splitIndex).trim();
    restWeight -= weight;
  }

  output.push(rest);
  return output;
};

const splitWordTextByWeights = (text: string, weights: number[]) => {
  const words = normalizeOneLine(text).split(/\s+/).filter(Boolean);
  if (words.length <= weights.length) {
    return weights.map((_, index) => words[index] || "");
  }

  const output: string[] = [];
  let cursor = 0;
  let restWeight = weights.reduce((sum, weight) => sum + Math.max(1, weight), 0);

  for (let index = 0; index < weights.length - 1; index += 1) {
    const weight = Math.max(1, weights[index]);
    const remainingSlots = weights.length - index - 1;
    const remainingWords = words.length - cursor;
    const take = Math.max(1, Math.min(remainingWords - remainingSlots, Math.round((remainingWords * weight) / Math.max(1, restWeight))));
    output.push(words.slice(cursor, cursor + take).join(" "));
    cursor += take;
    restWeight -= weight;
  }

  output.push(words.slice(cursor).join(" "));
  return output;
};

const splitTranslationForRenderBlocks = (translation: string, renderBlocks: OcrBlock[]) => {
  const normalized = normalizeOneLine(translation);
  if (renderBlocks.length <= 1) return [normalized];

  const explicitLines = translation
    .split(/\r?\n/)
    .map((line) => normalizeOneLine(line))
    .filter(Boolean);
  if (explicitLines.length === renderBlocks.length) return explicitLines;

  const weights = renderBlocks.map((block) => Math.max(1, normalizeOneLine(block.text).length));
  return isCjkLike(normalized)
    ? splitCjkTextByWeights(normalized, weights)
    : splitWordTextByWeights(normalized, weights);
};

export const distributeTranslationsForRender = (
  translationBlocks: OcrBlock[],
  translations: string[],
  renderBlocks: OcrBlock[],
): DistributedRenderTranslations => {
  if (renderBlocks.length === 0 || translationBlocks.length === 0) {
    return { blocks: translationBlocks, translations };
  }
  if (translationBlocks.length === renderBlocks.length) {
    return { blocks: renderBlocks, translations };
  }

  const outputBlocks: OcrBlock[] = [];
  const outputTranslations: string[] = [];
  const usedRenderIndexes = new Set<number>();

  translationBlocks.forEach((translationBlock, index) => {
    const translationBounds = getBounds(translationBlock);
    const matchingIndexes = renderBlocks
      .map((renderBlock, renderIndex) => ({ renderBlock, renderIndex, renderBounds: getBounds(renderBlock) }))
      .filter(({ renderIndex, renderBounds }) => (
        !usedRenderIndexes.has(renderIndex)
        && isRenderBlockInsideTranslationBlock(translationBounds, renderBounds)
      ))
      .sort((left, right) => left.renderBounds.minY - right.renderBounds.minY || left.renderBounds.minX - right.renderBounds.minX);

    if (matchingIndexes.length <= 1) {
      outputBlocks.push(translationBlock);
      outputTranslations.push(translations[index] || translationBlock.text);
      return;
    }

    const splitTranslations = splitTranslationForRenderBlocks(translations[index] || translationBlock.text, matchingIndexes.map((item) => item.renderBlock));
    matchingIndexes.forEach(({ renderBlock, renderIndex }, splitIndex) => {
      usedRenderIndexes.add(renderIndex);
      outputBlocks.push(renderBlock);
      outputTranslations.push(splitTranslations[splitIndex] || "");
    });
  });

  return { blocks: outputBlocks, translations: outputTranslations };
};
