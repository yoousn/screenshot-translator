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
import { distributeTranslationsForRender } from "../translation-render";

export type LocalTranslateConfig = {
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

export type TranslationServicePrewarmCandidate = {
  serverUrl: string;
  ok: boolean;
  latencyMs: number;
  checkedAt: string;
  statusCode?: number;
  activeChannel?: string;
  error?: string;
};

export type TranslationServicePrewarmSummary = {
  reason: string;
  checkedAt: string;
  preferredServerUrl: string;
  candidates: TranslationServicePrewarmCandidate[];
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

const DEFAULT_TRANSLATION_TIMEOUT_MS = 9000;
const RETRY_TRANSLATION_TIMEOUT_MS = 5000;
const TRANSLATION_HEDGE_DELAY_MS = 700;
const TRANSLATION_PREWARM_TIMEOUT_MS = 2500;
const TRANSLATION_PREWARM_CACHE_MS = 60_000;

export const buildTranslationServerCandidates = (config: LocalTranslateConfig) => {
  const remoteUrl = config.serverUrl || DEFAULT_TRANSLATION_SERVICE_URL;
  const candidates = [
    ...(config.preferLanServer && config.lanServerUrl ? [config.lanServerUrl] : []),
    remoteUrl,
  ];
  return Array.from(new Set(candidates.map((item) => item.trim()).filter(Boolean)));
};

let lastPrewarmSummary: TranslationServicePrewarmSummary | null = null;
let lastPrewarmSignature = "";
let lastPrewarmAt = 0;
let prewarmInFlight: Promise<TranslationServicePrewarmSummary> | null = null;

const fetchTranslationServiceHealth = async (
  serverUrl: string,
  timeoutMs: number,
): Promise<TranslationServicePrewarmCandidate> => {
  const normalizedServerUrl = normalizeTranslationServerUrl(serverUrl);
  const controller = new AbortController();
  const timeoutId = window.setTimeout(() => controller.abort(), timeoutMs);
  const started = performance.now();
  try {
    const response = await fetch(`${normalizedServerUrl}/api/health`, {
      method: "GET",
      cache: "no-store",
      signal: controller.signal,
    });
    const latencyMs = Math.round(performance.now() - started);
    let payload: any = null;
    try {
      payload = await response.json();
    } catch {
      payload = null;
    }
    return {
      serverUrl: normalizedServerUrl,
      ok: response.ok,
      latencyMs,
      checkedAt: new Date().toISOString(),
      statusCode: response.status,
      activeChannel: payload?.translation?.active_channel || payload?.channel,
      error: response.ok ? undefined : `HTTP ${response.status}`,
    };
  } catch (error: any) {
    return {
      serverUrl: normalizedServerUrl,
      ok: false,
      latencyMs: Math.round(performance.now() - started),
      checkedAt: new Date().toISOString(),
      error: error?.name === "AbortError" ? `timeout ${timeoutMs}ms` : (error?.message || String(error)),
    };
  } finally {
    window.clearTimeout(timeoutId);
  }
};

export const prewarmTranslationServices = async (
  config: LocalTranslateConfig,
  options: { force?: boolean; reason?: string; timeoutMs?: number } = {},
) => {
  const serverUrls = buildTranslationServerCandidates(config);
  const normalizedUrls = serverUrls.map((url) => normalizeTranslationServerUrl(url));
  const signature = normalizedUrls.join("|");
  const now = Date.now();
  if (
    !options.force
    && lastPrewarmSummary
    && signature === lastPrewarmSignature
    && now - lastPrewarmAt < TRANSLATION_PREWARM_CACHE_MS
  ) {
    return lastPrewarmSummary;
  }
  if (!options.force && prewarmInFlight) {
    return prewarmInFlight;
  }

  prewarmInFlight = (async () => {
    const reason = options.reason || "manual";
    const candidates = await Promise.all(
      normalizedUrls.map((serverUrl) => fetchTranslationServiceHealth(serverUrl, options.timeoutMs || TRANSLATION_PREWARM_TIMEOUT_MS)),
    );
    const preferredServerUrl = candidates.find((candidate) => candidate.ok)?.serverUrl || candidates[0]?.serverUrl || "";
    const summary: TranslationServicePrewarmSummary = {
      reason,
      checkedAt: new Date().toISOString(),
      preferredServerUrl,
      candidates,
    };
    lastPrewarmSummary = summary;
    lastPrewarmSignature = signature;
    lastPrewarmAt = Date.now();
    console.info("[Translation Service Prewarm]", summary);
    return summary;
  })().finally(() => {
    prewarmInFlight = null;
  });

  return prewarmInFlight;
};

export const getLastTranslationServicePrewarm = () => lastPrewarmSummary;

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

type TranslateOcrBlocksOptions = {
  flowStarted?: number;
  ocrMs?: number;
  source?: string;
};

export const translateOcrBlocks = async (
  base64: string,
  rawBlocks: OcrBlock[],
  config: LocalTranslateConfig,
  options: TranslateOcrBlocksOptions = {},
) => {
  const flowStarted = options.flowStarted ?? performance.now();
  const serverUrls = buildTranslationServerCandidates(config);
  const token = config.clientToken || "";
  const targetLang = config.targetLang || "zh";
  const translationTimeoutMs = config.translationTimeoutMs || DEFAULT_TRANSLATION_TIMEOUT_MS;
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
  let translationMs = 0;
  if (requestGroups.length > 0) {
    try {
      const translationStarted = performance.now();
      transData = await requestTextTranslationsWithFallback(
        serverUrls,
        token,
        requestGroups.map((item) => item.block),
        preferredSourceLang,
        targetLang,
        translationTimeoutMs,
      );
      translationMs = Math.round(performance.now() - translationStarted);
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

  const retryStarted = performance.now();
  try {
    const retriedTranslations = await retryUntranslatedLatinBlocks(
      serverUrls,
      token,
      ocrBlocks,
      translations,
      targetLang,
      preferredSourceLang,
      Math.min(translationTimeoutMs, RETRY_TRANSLATION_TIMEOUT_MS),
    );
    translations.splice(0, translations.length, ...retriedTranslations);
  } catch (error) {
    throw new Error(normalizeLocalTranslateError(error));
  }
  const retryMs = Math.round(performance.now() - retryStarted);
  const { translations: normalizedTranslations, quality: translationQuality } = validateAndNormalizeTranslationResults(ocrBlocks, translations, targetLang);
  translationMemoryStats.stored = storeTranslationMemory(ocrBlocks, normalizedTranslations, preferredSourceLang, targetLang, transData.channel || config.channel);

  const renderStarted = performance.now();
  const distributedRender = distributeTranslationsForRender(ocrBlocks, normalizedTranslations, normalization.renderBlocks || ocrBlocks);
  const resultBase64 = await renderTranslatedBlocks(base64, distributedRender.blocks, distributedRender.translations);
  const renderMs = Math.round(performance.now() - renderStarted);
  const pairs: TranslatePair[] = buildTranslatePairs(ocrBlocks, normalizedTranslations, targetLang);
  return { resultBase64, pairs, usedChannel: transData.channel || config.channel || config.targetLang || "auto", usedServerUrl: transData.serverUrl || (requestGroups.length > 0 ? serverUrls[0] : "local-cache"), blocksCount: ocrBlocks.length, routePlan, normalization, translationQuality, translationMemoryStats, translationTimings: transData.timings, servicePrewarm: getLastTranslationServicePrewarm(), localTimings: { source: options.source || "rapidocr", ocrMs: options.ocrMs ?? 0, translationMs, retryMs, renderMs, totalMs: Math.round(performance.now() - flowStarted) } };
};

export const translateWithLocalOcr = async (base64: string, config: LocalTranslateConfig) => {
  const flowStarted = performance.now();
  let rawBlocks: OcrBlock[];
  const ocrStarted = performance.now();
  try {
    rawBlocks = await invoke("run_local_ocr", {
      imageBase64: base64,
      executablePath: null,
      timeoutMs: config.localOcrTimeoutMs || 15000,
    });
  } catch (error) {
    throw new Error(normalizeLocalTranslateError(error));
  }
  const ocrMs = Math.round(performance.now() - ocrStarted);
  return translateOcrBlocks(base64, rawBlocks, config, { flowStarted, ocrMs, source: "rapidocr" });
};

const requestTextTranslationsWithFallback = async (
  serverUrls: string[],
  token: string,
  blocks: OcrBlock[],
  sourceLang: TranslationSourceLanguage,
  targetLang: string,
  timeoutMs: number,
) => {
  if (serverUrls.length <= 1) {
    return await requestTextTranslations(serverUrls[0], token, blocks, sourceLang, targetLang, timeoutMs);
  }

  const errors: string[] = [];
  let started = 0;
  let finished = 0;
  let settled = false;

  return await new Promise<TranslationServiceResponse>((resolve, reject) => {
    const startCandidate = (serverUrl: string) => {
      if (settled) return;
      started += 1;
      requestTextTranslations(serverUrl, token, blocks, sourceLang, targetLang, timeoutMs)
        .then((result) => {
          if (settled) return;
          settled = true;
          resolve(result);
        })
        .catch((error: any) => {
          errors.push(`${serverUrl}: ${error?.message || error}`);
        })
        .finally(() => {
          finished += 1;
          if (!settled && started >= serverUrls.length && finished >= started) {
            settled = true;
            reject(new Error(errors.join("; ")));
          }
        });
    };

    serverUrls.forEach((serverUrl, index) => {
      window.setTimeout(() => startCandidate(serverUrl), index * TRANSLATION_HEDGE_DELAY_MS);
    });
  });
};
