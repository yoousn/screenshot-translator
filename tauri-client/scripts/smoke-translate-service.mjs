import { readFileSync } from "node:fs";
import { join } from "node:path";

const appData = process.env.LOCALAPPDATA || join(process.env.USERPROFILE || "", "AppData", "Local");
const configPath = join(appData, "ScreenshotTranslator", "config.json");
const readJsonConfig = (path) => JSON.parse(readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
const config = readJsonConfig(configPath);
const serverUrl = ((config.preferLanServer && config.lanServerUrl) ? config.lanServerUrl : (config.serverUrl || "https://ocr.yousn.me")).replace(/\/$/, "");
const token = config.clientToken || "";
const targetLang = config.targetLang || "zh";
const duplicateProbe = `Open duplicate probe ${Date.now()} before saving`;

const cases = [
  { name: "tiny-ui", text: "Open preview", shouldTranslate: true },
  { name: "single-word", text: "Save", shouldTranslate: true },
  { name: "short-fragment", text: "Open preview and", shouldTranslate: true },
  { name: "duplicate-probe-a", text: duplicateProbe, shouldTranslate: true },
  { name: "duplicate-probe-b", text: duplicateProbe, shouldTranslate: true },
  { name: "mixed-zh-en", text: "打开 preview before saving", shouldTranslate: true },
  { name: "korean", text: "파일을 저장하세요", shouldTranslate: true },
  { name: "arabic", text: "افتح المعاينة قبل الحفظ", shouldTranslate: true },
  { name: "japanese", text: "保存する前にプレビューを開く", shouldTranslate: true },
  { name: "french", text: "Ouvrir l'aperçu avant d'enregistrer", shouldTranslate: true },
  { name: "spanish", text: "Abrir vista previa antes de guardar", shouldTranslate: true },
  { name: "medium-ui", text: "Translate selected text and keep commands safe", shouldTranslate: true },
  { name: "multi-line", text: "Open preview before saving\nCheck OCR result window\nCopy translated text", shouldTranslate: true },
  { name: "technical-filename", text: "COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md", shouldTranslate: false },
  { name: "technical-command", text: "PATH=C:\\Windows\\System32 && LocalModel.exe --help", shouldTranslate: false },
];

const normalize = (text) => String(text || "").replace(/\s+/g, " ").trim().toLowerCase();
const hasChinese = (text) => /[\u3400-\u9fff]/.test(text);
const isAlreadyChineseDominant = (text) => {
  const compact = String(text || "").replace(/\s+/g, "");
  if (!compact) return false;
  const chineseCount = Array.from(compact).filter((char) => /[\u3400-\u9fff]/.test(char)).length;
  return chineseCount / Array.from(compact).length >= 0.5;
};
const hasLikelyNonEnglishLatinText = (text) => (
  /[À-ÖØ-öø-ÿ]/.test(text)
  || /\b(?:abrir|antes|guardar|vista|previa|ouvrir|aperçu|apercu|avant|enregistrer|de|des|du|del|para|por|con|sin)\b/i.test(text)
);
const hasExpectedSemanticKeywords = (item, translated) => {
  if (item.name !== "french" && item.name !== "spanish") return true;
  return /打开/.test(translated) && /预览/.test(translated) && /保存/.test(translated);
};

const sourceHintForText = (text) => {
  if (/[\uac00-\ud7af]/.test(text)) return "ko";
  if (/[\u0600-\u06ff]/.test(text)) return "ar";
  if (/[\u3040-\u30ff]/.test(text)) return "ja";
  if (/[\u0400-\u052f]/.test(text)) return "ru";
  if (/[\u0e00-\u0e7f]/.test(text)) return "th";
  if (hasLikelyNonEnglishLatinText(text)) return "auto";
  if (/[A-Za-z]{2,}/.test(text)) return "en";
  return "auto";
};

const translateBatch = async (batchCases, sourceLang) => {
  const payload = {
    blocks: batchCases.map((item, index) => ({
      text: item.text,
      confidence: 0.96,
      box: [[0, index * 30], [420, index * 30], [420, index * 30 + 24], [0, index * 30 + 24]],
    })),
    source_lang: sourceLang,
    target_lang: targetLang,
  };

  const startedAt = performance.now();
  const response = await fetch(`${serverUrl}/api/translate_text`, {
    method: "POST",
    headers: { "Content-Type": "application/json", "x-api-key": token },
    body: JSON.stringify(payload),
  });
  const durationMs = Math.round(performance.now() - startedAt);

  if (!response.ok) {
    throw new Error(`translate service returned HTTP ${response.status}`);
  }

  const data = await response.json();
  if (data.status !== "success") {
    throw new Error(`translate service failed: ${data.error || "unknown error"}`);
  }

  return { data, durationMs, translations: data.translations || [] };
};

const formatTimings = (data) => {
  const timings = data.timings || {};
  const parts = [
    typeof timings.total_ms === "number" ? `server=${timings.total_ms}ms` : "",
    typeof timings.provider_ms === "number" ? `provider=${timings.provider_ms}ms` : "",
    typeof timings.cache_hits === "number" ? `cache=${timings.cache_hits}` : (typeof data.cache_hits === "number" ? `cache=${data.cache_hits}` : ""),
    typeof timings.provider_misses === "number" ? `miss=${timings.provider_misses}` : "",
    typeof timings.request_duplicates === "number" ? `dup=${timings.request_duplicates}` : "",
    typeof timings.preserved_hits === "number" ? `keep=${timings.preserved_hits}` : "",
  ].filter(Boolean);
  return parts.length ? ` ${parts.join(" ")}` : "";
};

const batches = new Map();
for (const item of cases) {
  const sourceLang = sourceHintForText(item.text);
  batches.set(sourceLang, [...(batches.get(sourceLang) || []), item]);
}

let totalDurationMs = 0;
let channel = "unknown";
const translationsByName = new Map();
for (const [sourceLang, batchCases] of batches.entries()) {
  const { data, durationMs, translations } = await translateBatch(batchCases, sourceLang);
  totalDurationMs += durationMs;
  channel = data.channel || channel;
  batchCases.forEach((item, index) => translationsByName.set(item.name, translations[index] || ""));
  console.log(`[BATCH] source=${sourceLang} blocks=${batchCases.length} client=${durationMs}ms${formatTimings(data)}`);
}

const rows = cases.map((item) => {
  const translated = translationsByName.get(item.name) || "";
  const unchanged = normalize(translated) === normalize(item.text);
  const preservesExpectedLines = item.name !== "multi-line" || translated.split(/\r?\n/).filter(Boolean).length >= 2;
  const semanticKeywordsOk = hasExpectedSemanticKeywords(item, translated);
  const passed = item.shouldTranslate
    ? Boolean(translated) && hasChinese(translated) && preservesExpectedLines && semanticKeywordsOk && (!unchanged || isAlreadyChineseDominant(item.text))
    : Boolean(translated);
  return { ...item, translated, passed };
});

const failed = rows.filter((row) => !row.passed);
for (const row of rows) {
  console.log(`[${row.passed ? "OK" : "FAIL"}] ${row.name}: ${row.text} => ${row.translated}`);
}

if (failed.length > 0) {
  throw new Error(`translation smoke failed for: ${failed.map((row) => row.name).join(", ")}`);
}

console.log(`Translation service smoke passed via ${serverUrl} (${channel}) in ${totalDurationMs}ms across ${batches.size} batches / ${cases.length} blocks.`);
