import { buildTranslationQualityPolicy, buildTranslationSystemInstruction, hasLatinText, normalizeForCompare } from "../ocr-processing";
import type { OcrBlock, TranslatePair } from "../types/screenshot";

export type TranslationSourceLanguage = "auto" | "en";

export type TranslationRequestBlock = {
  text: string;
  confidence: number;
  box: [number, number][];
};

export type TranslationRequestPayload = {
  blocks: TranslationRequestBlock[];
  source_lang: TranslationSourceLanguage;
  target_lang: string;
  system_instruction: string;
  quality_policy: ReturnType<typeof buildTranslationQualityPolicy>;
};

export type RetryTranslationBlock = {
  block: OcrBlock;
  index: number;
};

export const isChineseTargetLanguage = (targetLang: string) => targetLang === "zh" || targetLang.startsWith("zh");

export const selectPreferredSourceLanguage = (blocks: OcrBlock[], targetLang: string): TranslationSourceLanguage => (
  isChineseTargetLanguage(targetLang) && blocks.some((block) => hasLatinText(block.text)) ? "en" : "auto"
);

export const buildTranslationRequestPayload = (
  blocks: OcrBlock[],
  sourceLang: TranslationSourceLanguage,
  targetLang: string,
): TranslationRequestPayload => ({
  blocks: blocks.map((block) => ({ text: block.text, confidence: block.confidence, box: block.box_coords })),
  source_lang: sourceLang,
  target_lang: targetLang,
  system_instruction: buildTranslationSystemInstruction(targetLang),
  quality_policy: buildTranslationQualityPolicy(targetLang),
});

export const collectUntranslatedLatinRetryBlocks = (
  blocks: OcrBlock[],
  translations: string[],
  targetLang: string,
  preferredSourceLang: TranslationSourceLanguage,
): RetryTranslationBlock[] => {
  if (preferredSourceLang === "en" || !isChineseTargetLanguage(targetLang)) return [];
  return blocks
    .map((block, index) => ({ block, index }))
    .filter(({ block, index }) => {
      const translated = translations[index] || "";
      return hasLatinText(block.text) && normalizeForCompare(translated) === normalizeForCompare(block.text);
    });
};

export const mergeRetryTranslations = (
  translations: string[],
  retryBlocks: RetryTranslationBlock[],
  retryTranslations: string[],
) => translations.map((item, index) => {
  const retryIndex = retryBlocks.findIndex((entry) => entry.index === index);
  return retryIndex >= 0 ? (retryTranslations[retryIndex] || item) : item;
});

export const normalizeTranslationResults = (blocks: OcrBlock[], translations: string[]) => (
  blocks.map((block, index) => {
    const translated = translations[index]?.trim();
    return translated || block.text;
  })
);

export const buildTranslatePairs = (blocks: OcrBlock[], translations: string[]): TranslatePair[] => {
  const normalizedTranslations = normalizeTranslationResults(blocks, translations);
  return blocks.map((block, index) => ({ o: block.text, t: normalizedTranslations[index] }));
};
