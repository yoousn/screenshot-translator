export type TranslationQualityPolicy = {
  preserveLineCount: boolean;
  preserveOrder: boolean;
  translateShortUiText: boolean;
  sourceLanguageMode: "auto";
  targetLanguage: string;
  protectedTerms: string[];
  domainTermHints: Record<string, string>;
  instructions: string[];
};

const protectedTerms = [
  "Codex",
  "OpenAI",
  "ChatGPT",
  "API",
  "APIs",
  "SDK",
  "MCP",
  "GPT-5",
  "VLM",
  "PATH",
  "Windows",
  "OCR",
  "ONNX",
  "RapidOCR",
  "LocalModel",
  "LocalModel.exe",
  "ffmpeg.exe",
  "Ctrl+D",
  "Ctrl+Q",
];

const domainTermHints: Record<string, string> = {
  ticket: "工单（论坛、客服、缺陷反馈、支持上下文），不要译成票、门票或票据",
  fixture: "固定测试样例 / 测试夹具（软件测试上下文）；只有硬件/机械上下文才译成固定装置",
  fallback: "兜底 / 回退（产品和模型链路上下文），不要译成后备",
  issue: "问题 / 议题；代码托管或社区反馈上下文可译为问题",
  bug: "漏洞 / 缺陷；按上下文选择更自然的中文",
};

const buildContextInstruction = (sourceTexts: string[]) => {
  const normalizedItems = sourceTexts
    .map((item) => item.replace(/\s+/g, " ").trim())
    .filter(Boolean)
    .slice(0, 80);
  if (normalizedItems.length === 0) return "";

  const contextText = normalizedItems.join("\n").slice(0, 4000);
  const looksTechnical = /\b(?:Codex|OpenAI|ChatGPT|API|APIs|SDK|MCP|GPT(?:-\d+(?:\.\d+)?)?|VLM|fixture|fallback|ticket|bug|issue)\b/i.test(contextText);
  return [
    "Use the full screenshot context below to disambiguate each block, but still return one translation per input block.",
    looksTechnical ? "The screenshot appears to contain developer/product/community text; prefer software and support-domain terminology over physical-object meanings." : "",
    "Screenshot context:",
    contextText,
  ].filter(Boolean).join("\n");
};

export const buildTranslationQualityPolicy = (targetLanguage: string): TranslationQualityPolicy => ({
  preserveLineCount: true,
  preserveOrder: true,
  translateShortUiText: true,
  sourceLanguageMode: "auto",
  targetLanguage,
  protectedTerms,
  domainTermHints,
  instructions: [
    "Detect the source language automatically; never require the user to choose source language.",
    "Return exactly one translation for each input block, preserving order and count.",
    "Translate short UI labels, buttons, menu items, list items, and error messages; do not skip short English fragments.",
    "Preserve technical identifiers, commands, paths, flags, package names, executable names, and version-like tokens.",
    "Use surrounding screenshot context to choose domain-appropriate terminology instead of translating each isolated word literally.",
    "Do not merge separate input lines into one output line.",
    "If a line is already in the target language, keep it natural and clean instead of returning an empty string.",
  ],
});

export const buildTranslationSystemInstruction = (targetLanguage: string, sourceTexts: string[] = []) => {
  const policy = buildTranslationQualityPolicy(targetLanguage);
  const contextInstruction = buildContextInstruction(sourceTexts);
  return [
    "You are a screenshot UI translation engine for a commercial desktop app.",
    ...policy.instructions,
    `Target language: ${targetLanguage}.`,
    `Protected terms: ${policy.protectedTerms.join(", ")}.`,
    `Domain term hints: ${Object.entries(policy.domainTermHints).map(([source, hint]) => `${source}=${hint}`).join("; ")}.`,
    contextInstruction,
  ].join("\n");
};
