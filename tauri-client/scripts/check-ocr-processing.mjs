import { build } from "esbuild";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const clientRoot = dirname(scriptDir);
const tempDir = join(clientRoot, ".tmp-ocr-processing-check");
const entryPath = join(tempDir, "check-ocr-processing.ts");
const outPath = join(tempDir, "check-ocr-processing.mjs");

const checkSource = String.raw`
import {
  filterUsefulOcrBlocks,
  buildVirtualOcrLines,
  buildOcrNormalizationReport,
  buildTranslationQualityPolicy,
  buildTranslationSystemInstruction,
  restoreCollapsedUiTextSpacing,
} from "../src/ocr-processing/index.ts";
import {
  buildTranslatePairs,
  buildTranslationRequestPayload,
  collectUntranslatedLatinRetryBlocks,
  evaluateTranslationQuality,
  isLikelyProtectedTechnicalText,
  mergeRetryTranslations,
  normalizeTranslationResults,
  selectPreferredSourceLanguage,
  hasLikelyNonEnglishLatinText,
  shouldRequireTranslation,
  validateAndNormalizeTranslationResults,
} from "../src/utils/ocrTranslationRequest.ts";
import { buildTranslationEraseRegion, shouldRenderTranslationBlock } from "../src/translation-render/renderGeometry.ts";
import { buildRenderBlocks } from "../src/translation-render/renderBlockLayout.ts";
import { createTranslationMemoryStats, lookupLocalTranslation, storeTranslationMemory } from "../src/utils/translationMemory.ts";
import translationGlossary from "../src/utils/translationGlossary.json";
import type { OcrBlock } from "../src/types/screenshot.ts";

const assert = (condition: unknown, message: string) => {
  if (!condition) throw new Error(message);
};

const block = (text: string, confidence: number, x: number, y: number, width = 40, height = 14): OcrBlock => ({
  text,
  confidence,
  box_coords: [[x, y], [x + width, y], [x + width, y + height], [x, y + height]],
});

const collapsed = "AddthemissingPATHfallbackbesideLocalModel.exepathsetting";
const restored = restoreCollapsedUiTextSpacing(collapsed);
assert(
  restored === "Add the missing PATH fallback beside Local Model.exe path setting",
  "collapsed UI spacing was not restored: " + restored,
);

const usefulBlocks = filterUsefulOcrBlocks([
  block("O", 0.7, 0, 0, 10, 12),
  block("○", 0.99, 0, 20, 10, 12),
  block("Add", 0.97, 20, 0, 28, 12),
  block("PATH", 0.96, 52, 0, 36, 12),
]);
assert(usefulBlocks.length === 2, "expected 2 useful OCR blocks, got " + usefulBlocks.length);
assert(usefulBlocks.every((item) => item.text !== "O" && item.text !== "○"), "icon-only OCR blocks were not filtered");

const virtualLines = buildVirtualOcrLines([
  block("AddthemissingPATH", 0.96, 10, 10, 132, 14),
  block("fallbackbeside", 0.95, 150, 11, 100, 14),
  block("LocalModel.exepathsetting", 0.94, 260, 10, 220, 14),
  block("download", 0.96, 12, 42, 72, 14),
  block("runtime", 0.96, 90, 43, 64, 14),
]);
assert(virtualLines.length === 2, "expected 2 virtual OCR lines, got " + virtualLines.length);
assert(
  virtualLines[0].text === "Add the missing PATH fallback beside Local Model.exe path setting",
  "first virtual line text mismatch: " + virtualLines[0].text,
);
assert(virtualLines[1].text === "download runtime", "second virtual line text mismatch: " + virtualLines[1].text);
assert(virtualLines[0].confidence > 0.94 && virtualLines[0].confidence < 0.97, "line confidence should average source blocks");

const normalization = await buildOcrNormalizationReport([
  block("O", 0.7, 0, 0, 10, 12),
  block("AddthemissingPATH", 0.96, 10, 10, 132, 14),
  block("fallbackbeside", 0.95, 150, 11, 100, 14),
  block("LocalModel.exepathsetting", 0.94, 260, 10, 220, 14),
  block("download", 0.96, 12, 42, 72, 14),
  block("runtime", 0.96, 90, 43, 64, 14),
]);
assert(normalization.rawCount === 6, "normalization raw count mismatch");
assert(normalization.usefulCount === 5, "normalization useful count mismatch");
assert(normalization.droppedCount === 1, "normalization dropped count mismatch");
assert(normalization.virtualLineCount === 2, "normalization virtual line count mismatch");
assert(
  normalization.text === "Add the missing PATH fallback beside Local Model.exe path setting\ndownload runtime",
  "normalization text should be newline-joined virtual lines: " + normalization.text,
);

const zhPolicy = buildTranslationQualityPolicy("zh-CN");
assert(zhPolicy.sourceLanguageMode === "auto", "translation source language must stay automatic");
assert(zhPolicy.targetLanguage === "zh-CN", "translation target language mismatch");
assert(zhPolicy.preserveLineCount, "translation policy must preserve line count");
assert(zhPolicy.preserveOrder, "translation policy must preserve order");
assert(zhPolicy.translateShortUiText, "translation policy must translate short UI text");
for (const term of ["PATH", "Windows", "LocalModel.exe", "LocalModel", "ONNX", "Ctrl+D"]) {
  assert(zhPolicy.protectedTerms.includes(term), "translation policy missing protected term: " + term);
}

const systemInstruction = buildTranslationSystemInstruction("zh-CN");
for (const expected of ["Detect the source language automatically", "Return exactly one translation for each input block", "Translate short UI labels", "Protected terms: PATH"]) {
  assert(systemInstruction.includes(expected), "translation system instruction missing: " + expected);
}

const sourceBlocks = [block("Add the missing PATH", 0.96, 0, 0), block("保存", 0.98, 0, 20)];
assert(selectPreferredSourceLanguage(sourceBlocks, "zh-CN") === "en", "Chinese target with Latin text should prefer English source retry path");
assert(selectPreferredSourceLanguage([block("保存", 0.98, 0, 0)], "zh-CN") === "auto", "Chinese target without Latin text should stay auto source");
assert(selectPreferredSourceLanguage([block("Open preview", 0.98, 0, 0), block("파일을 저장하세요", 0.98, 0, 20)], "zh-CN") === "auto", "mixed Latin and non-Latin scripts should stay automatic source");
assert(hasLikelyNonEnglishLatinText("Ouvrir l'aperçu avant d'enregistrer"), "French Latin text should be detected as non-English Latin");
assert(hasLikelyNonEnglishLatinText("Abrir vista previa antes de guardar"), "Spanish Latin text should be detected as non-English Latin");
assert(selectPreferredSourceLanguage([block("Ouvrir l'aperçu avant d'enregistrer", 0.98, 0, 0)], "zh-CN") === "auto", "French Latin text should stay automatic source");
assert(selectPreferredSourceLanguage([block("Abrir vista previa antes de guardar", 0.98, 0, 0)], "zh-CN") === "auto", "Spanish Latin text should stay automatic source");
assert(selectPreferredSourceLanguage(sourceBlocks, "ja") === "auto", "non-Chinese target should stay auto source");

const payload = buildTranslationRequestPayload(sourceBlocks, "en", "zh-CN");
assert(payload.source_lang === "en", "translation payload source_lang mismatch");
assert(payload.target_lang === "zh-CN", "translation payload target_lang mismatch");
assert(payload.blocks.length === 2 && payload.blocks[0].text === "Add the missing PATH", "translation payload blocks mismatch");
assert(payload.quality_policy.preserveLineCount, "translation payload must include quality policy");
assert(payload.system_instruction.includes("Target language: zh-CN"), "translation payload must include target instruction");

const retryBlocks = collectUntranslatedLatinRetryBlocks(sourceBlocks, ["Add the missing PATH", "保存"], "zh-CN", "auto");
assert(retryBlocks.length === 1 && retryBlocks[0].index === 0, "retry collection should target only untranslated Latin blocks");
assert(
  collectUntranslatedLatinRetryBlocks([block("PATH=C:\\Windows\\System32 && LocalModel.exe --help", 0.98, 0, 0, 360, 14)], ["PATH=C:\\Windows\\System32 && LocalModel.exe --help"], "zh-CN", "auto").length === 0,
  "retry collection must not send protected technical identifiers back to translation",
);
assert(collectUntranslatedLatinRetryBlocks(sourceBlocks, ["添加缺失的 PATH", "保存"], "zh-CN", "auto").length === 0, "translated Latin blocks should not retry");
assert(collectUntranslatedLatinRetryBlocks(sourceBlocks, ["Add the missing PATH", "保存"], "zh-CN", "en").length === 0, "English source requests should not retry again");
const mergedRetry = mergeRetryTranslations(["Add the missing PATH", "保存"], retryBlocks, ["添加缺失的 PATH"]);
assert(mergedRetry[0] === "添加缺失的 PATH" && mergedRetry[1] === "保存", "retry merge must preserve order and replace only retried entries");

const alignedTranslations = normalizeTranslationResults(sourceBlocks, ["添加缺失的 PATH", "", "extra translation"]);
assert(alignedTranslations.length === sourceBlocks.length, "normalized translations must match OCR block count");
assert(alignedTranslations[0] === "添加缺失的 PATH", "normalized translations should keep available translation");
assert(alignedTranslations[1] === "保存", "blank or missing translation should fall back to original text");
assert(isLikelyProtectedTechnicalText("COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md"), "uppercase filename should be preserved as a technical identifier");
assert(!isLikelyProtectedTechnicalText("Open preview"), "plain UI text should not be treated as a protected identifier");
assert(shouldRequireTranslation("Open preview", "zh-CN"), "plain English UI text should require translation");
assert(!shouldRequireTranslation("COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md", "zh-CN"), "technical filename should not require translation");
const quality = evaluateTranslationQuality(sourceBlocks, ["添加缺失的 PATH", "保存"], ["添加缺失的 PATH", "保存"], "zh-CN");
assert(quality.translatableCount === 1 && quality.translatedCount === 1, "quality summary should count translated translatable lines");
const preservedOnly = [block("COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md", 0.96, 0, 0, 280, 14)];
const preservedResult = validateAndNormalizeTranslationResults(preservedOnly, ["COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md"], "zh-CN");
assert(preservedResult.quality.translatableCount === 0 && preservedResult.quality.preservedCount === 1, "protected-only text should validate as preserved");
let blockedUntranslated = false;
try {
  validateAndNormalizeTranslationResults([block("Open preview", 0.96, 0, 0, 100, 14)], ["Open preview"], "zh-CN");
} catch {
  blockedUntranslated = true;
}
assert(blockedUntranslated, "unchanged translatable Latin text must be rejected instead of silently succeeding");
const pairs = buildTranslatePairs(sourceBlocks, ["添加缺失的 PATH"], "zh-CN");
assert(pairs.length === sourceBlocks.length, "translate pairs must match OCR block count");
assert(pairs[0].o === "Add the missing PATH" && pairs[0].t === "添加缺失的 PATH", "first translate pair mismatch");
assert(pairs[1].o === "保存" && pairs[1].t === "保存", "missing pair translation should fall back to original text");
assert(pairs[0].status === "translated" && pairs[1].status === "preserved", "translate pairs should expose translated/preserved status");

const memoryStats = createTranslationMemoryStats();
assert(memoryStats.preservedHits === 0 && memoryStats.requestedBlocks === 0, "translation memory stats should start empty");
assert(translationGlossary.zh.ui.save === "保存", "translation glossary manifest should provide Save => 保存");
assert(translationGlossary.zh.ui["open preview"] === "打开预览", "translation glossary manifest should provide Open preview => 打开预览");
const glossaryHit = lookupLocalTranslation(block("Save", 0.99, 0, 0), "en", "zh-CN", "google");
assert(glossaryHit?.source === "glossary" && glossaryHit.translation === "保存", "short UI glossary should translate Save locally");
const preservedHit = lookupLocalTranslation(block("COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md", 0.99, 0, 0, 280, 14), "en", "zh-CN", "google");
assert(preservedHit?.source === "preserved", "protected technical text should be satisfied locally");
const memoryBacking = new Map<string, string>();
(globalThis as any).window = {
  localStorage: {
    getItem: (key: string) => memoryBacking.get(key) ?? null,
    setItem: (key: string, value: string) => memoryBacking.set(key, value),
    removeItem: (key: string) => memoryBacking.delete(key),
  },
};
assert(
  storeTranslationMemory([block("Open settings panel before saving", 0.99, 0, 0, 240, 14)], ["保存前打开设置面板"], "en", "zh-CN", "google") === 1,
  "translated text should be stored in persistent translation memory",
);
const memoryHit = lookupLocalTranslation(block("Open settings panel before saving", 0.99, 0, 0, 240, 14), "en", "zh-CN", "google");
assert(memoryHit?.source === "memory" && memoryHit.translation === "保存前打开设置面板", "stored translation memory should satisfy repeated text locally");

const preservedRenderBlock = { text: "COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md", translated: "COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md", minX: 40, minY: 20, maxX: 360, maxY: 44, direction: "ltr" };
assert(!shouldRenderTranslationBlock(preservedRenderBlock), "preserved technical text should not be erased and redrawn");
const translatedRenderBlock = { text: "Open preview", translated: "打开预览", minX: 40, minY: 20, maxX: 160, maxY: 44, direction: "ltr" };
assert(shouldRenderTranslationBlock(translatedRenderBlock), "changed translations should be rendered");
const renderBlocks = buildRenderBlocks(
  [block("How are you?", 0.98, 120, 80, 110, 18), block("Oh hi, Ben.", 0.98, 120, 112, 94, 18)],
  ["你好吗？", "哦，Ben。"],
);
assert(renderBlocks.length === 2, "render blocks must preserve original OCR block positions instead of merging paragraphs");
assert(renderBlocks[0].minX === 120 && renderBlocks[0].minY === 80, "first render block should stay anchored to original OCR bounds");
assert(renderBlocks[1].minX === 120 && renderBlocks[1].minY === 112, "second render block should stay anchored to original OCR bounds");
const ltrRegion = buildTranslationEraseRegion(translatedRenderBlock, 640, 160, 260, 5, 3);
assert(ltrRegion.eraseX === 35, "LTR translation erase should stay anchored to the original left edge");
assert(ltrRegion.eraseRight > translatedRenderBlock.maxX, "LTR translation erase should expand to the right");
const rtlRegion = buildTranslationEraseRegion({ ...translatedRenderBlock, direction: "rtl" }, 640, 160, 260, 5, 3);
assert(rtlRegion.eraseRight === translatedRenderBlock.maxX + 5, "RTL translation erase should stay anchored to the original right edge");
assert(rtlRegion.eraseX < translatedRenderBlock.minX, "RTL translation erase should expand to the left");

console.log("OCR processing checks passed.");
`;

rmSync(tempDir, { recursive: true, force: true });
mkdirSync(tempDir, { recursive: true });
writeFileSync(entryPath, checkSource, "utf8");

try {
  await build({
    entryPoints: [entryPath],
    bundle: true,
    platform: "node",
    format: "esm",
    outfile: outPath,
    logLevel: "silent",
  });
  await import(pathToFileURL(outPath).href);
} finally {
  rmSync(tempDir, { recursive: true, force: true });
}

