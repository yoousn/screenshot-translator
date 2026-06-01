import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_TRANSLATION_SERVICE_URL } from "./translationService";
import type { OcrBlock, TranslatePair } from "../types/screenshot";
import { renderTranslatedBlocks } from "./translatedBlocks";

type LocalTranslateConfig = {
  serverUrl?: string;
  clientToken?: string;
  targetLang?: string;
  channel?: string;
  localOcrExecutablePath?: string;
  localOcrTimeoutMs?: number;
};

type BlockBounds = { minX: number; minY: number; maxX: number; maxY: number };

const hasLatinText = (text: string) => /[A-Za-z]{2,}/.test(text);
const normalizeForCompare = (text: string) => text.replace(/\s+/g, " ").trim().toLowerCase();
const iconOnlyPattern = /^[oO]$/;

const getBounds = (block: OcrBlock): BlockBounds => {
  const xs = block.box_coords.map((point) => point[0]);
  const ys = block.box_coords.map((point) => point[1]);
  return { minX: Math.min(...xs), minY: Math.min(...ys), maxX: Math.max(...xs), maxY: Math.max(...ys) };
};

const fixCollapsedUiText = (text: string) => {
  let next = text.replace(/\s+/g, " ").trim();
  const technicalTokens = ["PaddleOCR-json", "PaddleOCR", "Windows", "PATH", "OCR", "ONNX", "RapidOCR"];
  for (const token of technicalTokens) next = next.replace(new RegExp(token, "gi"), token);
  next = next
    .replace(/\b(Add|Bundle|into|the|missing|fallback|for|local|self|test|beside|executable|path|setting|build|works|anywhere)(?=[A-Z])/g, "$1 ")
    .replace(/([a-z])([A-Z][a-z])/g, "$1 $2")
    .replace(/(PaddleOCR-json\.exe)(?=[A-Za-z])/g, "$1 ")
    .replace(/(PATH)(?=[a-z])/g, "$1 ")
    .replace(/\s+/g, " ")
    .trim();
  return next;
};

const isUsefulBlock = (block: OcrBlock) => {
  const text = block.text.trim();
  if (!text) return false;
  if (iconOnlyPattern.test(text) && block.confidence < 0.92) return false;
  const bounds = getBounds(block);
  const width = Math.max(1, bounds.maxX - bounds.minX);
  const height = Math.max(1, bounds.maxY - bounds.minY);
  if (iconOnlyPattern.test(text) && width <= height * 1.5) return false;
  return block.box_coords.length >= 4;
};

const buildVirtualLines = (blocks: OcrBlock[]) => {
  const items = blocks
    .filter(isUsefulBlock)
    .map((block) => ({ block, bounds: getBounds(block) }))
    .sort((a, b) => (a.bounds.minY + a.bounds.maxY) / 2 - (b.bounds.minY + b.bounds.maxY) / 2 || a.bounds.minX - b.bounds.minX);

  const rows: typeof items[] = [];
  const heights = items.map((item) => Math.max(1, item.bounds.maxY - item.bounds.minY)).sort((a, b) => a - b);
  const medianHeight = heights[Math.floor(heights.length / 2)] || 12;
  const rowTolerance = Math.max(6, medianHeight * 0.65);

  for (const item of items) {
    const centerY = (item.bounds.minY + item.bounds.maxY) / 2;
    const row = rows.find((candidate) => {
      const rowCenter = candidate.reduce((sum, entry) => sum + (entry.bounds.minY + entry.bounds.maxY) / 2, 0) / candidate.length;
      return Math.abs(rowCenter - centerY) <= rowTolerance;
    });
    if (row) row.push(item);
    else rows.push([item]);
  }

  return rows.map((row) => {
    const sorted = [...row].sort((a, b) => a.bounds.minX - b.bounds.minX);
    const minX = Math.min(...sorted.map((item) => item.bounds.minX));
    const minY = Math.min(...sorted.map((item) => item.bounds.minY));
    const maxX = Math.max(...sorted.map((item) => item.bounds.maxX));
    const maxY = Math.max(...sorted.map((item) => item.bounds.maxY));
    const text = fixCollapsedUiText(sorted.map((item) => item.block.text.trim()).join(" "));
    return {
      text,
      confidence: sorted.reduce((sum, item) => sum + item.block.confidence, 0) / sorted.length,
      box_coords: [[minX, minY], [maxX, minY], [maxX, maxY], [minX, maxY]] as [number, number][],
    } satisfies OcrBlock;
  }).filter((block) => block.text);
};

const shouldUseEnglishSource = (blocks: OcrBlock[], targetLang: string) => (
  (targetLang === "zh" || targetLang.startsWith("zh")) && blocks.some((block) => hasLatinText(block.text))
);

const requestTextTranslations = async (serverUrl: string, token: string, blocks: OcrBlock[], sourceLang: string, targetLang: string) => {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/translate_text`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "x-api-key": token },
    body: JSON.stringify({
      blocks: blocks.map((block) => ({ text: block.text, confidence: block.confidence, box: block.box_coords })),
      source_lang: sourceLang,
      target_lang: targetLang,
    }),
  });
  if (!response.ok) throw new Error(`Text translation API failed: ${response.status}`);
  const transData = await response.json();
  if (transData.status !== "success") throw new Error(transData.error || "Text translation failed");
  return transData as { translations?: string[]; channel?: string };
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
  const ocrBlocks = buildVirtualLines(rawBlocks || []);
  if (!ocrBlocks.length) throw new Error("Local OCR did not recognize text");

  const preferredSourceLang = shouldUseEnglishSource(ocrBlocks, targetLang) ? "en" : "auto";
  const transData = await requestTextTranslations(serverUrl, token, ocrBlocks, preferredSourceLang, targetLang);
  let translations: string[] = transData.translations || [];

  const retryBlocks = ocrBlocks.filter((block, index) => {
    const translated = translations[index] || "";
    return hasLatinText(block.text) && normalizeForCompare(translated) === normalizeForCompare(block.text);
  });
  if (retryBlocks.length > 0 && preferredSourceLang !== "en" && (targetLang === "zh" || targetLang.startsWith("zh"))) {
    const retryData = await requestTextTranslations(serverUrl, token, retryBlocks, "en", targetLang);
    const retryTranslations = retryData.translations || [];
    translations = translations.map((item, index) => {
      const retryIndex = retryBlocks.findIndex((block) => block === ocrBlocks[index]);
      return retryIndex >= 0 ? (retryTranslations[retryIndex] || item) : item;
    });
  }

  const resultBase64 = await renderTranslatedBlocks(base64, ocrBlocks, translations);
  const pairs: TranslatePair[] = ocrBlocks.map((block, index) => ({ o: block.text, t: translations[index] || block.text }));
  return { resultBase64, pairs, usedChannel: transData.channel || config.channel || config.targetLang || "auto", blocksCount: ocrBlocks.length };
};
