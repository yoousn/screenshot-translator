import type { OcrBlock } from "../types/screenshot";
import { filterUsefulOcrBlocks } from "./blockFilters";
import { applyForumListProfile } from "./forumListProfile";
import { buildVirtualOcrLines } from "./lineGrouping";
import { planOcrRoutes, type OcrRoutePlan } from "./ocrRoutePlan";
import { applyParagraphProfile } from "./paragraphProfile";

export type OcrNormalizationReport = {
  rawCount: number;
  usefulCount: number;
  virtualLineCount: number;
  droppedCount: number;
  blocks: OcrBlock[];
  renderBlocks: OcrBlock[];
  routePlan: OcrRoutePlan | null;
  text: string;
};

export const buildOcrNormalizationReport = async (rawBlocks: OcrBlock[]): Promise<OcrNormalizationReport> => {
  const usefulBlocks = filterUsefulOcrBlocks(rawBlocks || []);
  const renderBlocks = applyForumListProfile(buildVirtualOcrLines(usefulBlocks));
  const blocks = applyParagraphProfile(renderBlocks);
  const routePlan = await planOcrRoutes(blocks);
  return {
    rawCount: rawBlocks?.length || 0,
    usefulCount: usefulBlocks.length,
    virtualLineCount: renderBlocks.length,
    droppedCount: Math.max(0, (rawBlocks?.length || 0) - usefulBlocks.length),
    blocks,
    renderBlocks,
    routePlan,
    text: blocks.map((item) => item.text).filter(Boolean).join("\n"),
  };
};
