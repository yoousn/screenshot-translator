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
const fileExtensionPattern = /exe|dll|json|md|markdown|txt|onnx|yaml|yml|toml|rs|ts|tsx|js|jsx|mjs|py|ps1|bat|cmd|png|jpe?g|webp|gif|zip|7z|msi|nsi|lock|log/i;
const splitFileExtensionPattern = new RegExp(`([A-Za-z0-9_-]+)\\.\\s+(${fileExtensionPattern.source})\\b`, "gi");
const gluedFileExtensionPattern = new RegExp(`([A-Za-z0-9_-]+\\.(?:${fileExtensionPattern.source}))(?=[A-Za-z])`, "gi");
const technicalFileTokenPattern = new RegExp(`\\b[A-Za-z0-9_-]+\\.(?:${fileExtensionPattern.source})\\b`, "gi");
const maybeGluedTechnicalFilePattern = new RegExp(`\\b([A-Za-z][A-Za-z0-9_-]*)\\.(${fileExtensionPattern.source})\\b`, "gi");

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

const splitCommonUiPrefixBeforeTechnicalFile = (text: string) => (
  text.replace(maybeGluedTechnicalFilePattern, (match, stem: string, extension: string) => {
    for (let index = 1; index < stem.length; index += 1) {
      if (!/[a-z]/.test(stem[index - 1]) || !/[A-Z]/.test(stem[index])) continue;
      const prefix = stem.slice(0, index);
      const suffix = stem.slice(index);
      const segmentedPrefix = segmentCollapsedUiWords(prefix);
      if (segmentedPrefix === prefix || segmentedPrefix.split(" ").length < 2) continue;
      return `${segmentedPrefix} ${suffix}.${extension}`;
    }
    return match;
  })
);

export const hasLatinText = (text: string) => /[A-Za-z]{2,}/.test(text);
export const normalizeForCompare = (text: string) => text.replace(/\s+/g, " ").trim().toLowerCase();

export const restoreCollapsedUiTextSpacing = (text: string) => {
  let next = restoreProtectedTechnicalTerms(text.replace(/\s+/g, " ").trim());
  next = next
    .replace(splitFileExtensionPattern, "$1.$2")
    .replace(gluedFileExtensionPattern, "$1 ");

  const wordPattern = new RegExp(`\\b(${commonUiWords.join("|")})(?=[A-Z])`, "g");
  next = next
    .replace(/([a-z])(?=PATH)/g, "$1 ")
    .replace(wordPattern, "$1 ")
    .replace(/(PATH)(?=[a-z])/g, "$1 ");

  next = splitCommonUiPrefixBeforeTechnicalFile(next);

  const protectedTokens: string[] = [];
  next = next.replace(technicalFileTokenPattern, (token) => {
    const marker = `\uE000${protectedTokens.length}\uE000`;
    protectedTokens.push(token);
    return marker;
  });

  next = next
    .replace(/\b[A-Za-z]{6,}\b/g, (chunk) => segmentCollapsedUiWords(chunk))
    .replace(/([a-z])([A-Z][a-z])/g, "$1 $2")
    .replace(/\bfort\s+the\b/gi, "for the")
    .replace(/\bP've\b/g, "I've")
    .replace(/\s+/g, " ")
    .trim();
  for (let index = 0; index < protectedTokens.length; index += 1) {
    next = next.replace(new RegExp(`\uE000${index}\uE000`, "g"), protectedTokens[index]);
  }
  return restoreProtectedTechnicalTerms(next);
};
