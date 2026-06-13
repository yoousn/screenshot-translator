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
  force_translate_technical_text: boolean;
  system_instruction: string;
  quality_policy: ReturnType<typeof buildTranslationQualityPolicy>;
};

export type TranslationRequirementOptions = {
  forceTranslateTechnicalText?: boolean;
};

export const shouldForceTranslateTechnicalTextForModel = (modelVersion?: string) => !modelVersion || modelVersion === "v6";

export type RetryTranslationBlock = {
  block: OcrBlock;
  index: number;
};

export type TranslationQualitySummary = {
  total: number;
  translatableCount: number;
  translatedCount: number;
  preservedCount: number;
  missingCount: number;
  untranslatedCount: number;
  badTranslationCount: number;
  untranslatedIndexes: number[];
  preservedIndexes: number[];
  badTranslationIndexes: number[];
  badTranslationReasons: Record<number, string[]>;
};

export const isChineseTargetLanguage = (targetLang: string) => targetLang === "zh" || targetLang.startsWith("zh");

const hasNonLatinTranslatableScript = (text: string) => /[\u0400-\u052f\u0600-\u06ff\u0e00-\u0e7f\u3040-\u30ff\uac00-\ud7af]/.test(text);
const latinDiacriticPattern = /[À-ÖØ-öø-ÿ]/;
const nonEnglishLatinWordPattern = /\b(?:abrir|antes|guardar|vista|previa|ouvrir|aperçu|apercu|avant|enregistrer|paramètres|parametres|fichier|fenêtre|fenetre|actualizar|cancelar|copiar|guardar|configuración|configuracion|de|des|du|del|para|por|con|sin)\b/i;

export const hasLikelyNonEnglishLatinText = (text: string) => {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (!/[A-Za-z]{2,}/.test(normalized)) return false;
  if (latinDiacriticPattern.test(normalized)) return true;
  return nonEnglishLatinWordPattern.test(normalized);
};

export const selectPreferredSourceLanguage = (blocks: OcrBlock[], targetLang: string): TranslationSourceLanguage => (
  isChineseTargetLanguage(targetLang)
  && blocks.some((block) => hasLatinText(block.text))
  && !blocks.some((block) => hasNonLatinTranslatableScript(block.text))
  && !blocks.some((block) => hasLikelyNonEnglishLatinText(block.text))
    ? "en"
    : "auto"
);

const protectedExactTerms = new Set([
  "codex",
  "openai",
  "chatgpt",
  "api",
  "apis",
  "sdk",
  "mcp",
  "gpt",
  "gpt-5",
  "vlm",
  "path",
  "windows",
  "ocr",
  "onnx",
  "rapidocr",
  "paddleocr-json",
  "ysn ocr runtime",
  "ctrl+d",
  "ctrl+q",
]);

const fileExtensionPattern = /\.(?:exe|dll|json|md|markdown|txt|onnx|yaml|yml|toml|rs|ts|tsx|js|jsx|mjs|py|ps1|bat|cmd|png|jpe?g|webp|gif|zip|7z|msi|nsi|lock|log)$/i;
const pathLikePattern = /(?:^[A-Za-z]:[\\/]|[\\/]|^\.\.?[\\/]|~[\\/])/;
const commandFlagPattern = /^-{1,2}[\w-]+(?:[=:][^\s]+)?$/;
const envAssignmentPattern = /^[A-Z_][A-Z0-9_]*=.+$/;
const commandLineMarkerPattern = /(?:&&|\|\||\s-{1,2}[\w-]+)/;
const packageLikePattern = /^(?:@[\w.-]+\/)?[\w.-]+(?:\/[\w.-]+)+$/;
const uppercaseIdentifierPattern = /^[A-Z0-9][A-Z0-9_.-]*[_./-][A-Z0-9_.-]*$/;
const translatableScriptPattern = /[A-Za-z]{2,}|[\u0400-\u052f]{2,}|[\u0600-\u06ff]{2,}|[\u0e00-\u0e7f]{2,}|[\u3040-\u30ff]{2,}|[\uac00-\ud7af]{2,}/;
const protectedTranslationTokenPattern = /\b(?:Codex|OpenAI|ChatGPT|API|APIs|SDK|MCP|GPT(?:-\d+(?:\.\d+)?(?:-Codex)?)?|VLM|ONNX|ONNXRuntime|RapidOCR|Windows|PATH)\b/g;

const hasChineseText = (text: string) => /[\u3400-\u9fff]/.test(text);

const trimTokenPunctuation = (text: string) => text.trim().replace(/^[`"'([{<]+|[`"'\])}>.,;:]+$/g, "");

const isProtectedTechnicalToken = (raw: string) => {
  const token = trimTokenPunctuation(raw);
  if (!token) return false;
  const lower = token.toLowerCase();
  if (protectedExactTerms.has(lower)) return true;
  if (/^ctrl\+[a-z0-9]$/i.test(token)) return true;
  if (envAssignmentPattern.test(token)) return true;
  if (commandFlagPattern.test(token)) return true;
  if (fileExtensionPattern.test(token)) return true;
  if (pathLikePattern.test(token) && /^[\w .:@~+\\/-]+$/.test(token)) return true;
  if (packageLikePattern.test(token)) return true;
  if (uppercaseIdentifierPattern.test(token) && token === token.toUpperCase()) return true;
  return false;
};

export const isLikelyProtectedTechnicalText = (text: string) => {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (!normalized || hasChineseText(normalized)) return false;
  if (isProtectedTechnicalToken(normalized)) return true;

  const tokens = normalized.split(/\s+/).filter(Boolean);
  if (tokens.length <= 1) return false;

  const hasPathOrFileMarker = pathLikePattern.test(normalized) || fileExtensionPattern.test(normalized);
  if (hasPathOrFileMarker && /^[\w .:@~+\\/-]+$/.test(normalized)) return true;
  const hasCommandLineMarker = envAssignmentPattern.test(tokens[0] || "") || commandLineMarkerPattern.test(normalized);
  if (hasCommandLineMarker && /^[\w .:@~+\\/\-=&|]+$/.test(normalized)) return true;

  return tokens.every(isProtectedTechnicalToken);
};

export const shouldRequireTranslation = (
  text: string,
  targetLang: string,
  options: TranslationRequirementOptions = {},
) => {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (!normalized) return false;
  if (isLikelyProtectedTechnicalText(normalized) && !options.forceTranslateTechnicalText) return false;
  if (isChineseTargetLanguage(targetLang) && hasChineseText(normalized) && !hasLatinText(normalized)) return false;
  return translatableScriptPattern.test(normalized);
};

export const buildTranslationRequestPayload = (
  blocks: OcrBlock[],
  sourceLang: TranslationSourceLanguage,
  targetLang: string,
  options: TranslationRequirementOptions = {},
): TranslationRequestPayload => ({
  blocks: blocks.map((block) => ({ text: block.text, confidence: block.confidence, box: block.box_coords })),
  source_lang: sourceLang,
  target_lang: targetLang,
  force_translate_technical_text: Boolean(options.forceTranslateTechnicalText),
  system_instruction: buildTranslationSystemInstruction(targetLang, blocks.map((block) => block.text)),
  quality_policy: buildTranslationQualityPolicy(targetLang),
});

export const collectUntranslatedLatinRetryBlocks = (
  blocks: OcrBlock[],
  translations: string[],
  targetLang: string,
  preferredSourceLang: TranslationSourceLanguage,
  options: TranslationRequirementOptions = {},
): RetryTranslationBlock[] => {
  if (preferredSourceLang === "en" || !isChineseTargetLanguage(targetLang)) return [];
  return blocks
    .map((block, index) => ({ block, index }))
    .filter(({ block, index }) => {
      const translated = translations[index] || "";
      if (isLikelyProtectedTechnicalText(block.text)) return false;
      return shouldRequireTranslation(block.text, targetLang, options)
        && hasLatinText(block.text)
        && normalizeForCompare(translated) === normalizeForCompare(block.text);
    });
};

export const mergeRetryTranslations = (
  translations: string[],
  retryBlocks: RetryTranslationBlock[],
  retryTranslations: string[],
) => {
  const retryByOriginalIndex = new Map<number, string>();
  retryBlocks.forEach((entry, retryIndex) => {
    const translated = retryTranslations[retryIndex]?.trim();
    if (translated) {
      retryByOriginalIndex.set(entry.index, translated);
    }
  });
  return translations.map((item, index) => retryByOriginalIndex.get(index) || item);
};

export const repairKnownBadTranslationTerms = (sourceText: string, translatedText: string) => {
  const source = sourceText.replace(/\s+/g, " ").trim();
  let next = translatedText.replace(/\s+/g, " ").trim();
  if (!next) return next;

  if (/\bCodex\b/i.test(source)) {
    next = next.replace(/法典|科德克斯/g, "Codex");
  }
  if (/\bOpenAI\b/i.test(source)) {
    next = next.replace(/开放人工智能|开放AI/g, "OpenAI");
  }
  if (/\bChatGPT\b/i.test(source)) {
    next = next.replace(/聊天GPT|聊天机器人GPT/g, "ChatGPT");
  }
  if (/\bticket\b/i.test(source)) {
    next = next.replace(/(?:支持)?票据|门票|票/g, "工单");
  }
  if (/\bfixture(?:s)?\b/i.test(source)) {
    next = next.replace(/固定装置|夹具/g, "固定测试样例");
  }
  if (/\bVLM\s+fallback\b/i.test(source)) {
    next = next.replace(/VLM\s*后备|视觉语言模型后备|VLM\s*fallback/gi, "VLM 兜底识别");
  }

  return next;
};

const collectProtectedTranslationTokens = (text: string) => (
  Array.from(new Set(text.match(protectedTranslationTokenPattern) || []))
);

const getBadTranslationReasons = (sourceText: string, translatedText: string) => {
  const reasons: string[] = [];
  const source = sourceText.replace(/\s+/g, " ").trim();
  const translated = translatedText.replace(/\s+/g, " ").trim();
  if (!source || !translated) return reasons;

  for (const token of collectProtectedTranslationTokens(source)) {
    if (!translated.includes(token)) reasons.push(`missing-protected-token:${token}`);
  }
  if (/\bCodex\b/i.test(source) && /法典|科德克斯/.test(translated)) reasons.push("bad-term:codex");
  if (/\bticket\b/i.test(source) && /(?:支持)?票据|门票|票/.test(translated)) reasons.push("bad-term:ticket");
  if (/\bfixture(?:s)?\b/i.test(source) && /固定装置|夹具/.test(translated)) reasons.push("bad-term:fixture");
  if (/\bVLM\s+fallback\b/i.test(source) && /VLM\s*后备|视觉语言模型后备/i.test(translated)) reasons.push("bad-term:vlm-fallback");
  return reasons;
};

export const normalizeTranslationResults = (blocks: OcrBlock[], translations: string[]) => (
  blocks.map((block, index) => {
    const translated = repairKnownBadTranslationTerms(block.text, translations[index]?.trim() || "");
    return translated || block.text;
  })
);

export const evaluateTranslationQuality = (
  blocks: OcrBlock[],
  rawTranslations: string[],
  normalizedTranslations: string[],
  targetLang: string,
  options: TranslationRequirementOptions = {},
): TranslationQualitySummary => {
  const summary: TranslationQualitySummary = {
    total: blocks.length,
    translatableCount: 0,
    translatedCount: 0,
    preservedCount: 0,
    missingCount: 0,
    untranslatedCount: 0,
    badTranslationCount: 0,
    untranslatedIndexes: [],
    preservedIndexes: [],
    badTranslationIndexes: [],
    badTranslationReasons: {},
  };

  blocks.forEach((block, index) => {
    const rawTranslated = rawTranslations[index]?.trim() || "";
    const normalizedTranslated = normalizedTranslations[index] || block.text;
    const requiresTranslation = shouldRequireTranslation(block.text, targetLang, options);
    const forcedTechnicalText = Boolean(options.forceTranslateTechnicalText)
      && isLikelyProtectedTechnicalText(block.text);
    const unchanged = normalizeForCompare(normalizedTranslated) === normalizeForCompare(block.text);

    if (!rawTranslated) summary.missingCount += 1;
    const badReasons = rawTranslated ? getBadTranslationReasons(block.text, normalizedTranslated) : [];
    if (badReasons.length > 0) {
      summary.badTranslationCount += 1;
      summary.badTranslationIndexes.push(index);
      summary.badTranslationReasons[index] = badReasons;
    }
    if (requiresTranslation) {
      summary.translatableCount += 1;
      if (forcedTechnicalText && rawTranslated) {
        summary.translatedCount += 1;
      } else if (unchanged || !rawTranslated) {
        summary.untranslatedCount += 1;
        summary.untranslatedIndexes.push(index);
      } else {
        summary.translatedCount += 1;
      }
    } else if (unchanged) {
      summary.preservedCount += 1;
      summary.preservedIndexes.push(index);
    }
  });

  return summary;
};

export const validateAndNormalizeTranslationResults = (
  blocks: OcrBlock[],
  translations: string[],
  targetLang: string,
  providerError?: string,
  options: TranslationRequirementOptions = {},
) => {
  const normalizedTranslations = normalizeTranslationResults(blocks, translations);
  const quality = evaluateTranslationQuality(blocks, translations, normalizedTranslations, targetLang, options);
  if (quality.translatableCount > 0 && quality.untranslatedCount === quality.translatableCount) {
    const errorSuffix = providerError ? `（服务端原因：${providerError}）` : "";
    throw new Error(`翻译服务没有返回可用译文：${quality.translatableCount} 行可翻译文本仍是原文或为空。请检查翻译服务地址、令牌和当前翻译通道后重试。${errorSuffix}`);
  }
  return { translations: normalizedTranslations, quality };
};

export const buildTranslatePairs = (
  blocks: OcrBlock[],
  translations: string[],
  targetLang = "zh",
  options: TranslationRequirementOptions = {},
): TranslatePair[] => {
  const normalizedTranslations = normalizeTranslationResults(blocks, translations);
  return blocks.map((block, index) => {
    const translated = normalizedTranslations[index];
    const unchanged = normalizeForCompare(translated) === normalizeForCompare(block.text);
    const forcedTechnicalText = Boolean(options.forceTranslateTechnicalText)
      && isLikelyProtectedTechnicalText(block.text);
    const status = unchanged
      ? (forcedTechnicalText
          ? "translated"
          : (shouldRequireTranslation(block.text, targetLang, options) ? "untranslated" : "preserved"))
      : "translated";
    return { o: block.text, t: translated, status };
  });
};
