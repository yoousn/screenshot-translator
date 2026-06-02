export type TranslationQualityPolicy = {
  preserveLineCount: boolean;
  preserveOrder: boolean;
  translateShortUiText: boolean;
  sourceLanguageMode: "auto";
  targetLanguage: string;
  protectedTerms: string[];
  instructions: string[];
};

const protectedTerms = [
  "PATH",
  "Windows",
  "OCR",
  "ONNX",
  "RapidOCR",
  "PaddleOCR-json",
  "PaddleOCR-json.exe",
  "ffmpeg.exe",
  "YSN OCR Runtime",
  "Ctrl+D",
  "Ctrl+Q",
];

export const buildTranslationQualityPolicy = (targetLanguage: string): TranslationQualityPolicy => ({
  preserveLineCount: true,
  preserveOrder: true,
  translateShortUiText: true,
  sourceLanguageMode: "auto",
  targetLanguage,
  protectedTerms,
  instructions: [
    "Detect the source language automatically; never require the user to choose source language.",
    "Return exactly one translation for each input block, preserving order and count.",
    "Translate short UI labels, buttons, menu items, list items, and error messages; do not skip short English fragments.",
    "Preserve technical identifiers, commands, paths, flags, package names, executable names, and version-like tokens.",
    "Do not merge separate input lines into one output line.",
    "If a line is already in the target language, keep it natural and clean instead of returning an empty string.",
  ],
});

export const buildTranslationSystemInstruction = (targetLanguage: string) => {
  const policy = buildTranslationQualityPolicy(targetLanguage);
  return [
    "You are a screenshot UI translation engine for a commercial desktop app.",
    ...policy.instructions,
    `Target language: ${targetLanguage}.`,
    `Protected terms: ${policy.protectedTerms.join(", ")}.`,
  ].join("\n");
};
