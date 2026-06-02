import type { OcrBlock } from "../types/screenshot";
import type { RenderBlock } from "./types";
import { getBlockBounds, isLikelyRtlText } from "./geometry";

const normalizeRenderText = (text: string) => (
  text
    .replace(/[ \t\f\v]+/g, " ")
    .replace(/[ \t\f\v]*\r?\n[ \t\f\v]*/g, "\n")
    .trim()
);

export const buildRenderBlocks = (blocks: OcrBlock[], translations: string[]): RenderBlock[] => (
  blocks
    .map((block, index) => ({ block, translation: translations[index] || block.text, bounds: getBlockBounds(block) }))
    .filter((item) => item.block.box_coords.length >= 4)
    .map(({ block, translation, bounds }) => {
      const text = normalizeRenderText(block.text);
      const translated = normalizeRenderText(translation || block.text);
      return {
        text,
        translated,
        minX: bounds.minX,
        minY: bounds.minY,
        maxX: bounds.maxX,
        maxY: bounds.maxY,
        direction: isLikelyRtlText(translated || text) ? "rtl" : "ltr",
      } satisfies RenderBlock;
    })
);
