import type { OcrBlock, Rect } from "../types/screenshot";

export type TextSourceSelectionElement = {
  text?: string;
  x?: number;
  y?: number;
  w?: number;
  h?: number;
};

export type TextSourceSelectionScreen = {
  x?: number;
  y?: number;
  w?: number;
  h?: number;
};

export type TextSourceSelectionCandidate = {
  block: OcrBlock;
  text: string;
  area: number;
  localLeft: number;
  localTop: number;
  localRight: number;
  localBottom: number;
  elementCoverage: number;
  selectionCoverage: number;
};

export type TextSourceSelectionResult = {
  blocks: OcrBlock[];
  matchedRawCount: number;
  rejectedRawCount: number;
  rejectedAggregateCount: number;
  maxElementCoverage: number;
  maxSelectionCoverage: number;
};

const clampNumber = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

const normalizeTextSourceText = (text: string) => text.replace(/\s+/g, " ").trim();

const containsText = (outer: string, inner: string) => (
  outer.toLocaleLowerCase().includes(inner.toLocaleLowerCase())
);

const isMostlyInside = (outer: TextSourceSelectionCandidate, inner: TextSourceSelectionCandidate) => (
  inner.localLeft >= outer.localLeft - 2
  && inner.localTop >= outer.localTop - 2
  && inner.localRight <= outer.localRight + 2
  && inner.localBottom <= outer.localBottom + 2
);

const isLikelyAggregateText = (text: string, elementArea: number, selectionArea: number) => {
  if (text.length >= 96) return true;
  const words = text.split(/\s+/).filter(Boolean);
  if (words.length >= 16) return true;
  return text.length >= 56 && elementArea >= selectionArea * 0.45;
};

const removeAggregateContainers = (candidates: TextSourceSelectionCandidate[]) => {
  if (candidates.length <= 1) return candidates;
  return candidates.filter((candidate) => {
    const smallerChildren = candidates.filter((other) => (
      other !== candidate
      && other.area < candidate.area * 0.72
      && isMostlyInside(candidate, other)
      && containsText(candidate.text, other.text)
    ));
    if (smallerChildren.length >= 2) return false;
    return !smallerChildren.some((child) => (
      candidate.text.length >= child.text.length + 8
      && candidate.area >= child.area * 1.8
    ));
  });
};

export const buildTextSourceBlocksForPhysicalSelection = (
  elements: TextSourceSelectionElement[] | undefined,
  screen: TextSourceSelectionScreen | undefined,
  physicalSelection: Rect,
): TextSourceSelectionResult => {
  const empty: TextSourceSelectionResult = {
    blocks: [],
    matchedRawCount: 0,
    rejectedRawCount: 0,
    rejectedAggregateCount: 0,
    maxElementCoverage: 0,
    maxSelectionCoverage: 0,
  };
  if (!screen || !elements?.length || physicalSelection.w < 6 || physicalSelection.h < 6) return empty;

  const screenX = Math.round(Number(screen.x ?? 0));
  const screenY = Math.round(Number(screen.y ?? 0));
  const selectionLeft = screenX + physicalSelection.x;
  const selectionTop = screenY + physicalSelection.y;
  const selectionRight = selectionLeft + physicalSelection.w;
  const selectionBottom = selectionTop + physicalSelection.h;
  const selectionArea = Math.max(1, physicalSelection.w * physicalSelection.h);
  const candidates: TextSourceSelectionCandidate[] = [];
  let rejectedRawCount = 0;
  let rejectedAggregateCount = 0;
  let maxElementCoverage = 0;
  let maxSelectionCoverage = 0;

  for (const element of elements) {
    const text = normalizeTextSourceText(String(element.text || ""));
    const x = Math.round(Number(element.x ?? 0));
    const y = Math.round(Number(element.y ?? 0));
    const w = Math.round(Number(element.w ?? 0));
    const h = Math.round(Number(element.h ?? 0));
    if (!text || text.length < 2 || w <= 0 || h <= 0) {
      rejectedRawCount += 1;
      continue;
    }

    const right = x + w;
    const bottom = y + h;
    const intersectionLeft = Math.max(x, selectionLeft);
    const intersectionTop = Math.max(y, selectionTop);
    const intersectionRight = Math.min(right, selectionRight);
    const intersectionBottom = Math.min(bottom, selectionBottom);
    const intersectionW = Math.max(0, intersectionRight - intersectionLeft);
    const intersectionH = Math.max(0, intersectionBottom - intersectionTop);
    const intersectionArea = intersectionW * intersectionH;
    if (intersectionArea <= 0) {
      rejectedRawCount += 1;
      continue;
    }

    const elementArea = Math.max(1, w * h);
    const elementCoverage = intersectionArea / elementArea;
    const selectionCoverage = intersectionArea / selectionArea;
    maxElementCoverage = Math.max(maxElementCoverage, elementCoverage);
    maxSelectionCoverage = Math.max(maxSelectionCoverage, selectionCoverage);

    const oversized = w > physicalSelection.w * 1.28 || h > physicalSelection.h * 1.28;
    const aggregate = isLikelyAggregateText(text, elementArea, selectionArea);
    if (elementCoverage < 0.55 || (oversized && elementCoverage < 0.86)) {
      rejectedRawCount += 1;
      continue;
    }
    if (aggregate && (selectionCoverage > 0.2 || elementArea >= selectionArea * 0.45)) {
      rejectedAggregateCount += 1;
      continue;
    }

    const localLeft = clampNumber(Math.round(intersectionLeft - selectionLeft), 0, Math.max(0, physicalSelection.w - 1));
    const localTop = clampNumber(Math.round(intersectionTop - selectionTop), 0, Math.max(0, physicalSelection.h - 1));
    const localRight = clampNumber(Math.round(intersectionRight - selectionLeft), localLeft + 1, physicalSelection.w);
    const localBottom = clampNumber(Math.round(intersectionBottom - selectionTop), localTop + 1, physicalSelection.h);
    candidates.push({
      text,
      area: Math.max(1, (localRight - localLeft) * (localBottom - localTop)),
      localLeft,
      localTop,
      localRight,
      localBottom,
      elementCoverage,
      selectionCoverage,
      block: {
        text,
        confidence: 0.995,
        box_coords: [
          [localLeft, localTop],
          [localRight, localTop],
          [localRight, localBottom],
          [localLeft, localBottom],
        ],
      },
    });
  }

  const seen = new Set<string>();
  const blocks = removeAggregateContainers(candidates)
    .filter((candidate) => {
      const key = `${candidate.text}|${candidate.localLeft}|${candidate.localTop}|${candidate.localRight}|${candidate.localBottom}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .sort((left, right) => (left.localTop - right.localTop) || (left.localLeft - right.localLeft))
    .slice(0, 120)
    .map((candidate) => candidate.block);

  return {
    blocks,
    matchedRawCount: candidates.length,
    rejectedRawCount,
    rejectedAggregateCount,
    maxElementCoverage,
    maxSelectionCoverage,
  };
};
