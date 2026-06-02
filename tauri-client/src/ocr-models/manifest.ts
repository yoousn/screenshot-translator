import type { OcrModelManifest } from "./types";

export const defaultOcrModelManifest: OcrModelManifest = {
  schemaVersion: 1,
  runtime: "ysn-ocr-runtime",
  runtimeVersion: "0.1.0-planned",
  modelSetVersion: "2026.06.ocr.v1",
  defaultSourceLanguage: "auto",
  defaultProfile: "balanced",
  installedAt: null,
  lastSelfTestAt: null,
  packs: [
    {
      id: "auto-multilingual-balanced",
      name: {
        "zh-CN": "PP-OCRv5 基础模型包",
        "en-US": "PP-OCRv5 Base OCR Pack",
      },
      profile: "balanced",
      required: true,
      languages: ["zh-Hans", "zh-Hant", "en", "fr", "ja", "de", "es", "pt", "it", "ko", "ru", "ar", "th", "tr"],
      scripts: ["cjk", "latin", "hangul", "cyrillic", "arabic", "thai"],
      modelIds: ["det-default", "cls-default", "rec-cjk", "rec-latin", "rec-korean", "rec-cyrillic", "rec-arabic", "rec-thai"],
      status: "not-installed",
      lastSelfTestAt: null,
    },
    {
      id: "accurate-extension",
      name: {
        "zh-CN": "高精度扩展包（待开放）",
        "en-US": "Accurate Extension Pack (coming soon)",
      },
      profile: "accurate",
      required: false,
      languages: ["zh-Hans", "zh-Hant", "en", "fr", "ja", "de", "es", "pt", "it", "ko", "ru", "ar", "th", "tr"],
      scripts: ["cjk", "latin", "hangul", "cyrillic", "arabic", "thai"],
      modelIds: [],
      status: "not-installed",
      lastSelfTestAt: null,
    },
  ],
  models: [],
};

