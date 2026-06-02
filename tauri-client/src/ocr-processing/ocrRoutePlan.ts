import { invoke } from "@tauri-apps/api/core";
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

export const planOcrRoutes = async (blocks: OcrBlock[]): Promise<OcrRoutePlan | null> => {
  if (!blocks.length) return null;
  if (typeof window === "undefined") return null;
  try {
    return await invoke<OcrRoutePlan>("plan_ysn_ocr_routes", {
      texts: blocks.map((block) => block.text || ""),
    });
  } catch (error) {
    console.warn("YSN OCR route plan unavailable", error);
    return null;
  }
};
