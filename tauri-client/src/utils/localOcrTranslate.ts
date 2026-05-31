import { invoke } from "@tauri-apps/api/core";
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

const hasLatinText = (text: string) => /[A-Za-z]{2,}/.test(text);
const normalizeForCompare = (text: string) => text.replace(/\s+/g, " ").trim().toLowerCase();

const shouldUseEnglishSource = (blocks: OcrBlock[], targetLang: string) => (
  (targetLang === "zh" || targetLang.startsWith("zh")) && blocks.some((block) => hasLatinText(block.text))
);

const requestTextTranslations = async (serverUrl: string, token: string, blocks: OcrBlock[], sourceLang: string, targetLang: string) => {
  const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/translate_text`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-api-key": token,
    },
    body: JSON.stringify({
      blocks: blocks.map((block) => ({
        text: block.text,
        confidence: block.confidence,
        box: block.box_coords,
      })),
      source_lang: sourceLang,
      target_lang: targetLang,
    }),
  });

  if (!response.ok) {
    throw new Error(`文本翻译接口异常：${response.status}`);
  }

  const transData = await response.json();
  if (transData.status !== "success") {
    throw new Error(transData.error || "文本翻译失败");
  }

  return transData as { translations?: string[]; channel?: string };
};

export const translateWithLocalOcr = async (base64: string, config: LocalTranslateConfig) => {
  const serverUrl = config.serverUrl || "https://ocr.yousn.me";
  const token = config.clientToken || "";
  const targetLang = config.targetLang || "zh";

  const ocrBlocks: OcrBlock[] = await invoke("run_local_ocr", {
    imageBase64: base64,
    executablePath: config.localOcrExecutablePath || null,
    timeoutMs: config.localOcrTimeoutMs || 15000,
  });

  if (!ocrBlocks || ocrBlocks.length === 0) {
    throw new Error("本地 OCR 未识别到文字");
  }

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

  return {
    resultBase64,
    pairs,
    usedChannel: transData.channel || config.channel || config.targetLang || "auto",
    blocksCount: ocrBlocks.length,
  };
};
