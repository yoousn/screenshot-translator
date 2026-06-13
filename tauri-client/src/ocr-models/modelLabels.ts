import type { RapidOcrModelVersion } from "./types";

export const localOcrModelOptions: Array<{ value: RapidOcrModelVersion; label: string }> = [
  { value: "v6", label: "PP-OCRv6 Small（默认）" },
  { value: "v5", label: "Rapid OCR V5 多语言" },
  { value: "v4", label: "Rapid OCR V4 兼容模式" },
];

export function localOcrModelLabel(version: RapidOcrModelVersion): string {
  return localOcrModelOptions.find((option) => option.value === version)?.label || "PP-OCRv6 Small（默认）";
}

export function localOcrModelName(version: RapidOcrModelVersion): string {
  if (version === "v5") return "Rapid OCR V5 多语言";
  if (version === "v4") return "Rapid OCR V4 兼容模式";
  return "PP-OCRv6 Small";
}
