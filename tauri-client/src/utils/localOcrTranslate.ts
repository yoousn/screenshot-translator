import { invoke } from "@tauri-apps/api/core";
import { buildOcrNormalizationReport } from "../ocr-processing";
import type { OcrBlock, TranslatePair } from "../types/screenshot";
import {
  buildTranslatePairs,
  buildTranslationRequestPayload,
  collectUntranslatedLatinRetryBlocks,
  mergeRetryTranslations,
  selectPreferredSourceLanguage,
  validateAndNormalizeTranslationResults,
  type TranslationSourceLanguage,
} from "./ocrTranslationRequest";
import { DEFAULT_TRANSLATION_SERVICE_URL } from "./translationService";
import {
  createTranslationMemoryStats,
  lookupLocalTranslation,
  storeTranslationMemory,
} from "./translationMemory";
import { renderTranslatedBlocks } from "./translatedBlocks";

type LocalTranslateConfig = {
  serverUrl?: string;
  lanServerUrl?: string;
  preferLanServer?: boolean;
  clientToken?: string;
  targetLang?: string;
  channel?: string;
  localOcrTimeoutMs?: number;
  translationTimeoutMs?: number;
};


const normalizeLocalTranslateError = (error: unknown) => {
  const raw = error instanceof Error ? error.message : String(error || "");
  const hasNoText = /\u672a\u8bc6\u522b\u5230\u6587\u5b57|did not recognize text|recognized no text|no text/i.test(raw);
  const hasTimeout = /timed out|timeout|\u8d85\u65f6/i.test(raw);
  const hasModelFileIssue = /missing|\u7f3a\u5931|\u6821\u9a8c\u5931\u8d25|manifest|model|\u6a21\u578b/i.test(raw);

  if (hasNoText) {
    return "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u672a\u8bc6\u522b\u5230\u6587\u5b57\u3002\u8bf7\u91cd\u65b0\u6846\u9009\u66f4\u6e05\u6670\u3001\u66f4\u5b8c\u6574\u7684\u6587\u5b57\u533a\u57df\u3002";
  }
  if (hasTimeout) {
    return "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u5904\u7406\u8d85\u65f6\u3002\u8bf7\u5148\u6846\u9009\u66f4\u5c0f\u7684\u6587\u5b57\u533a\u57df\uff1b\u6211\u4f1a\u7ee7\u7eed\u4f18\u5316\u5927\u5c4f\u6781\u901f\u8bc6\u522b\u3002";
  }
  if (hasModelFileIssue) {
    return "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6a21\u578b\u6587\u4ef6\u5f02\u5e38\u3002\u8bf7\u91cd\u65b0\u8fd0\u884c\u9879\u76ee\u6839\u76ee\u5f55\u7684\u6a21\u578b\u5b89\u88c5\u811a\u672c\u540e\u518d\u8bd5\u3002";
  }
  return raw
    .replace(/YSN OCR Runtime/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
    .replace(/PP-OCRv5\s*ONNX\s*OCR/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
    .replace(/PP-OCRv5/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
    .replace(/ONNX/gi, "\u672c\u5730\u6a21\u578b")
    .trim() || "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6682\u4e0d\u53ef\u7528\uff0c\u8bf7\u91cd\u65b0\u6846\u9009\u6587\u5b57\u533a\u57df\u540e\u518d\u8bd5\u3002";
};

const normalizeTranslationServerUrl = (serverUrl: string) => {
  const trimmed = serverUrl.trim();
  const urlWithProtocol = /^[a-z][a-z0-9+.-]*:/i.test(trimmed) ? trimmed : `https://${trimmed}`;
  const parsed = new URL(urlWithProtocol);
  if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
    throw new Error("\u7ffb\u8bd1\u670d\u52a1\u5730\u5740\u5fc5\u987b\u662f http \u6216 https URL");
  }
  parsed.pathname = parsed.pathname.replace(/\/$/, "");
  parsed.search = "";
  parsed.hash = "";
  return parsed.toString().replace(/\/$/, "");
};

type TranslationServiceTimings = {
  total_ms?: number;
  provider_ms?: number;
  cache_hits?: number;
  provider_misses?: number;
  request_duplicates?: number;
  preserved_hits?: number;
  blocks?: number;
};

type TranslationServiceResponse = {
  translations?: string[];
  channel?: string;
  serverUrl?: string;
  cache_hits?: number;
  timings?: TranslationServiceTimings;
};

const buildTranslationServerCandidates = (config: LocalTranslateConfig) => {
  const remoteUrl = config.serverUrl || DEFAULT_TRANSLATION_SERVICE_URL;
  const candidates = [
    ...(config.preferLanServer && config.lanServerUrl ? [config.lanServerUrl] : []),
    remoteUrl,
  ];
  return Array.from(new Set(candidates.map((item) => item.trim()).filter(Boolean)));
};

const requestTextTranslations = async (serverUrl: string, token: string, blocks: OcrBlock[], sourceLang: TranslationSourceLanguage, targetLang: string, timeoutMs: number) => {
  const normalizedServerUrl = normalizeTranslationServerUrl(serverUrl);
  const endpoint = `${normalizedServerUrl}/api/translate_text`;
  const controller = new AbortController();
  const timeoutId = window.setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json", "x-api-key": token },
      body: JSON.stringify(buildTranslationRequestPayload(blocks, sourceLang, targetLang)),
      signal: controller.signal,
    });
    if (!response.ok) throw new Error(`Text translation API failed: ${response.status}`);
    const transData = await response.json();
    if (transData.status !== "success") throw new Error(transData.error || "Text translation failed");
    return { ...(transData as TranslationServiceResponse), serverUrl: normalizedServerUrl };
  } catch (error: any) {
    if (error?.name === "AbortError") {
      throw new Error(`\u6587\u672c\u7ffb\u8bd1\u670d\u52a1\u8d85\u65f6\uff08${Math.round(timeoutMs / 1000)} \u79d2\uff09`);
    }
    throw error;
  } finally {
    window.clearTimeout(timeoutId);
  }
};

const retryUntranslatedLatinBlocks = async (
  serverUrls: string[],
  token: string,
  blocks: OcrBlock[],
  translations: string[],
  targetLang: string,
  preferredSourceLang: TranslationSourceLanguage,
  timeoutMs: number,
) => {
  const retryBlocks = collectUntranslatedLatinRetryBlocks(blocks, translations, targetLang, preferredSourceLang);

  if (retryBlocks.length === 0) {
    return translations;
  }

  const retryData = await requestTextTranslationsWithFallback(serverUrls, token, retryBlocks.map((item) => item.block), "en", targetLang, timeoutMs);
  const retryTranslations = retryData.translations || [];
  return mergeRetryTranslations(translations, retryBlocks, retryTranslations);
};

const normalizeRequestTextKey = (text: string) => text.replace(/\s+/g, " ").trim();

const dedupeTranslationRequestItems = (items: { block: OcrBlock; index: number }[]) => {
  const groups = new Map<string, { block: OcrBlock; indexes: number[] }>();
  for (const item of items) {
    const key = normalizeRequestTextKey(item.block.text);
    const existing = groups.get(key);
    if (existing) {
      existing.indexes.push(item.index);
    } else {
      groups.set(key, { block: item.block, indexes: [item.index] });
    }
  }
  return Array.from(groups.values());
};

export const translateWithLocalOcr = async (base64: string, config: LocalTranslateConfig) => {
  const serverUrls = buildTranslationServerCandidates(config);
  const token = config.clientToken || "";
  const targetLang = config.targetLang || "zh";
  const translationTimeoutMs = config.translationTimeoutMs || 20000;

  let rawBlocks: OcrBlock[];
  try {
    rawBlocks = await invoke("run_local_ocr", {
      imageBase64: base64,
      executablePath: null,
      timeoutMs: config.localOcrTimeoutMs || 15000,
    });
  } catch (error) {
    throw new Error(normalizeLocalTranslateError(error));
  }
  const normalization = await buildOcrNormalizationReport(rawBlocks || []);
  const ocrBlocks = normalization.blocks;
  const routePlan = normalization.routePlan;
  if (!ocrBlocks.length) throw new Error("\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u672a\u8bc6\u522b\u5230\u6587\u5b57\u3002\u8bf7\u91cd\u65b0\u6846\u9009\u66f4\u6e05\u6670\u3001\u66f4\u5b8c\u6574\u7684\u6587\u5b57\u533a\u57df\u3002");

  const preferredSourceLang = selectPreferredSourceLanguage(ocrBlocks, targetLang);
  const translationMemoryStats = createTranslationMemoryStats();
  const translations = new Array<string>(ocrBlocks.length).fill("");
  const requestItems: { block: OcrBlock; index: number }[] = [];

  ocrBlocks.forEach((block, index) => {
    const localHit = lookupLocalTranslation(block, preferredSourceLang, targetLang, config.channel);
    if (localHit) {
      translations[index] = localHit.translation;
      if (localHit.source === "preserved") translationMemoryStats.preservedHits += 1;
      if (localHit.source === "glossary") translationMemoryStats.glossaryHits += 1;
      if (localHit.source === "memory") translationMemoryStats.memoryHits += 1;
      return;
    }
    requestItems.push({ block, index });
  });
  translationMemoryStats.requestedBlocks = requestItems.length;
  const requestGroups = dedupeTranslationRequestItems(requestItems);
  translationMemoryStats.deduplicatedBlocks = Math.max(0, requestItems.length - requestGroups.length);

  let transData: TranslationServiceResponse = { channel: config.channel };
  if (requestGroups.length > 0) {
    try {
      transData = await requestTextTranslationsWithFallback(
        serverUrls,
        token,
        requestGroups.map((item) => item.block),
        preferredSourceLang,
        targetLang,
        translationTimeoutMs,
      );
      (transData.translations || []).forEach((translated, requestIndex) => {
        const group = requestGroups[requestIndex];
        if (!group) return;
        group.indexes.forEach((originalIndex) => {
          translations[originalIndex] = translated;
        });
      });
    } catch (error) {
      throw new Error(normalizeLocalTranslateError(error));
    }
  }

  try {
    const retriedTranslations = await retryUntranslatedLatinBlocks(
      serverUrls,
      token,
      ocrBlocks,
      translations,
      targetLang,
      preferredSourceLang,
      translationTimeoutMs,
    );
    translations.splice(0, translations.length, ...retriedTranslations);
  } catch (error) {
    throw new Error(normalizeLocalTranslateError(error));
  }
  const { translations: normalizedTranslations, quality: translationQuality } = validateAndNormalizeTranslationResults(ocrBlocks, translations, targetLang);
  translationMemoryStats.stored = storeTranslationMemory(ocrBlocks, normalizedTranslations, preferredSourceLang, targetLang, transData.channel || config.channel);

  const resultBase64 = await renderTranslatedBlocks(base64, ocrBlocks, normalizedTranslations);
  const pairs: TranslatePair[] = buildTranslatePairs(ocrBlocks, normalizedTranslations, targetLang);
  return { resultBase64, pairs, usedChannel: transData.channel || config.channel || config.targetLang || "auto", usedServerUrl: transData.serverUrl || (requestGroups.length > 0 ? serverUrls[0] : "local-cache"), blocksCount: ocrBlocks.length, routePlan, normalization, translationQuality, translationMemoryStats, translationTimings: transData.timings };
};

const requestTextTranslationsWithFallback = async (
  serverUrls: string[],
  token: string,
  blocks: OcrBlock[],
  sourceLang: TranslationSourceLanguage,
  targetLang: string,
  timeoutMs: number,
) => {
  const errors: string[] = [];
  for (const serverUrl of serverUrls) {
    try {
      return await requestTextTranslations(serverUrl, token, blocks, sourceLang, targetLang, timeoutMs);
    } catch (error: any) {
      errors.push(`${serverUrl}: ${error?.message || error}`);
    }
  }
  throw new Error(errors.join("; "));
};
