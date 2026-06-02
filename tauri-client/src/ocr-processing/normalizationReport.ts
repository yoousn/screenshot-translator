import type { OcrBlock } from "../types/screenshot";
import { filterUsefulOcrBlocks } from "./blockFilters";
import { buildVirtualOcrLines } from "./lineGrouping";
import { planOcrRoutes, type OcrRoutePlan } from "./ocrRoutePlan";

export type OcrNormalizationReport = {
  rawCount: number;
  usefulCount: number;
  virtualLineCount: number;
  droppedCount: number;
  blocks: OcrBlock[];
  routePlan: OcrRoutePlan | null;
  text: string;
};

export const buildOcrNormalizationReport = async (rawBlocks: OcrBlock[]): Promise<OcrNormalizationReport> => {
  const usefulBlocks = filterUsefulOcrBlocks(rawBlocks || []);
  const blocks = buildVirtualOcrLines(usefulBlocks);
  const routePlan = await planOcrRoutes(blocks);
  return {
    rawCount: rawBlocks?.length || 0,
    usefulCount: usefulBlocks.length,
    virtualLineCount: blocks.length,
    droppedCount: Math.max(0, (rawBlocks?.length || 0) - usefulBlocks.length),
    blocks,
    routePlan,
    text: blocks.map((item) => item.text).filter(Boolean).join("\n"),
  };
};
