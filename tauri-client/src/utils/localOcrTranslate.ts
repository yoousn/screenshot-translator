import { invoke } from "@tauri-apps/api/core";
import { buildOcrNormalizationReport } from "../ocr-processing";
import type { OcrBlock, TranslatePair } from "../types/screenshot";
import {
  buildTranslatePairs,
  buildTranslationRequestPayload,
  collectUntranslatedLatinRetryBlocks,
  mergeRetryTranslations,
  normalizeTranslationResults,
  selectPreferredSourceLanguage,
  type TranslationSourceLanguage,
} from "./ocrTranslationRequest";
import { DEFAULT_TRANSLATION_SERVICE_URL } from "./translationService";
import { renderTranslatedBlocks } from "./translatedBlocks";

type LocalTranslateConfig = {
  serverUrl?: string;
  clientToken?: string;
  targetLang?: string;
  channel?: string;
  localOcrExecutablePath?: string;
  localOcrTimeoutMs?: number;
};

const requestTextTranslations = async (serverUrl: string, token: string, blocks: OcrBlock[], sourceLang: TranslationSourceLanguage, targetLang: string) => {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/translate_text`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "x-api-key": token },
    body: JSON.stringify(buildTranslationRequestPayload(blocks, sourceLang, targetLang)),
  });
  if (!response.ok) throw new Error(`Text translation API failed: ${response.status}`);
  const transData = await response.json();
  if (transData.status !== "success") throw new Error(transData.error || "Text translation failed");
  return transData as { translations?: string[]; channel?: string };
};

const retryUntranslatedLatinBlocks = async (
  serverUrl: string,
  token: string,
  blocks: OcrBlock[],
  translations: string[],
  targetLang: string,
  preferredSourceLang: TranslationSourceLanguage,
) => {
  const retryBlocks = collectUntranslatedLatinRetryBlocks(blocks, translations, targetLang, preferredSourceLang);

  if (retryBlocks.length === 0) {
    return translations;
  }

  const retryData = await requestTextTranslations(serverUrl, token, retryBlocks.map((item) => item.block), "en", targetLang);
  const retryTranslations = retryData.translations || [];
  return mergeRetryTranslations(translations, retryBlocks, retryTranslations);
};

export const translateWithLocalOcr = async (base64: string, config: LocalTranslateConfig) => {
  const serverUrl = config.serverUrl || DEFAULT_TRANSLATION_SERVICE_URL;
  const token = config.clientToken || "";
  const targetLang = config.targetLang || "zh";

  const rawBlocks: OcrBlock[] = await invoke("run_local_ocr", {
    imageBase64: base64,
    executablePath: config.localOcrExecutablePath || null,
    timeoutMs: config.localOcrTimeoutMs || 15000,
  });
  const normalization = await buildOcrNormalizationReport(rawBlocks || []);
  const ocrBlocks = normalization.blocks;
  const routePlan = normalization.routePlan;
  if (!ocrBlocks.length) throw new Error("Local OCR did not recognize text");

  const preferredSourceLang = selectPreferredSourceLanguage(ocrBlocks, targetLang);
  const transData = await requestTextTranslations(serverUrl, token, ocrBlocks, preferredSourceLang, targetLang);
  const translations = await retryUntranslatedLatinBlocks(
    serverUrl,
    token,
    ocrBlocks,
    transData.translations || [],
    targetLang,
    preferredSourceLang,
  );
  const normalizedTranslations = normalizeTranslationResults(ocrBlocks, translations);

  const resultBase64 = await renderTranslatedBlocks(base64, ocrBlocks, normalizedTranslations);
  const pairs: TranslatePair[] = buildTranslatePairs(ocrBlocks, normalizedTranslations);
  return { resultBase64, pairs, usedChannel: transData.channel || config.channel || config.targetLang || "auto", blocksCount: ocrBlocks.length, routePlan, normalization };
};
