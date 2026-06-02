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
  mergeRetryTranslations,
  normalizeTranslationResults,
  selectPreferredSourceLanguage,
} from "../src/utils/ocrTranslationRequest.ts";
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
assert(selectPreferredSourceLanguage(sourceBlocks, "ja") === "auto", "non-Chinese target should stay auto source");

const payload = buildTranslationRequestPayload(sourceBlocks, "en", "zh-CN");
assert(payload.source_lang === "en", "translation payload source_lang mismatch");
assert(payload.target_lang === "zh-CN", "translation payload target_lang mismatch");
assert(payload.blocks.length === 2 && payload.blocks[0].text === "Add the missing PATH", "translation payload blocks mismatch");
assert(payload.quality_policy.preserveLineCount, "translation payload must include quality policy");
assert(payload.system_instruction.includes("Target language: zh-CN"), "translation payload must include target instruction");

const retryBlocks = collectUntranslatedLatinRetryBlocks(sourceBlocks, ["Add the missing PATH", "保存"], "zh-CN", "auto");
assert(retryBlocks.length === 1 && retryBlocks[0].index === 0, "retry collection should target only untranslated Latin blocks");
assert(collectUntranslatedLatinRetryBlocks(sourceBlocks, ["添加缺失的 PATH", "保存"], "zh-CN", "auto").length === 0, "translated Latin blocks should not retry");
assert(collectUntranslatedLatinRetryBlocks(sourceBlocks, ["Add the missing PATH", "保存"], "zh-CN", "en").length === 0, "English source requests should not retry again");
const mergedRetry = mergeRetryTranslations(["Add the missing PATH", "保存"], retryBlocks, ["添加缺失的 PATH"]);
assert(mergedRetry[0] === "添加缺失的 PATH" && mergedRetry[1] === "保存", "retry merge must preserve order and replace only retried entries");

const alignedTranslations = normalizeTranslationResults(sourceBlocks, ["添加缺失的 PATH", "", "extra translation"]);
assert(alignedTranslations.length === sourceBlocks.length, "normalized translations must match OCR block count");
assert(alignedTranslations[0] === "添加缺失的 PATH", "normalized translations should keep available translation");
assert(alignedTranslations[1] === "保存", "blank or missing translation should fall back to original text");
const pairs = buildTranslatePairs(sourceBlocks, ["添加缺失的 PATH"]);
assert(pairs.length === sourceBlocks.length, "translate pairs must match OCR block count");
assert(pairs[0].o === "Add the missing PATH" && pairs[0].t === "添加缺失的 PATH", "first translate pair mismatch");
assert(pairs[1].o === "保存" && pairs[1].t === "保存", "missing pair translation should fall back to original text");

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

