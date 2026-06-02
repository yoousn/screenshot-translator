import { restoreProtectedTechnicalTerms } from "./technicalTerms";

const commonUiWords = [
  "Add",
  "Bundle",
  "into",
  "the",
  "missing",
  "fallback",
  "for",
  "local",
  "self",
  "test",
  "beside",
  "executable",
  "path",
  "setting",
  "build",
  "works",
  "anywhere",
  "download",
  "runtime",
  "model",
  "language",
  "translate",
  "recording",
  "config",
];

const commonUiWordMap = new Map(commonUiWords.map((word) => [word.toLowerCase(), word]));
const commonUiWordsByLength = [...commonUiWords].sort((a, b) => b.length - a.length);

const segmentCollapsedUiWords = (chunk: string) => {
  const lowerChunk = chunk.toLowerCase();
  const segments: string[] = [];
  let index = 0;

  while (index < lowerChunk.length) {
    const match = commonUiWordsByLength.find((word) => lowerChunk.startsWith(word.toLowerCase(), index));
    if (!match) return chunk;
    segments.push(commonUiWordMap.get(match.toLowerCase()) || match);
    index += match.length;
  }

  return segments.length > 1 ? segments.join(" ") : chunk;
};

export const hasLatinText = (text: string) => /[A-Za-z]{2,}/.test(text);
export const normalizeForCompare = (text: string) => text.replace(/\s+/g, " ").trim().toLowerCase();

export const restoreCollapsedUiTextSpacing = (text: string) => {
  let next = restoreProtectedTechnicalTerms(text.replace(/\s+/g, " ").trim());
  const wordPattern = new RegExp(`\\b(${commonUiWords.join("|")})(?=[A-Z])`, "g");
  next = next
    .replace(/([a-z])(?=PATH)/g, "$1 ")
    .replace(wordPattern, "$1 ")
    .replace(/([a-z])([A-Z][a-z])/g, "$1 $2")
    .replace(/(ffmpeg\.exe)(?=[A-Za-z])/gi, "$1 ")
    .replace(/([A-Za-z0-9_-]+\.exe)(?=[A-Za-z])/gi, "$1 ")
    .replace(/(PATH)(?=[a-z])/g, "$1 ")
    .replace(/\b[A-Za-z]{6,}\b/g, (chunk) => segmentCollapsedUiWords(chunk))
    .replace(/\s+/g, " ")
    .trim();
  return restoreProtectedTechnicalTerms(next);
};
