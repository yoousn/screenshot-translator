import type { OcrBlock } from "../types/screenshot";

export type OcrRouteCandidate = {
  modelId: string;
  scripts?: string[];
  languages?: string[];
  profile?: string;
  sourceProvider?: string;
};

export type OcrLineRoute = {
  index: number;
  textSample: string;
  dominantScript: string;
  scriptCounts: Record<string, number>;
  routeReason: string;
  candidateModels: OcrRouteCandidate[];
  needsFallback: boolean;
};

export type OcrRoutePlan = {
  sourceLanguage: "auto";
  lineCount: number;
  routes: OcrLineRoute[];
  missingScripts: string[];
  policy: string;
};

const countScripts = (text: string) => {
  const counts: Record<string, number> = {};
  for (const char of text) {
    let script = "unknown";
    if (/[A-Za-z]/.test(char)) script = "latin";
    else if (/[\u4e00-\u9fff\u3040-\u30ff]/.test(char)) script = "cjk";
    else if (/[\uac00-\ud7af]/.test(char)) script = "hangul";
    else if (/[\u0400-\u04ff]/.test(char)) script = "cyrillic";
    else if (/[\u0600-\u06ff]/.test(char)) script = "arabic";
    else if (/[\u0e00-\u0e7f]/.test(char)) script = "thai";
    if (script !== "unknown") counts[script] = (counts[script] || 0) + 1;
  }
  return counts;
};

const dominantScript = (counts: Record<string, number>) => {
  const entries = Object.entries(counts).sort((left, right) => right[1] - left[1]);
  return entries[0]?.[0] || "unknown";
};

export const planOcrRoutes = async (blocks: OcrBlock[]): Promise<OcrRoutePlan | null> => {
  if (!blocks.length) return null;
  const routes = blocks.map((block, index) => {
    const scriptCounts = countScripts(block.text || "");
    const script = dominantScript(scriptCounts);
    return {
      index,
      textSample: (block.text || "").slice(0, 80),
      dominantScript: script,
      scriptCounts,
      routeReason: "rapidocr-auto",
      candidateModels: [
        {
          modelId: "rapidocr-v5",
          scripts: script === "unknown" ? ["mixed"] : [script],
          languages: ["auto"],
          profile: "balanced",
          sourceProvider: "RapidOCR",
        },
      ],
      needsFallback: false,
    };
  });
  const missingScripts = Array.from(new Set(routes.filter((route) => route.dominantScript === "unknown").map((route) => route.textSample)));
  return {
    sourceLanguage: "auto",
    lineCount: routes.length,
    routes,
    missingScripts,
    policy: "rapidocr-auto-v5-primary-v4-selectable",
  };
};
