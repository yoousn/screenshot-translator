import { useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Config } from "../types/config";
import type { Rect, TranslatePair, OcrBlock } from "../types/screenshot";
import { openOcrResultWindow } from "../utils/ocrResultWindow";
import { FLOATING_PANEL_MARGIN, FLOATING_PANEL_GAP, OCR_WINDOW_SIZE } from "../utils/screenshotLayout";
import { buildOcrNormalizationReport } from "../ocr-processing";
import { prewarmTranslationServices, translateOcrBlocks, translateWithLocalOcr } from "../utils/localOcrTranslate";
import { loadPngImage } from "../utils/screenshotImage";

export interface ScreenshotOcrDeps {
  config: Config;
  rectRef: React.MutableRefObject<Rect>;
  captureRegionBase64: () => Promise<string>;
  resetScreenshotState: () => void;
  draw: (x: number, y: number, w: number, h: number, img: HTMLImageElement | HTMLCanvasElement | null) => void;
  translatedImgRef: React.MutableRefObject<HTMLImageElement | null>;
  getTextSourceBlocksForCurrentSelection: (maxWaitMs?: number) => Promise<{ usable: boolean; blocks: OcrBlock[]; elapsedMs: number }>;
}

export function useScreenshotOcr(deps: ScreenshotOcrDeps) {
  const {
    config,
    rectRef,
    captureRegionBase64,
    resetScreenshotState,
    draw,
    translatedImgRef,
    getTextSourceBlocksForCurrentSelection,
  } = deps;

  const configRef = useRef(config);
  configRef.current = config;

  const [isOCRing, setIsOCRing] = useState(false);
  const [isTranslating, setIsTranslating] = useState(false);
  const [translatePairs, setTranslatePairs] = useState<TranslatePair[] | null>(null);
  const [translatedResult, setTranslatedResult] = useState<string | null>(null);
  const [translateResultPreviewBase64, setTranslateResultPreviewBase64] = useState<string | null>(null);

  const isOCRingRef = useRef(false);
  const isTranslatingRef = useRef(false);
  const ocrPrewarmPromiseRef = useRef<Promise<any> | null>(null);

  // Sync refs to match useState
  const setIsOCRingSync = useCallback((val: boolean) => {
    isOCRingRef.current = val;
    setIsOCRing(val);
  }, []);

  const setIsTranslatingSync = useCallback((val: boolean) => {
    isTranslatingRef.current = val;
    setIsTranslating(val);
  }, []);

  const normalizeScreenshotTranslateError = useCallback((error: any) => {
    const raw = error?.message || error?.toString?.() || String(error || "");
    if (/\u672a\u8bc6\u522b\u5230\u6587\u5b57|did not recognize text|recognized no text|no text/i.test(raw)) {
      return "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u672a\u8bc6\u522b\u5230\u6587\u5b57\u3002\u8bf7\u91cd\u65b0\u6846\u9009\u66f4\u6e05\u6670\u3001\u66f4\u5b8c\u6574\u7684\u6587\u5b57\u533a\u57df\u3002";
    }
    return raw
      .replace(/YSN OCR Runtime/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
      .replace(/PP-OCRv5\s*ONNX\s*OCR/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
      .replace(/PP-OCRv5/gi, "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1")
      .replace(/ONNX/gi, "\u672c\u5730\u6a21\u578b")
      .trim() || "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6682\u4e0d\u53ef\u7528\uff0c\u8bf7\u91cd\u65b0\u6846\u9009\u6587\u5b57\u533a\u57df\u540e\u518d\u8bd5\u3002";
  }, []);

  const prewarmLocalOcrWorker = useCallback((reason: string) => {
    if (configRef.current.rapidOcrWorkerEnabled === false) return null;
    if (ocrPrewarmPromiseRef.current) return ocrPrewarmPromiseRef.current;

    const promise = invoke("prewarm_local_ocr_models")
      .then((result) => {
        console.info("[RapidOCR Worker Prewarm]", reason, result);
        return result;
      })
      .catch((error) => {
        console.warn("[RapidOCR Worker Prewarm] failed", reason, error);
        return null;
      })
      .finally(() => {
        window.setTimeout(() => {
          if (ocrPrewarmPromiseRef.current === promise) {
            ocrPrewarmPromiseRef.current = null;
          }
        }, 5000);
      });
    ocrPrewarmPromiseRef.current = promise;
    return promise;
  }, []);

  const handleOCR = useCallback(async () => {
    if (isOCRingRef.current || isTranslatingRef.current) return;
    let base64 = "";
    try {
      setIsOCRingSync(true);
      message.loading({ content: "\u6b63\u5728\u8bc6\u522b\u6587\u5b57...", key: "ocr", duration: 0 });

      base64 = await captureRegionBase64();
      const ocrBlocks: OcrBlock[] = await invoke("run_local_ocr", {
        imageBase64: base64,
        executablePath: null,
        timeoutMs: configRef.current.localOcrTimeoutMs || 15000
      });
      const normalization = await buildOcrNormalizationReport(ocrBlocks || []);
      const texts = normalization.text || "\u672a\u8bc6\u522b\u5230\u6587\u5b57\u3002\n\n\u8bf7\u91cd\u65b0\u6846\u9009\u66f4\u6e05\u6670\u3001\u66f4\u5b8c\u6574\u7684\u6587\u5b57\u533a\u57df\u3002";

      message.destroy();
      setIsOCRingSync(false);

      if (texts) {
        try {
          await navigator.clipboard.writeText(texts);
        } catch {}
      }

      await openOcrResultWindow({
        selection: rectRef.current,
        text: texts,
        previewBase64: base64,
        margin: FLOATING_PANEL_MARGIN,
        gap: FLOATING_PANEL_GAP,
        windowSize: OCR_WINDOW_SIZE,
        normalizationSummary: {
          rawCount: normalization.rawCount,
          usefulCount: normalization.usefulCount,
          virtualLineCount: normalization.virtualLineCount,
          droppedCount: normalization.droppedCount,
          routeMissingScripts: normalization.routePlan?.missingScripts || [],
        },
      });
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
    } catch (e: any) {
      const msg = normalizeScreenshotTranslateError(e);
      message.error({ content: `\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u5931\u8d25\uff1a${msg}`, key: "ocr", duration: 3 });
      setIsOCRingSync(false);
      if (base64) {
        await openOcrResultWindow({
          selection: rectRef.current,
          text: `\u8bc6\u522b\u6682\u4e0d\u53ef\u7528\u3002\n\n${msg}\n\n\u5f53\u524d\u5df2\u7ecf\u68c0\u67e5\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6a21\u578b\u3002`,
          previewBase64: base64,
          margin: FLOATING_PANEL_MARGIN,
          gap: FLOATING_PANEL_GAP,
          windowSize: OCR_WINDOW_SIZE,
          title: "\u8bc6\u522b\u72b6\u6001",
        });
        resetScreenshotState();
        await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
      }
    }
  }, [captureRegionBase64, normalizeScreenshotTranslateError, rectRef, resetScreenshotState, setIsOCRingSync]);

  const handleTranslate = useCallback(async () => {
    if (isTranslatingRef.current || isOCRingRef.current) return;
    const startTime = performance.now();
    let base64 = "";
    prewarmLocalOcrWorker("translate-action");
    prewarmTranslationServices(configRef.current, { reason: "translate-action" })
      .catch((error) => {
        console.warn("[Translation Service Prewarm] failed", error);
        return null;
      });
    try {
      setIsTranslatingSync(true);
      message.loading({ content: "\u6b63\u5728\u8bc6\u522b\u5e76\u7ffb\u8bd1...", key: "translate", duration: 0 });
      const captureStarted = performance.now();
      base64 = await captureRegionBase64();
      const captureMs = Math.round(performance.now() - captureStarted);

      let resultBase64 = "";
      let usedChannel = configRef.current.channel || configRef.current.targetLang || "auto";
      let blocksCount = 1;
      let translationQuality: Awaited<ReturnType<typeof translateWithLocalOcr>>["translationQuality"] | null = null;
      try {
        const localFlowStarted = performance.now();
        const textSource = await getTextSourceBlocksForCurrentSelection(80);
        const result = textSource.usable
          ? await translateOcrBlocks(base64, textSource.blocks, configRef.current, {
              flowStarted: localFlowStarted,
              ocrMs: textSource.elapsedMs,
              source: "text-source",
            })
          : await translateWithLocalOcr(base64, configRef.current);
        console.info("[Local Translate Flow] timings", {
          captureMs,
          ocrTranslateRenderMs: Math.round(performance.now() - localFlowStarted),
          totalMs: Math.round(performance.now() - startTime),
          server: result.usedServerUrl,
          channel: result.usedChannel,
          blocks: result.blocksCount,
          textSource,
          localTimings: result.localTimings,
          serverTimings: result.translationTimings,
        });
        resultBase64 = result.resultBase64;
        usedChannel = result.usedChannel;
        blocksCount = result.blocksCount;
        translationQuality = result.translationQuality;
        setTranslatePairs(result.pairs);
        setTranslateResultPreviewBase64(resultBase64);
      } catch (localErr: any) {
        console.warn("[Local Translate Flow] failed", localErr);
        throw localErr;
      }

      const overlayImg = await loadPngImage(resultBase64);
      translatedImgRef.current = overlayImg;
      draw(rectRef.current.x, rectRef.current.y, rectRef.current.w, rectRef.current.h, overlayImg);
      setTranslatedResult(resultBase64);
      if (translationQuality && translationQuality.translatableCount === 0 && translationQuality.preservedCount > 0) {
        message.info({ content: "\u5df2\u8bc6\u522b\u5230\u6587\u5b57\uff0c\u4f46\u9009\u533a\u4e3b\u8981\u662f\u6587\u4ef6\u540d\u3001\u8def\u5f84\u6216\u6280\u672f\u6807\u8bc6\uff0c\u5df2\u6309\u89c4\u5219\u4fdd\u7559\u539f\u6587\u3002", key: "translate", duration: 3 });
      } else if (translationQuality && translationQuality.untranslatedCount > 0) {
        message.warning({ content: `\u7ffb\u8bd1\u5b8c\u6210\uff0c${translationQuality.untranslatedCount} \u884c\u672a\u8fd4\u56de\u6709\u6548\u8bd1\u6587\uff0c\u53ef\u5728\u7ed3\u679c\u91cc\u67e5\u770b\u3002`, key: "translate", duration: 4 });
      } else {
        message.success({ content: "\u7ffb\u8bd1\u5b8c\u6210", key: "translate" });
      }

      try {
        const durationSec = ((performance.now() - startTime) / 1000).toFixed(2);
        const record = {
          id: "rec-" + Date.now(),
          time: new Date().toLocaleString(),
          filename: "Screenshot_" + Date.now() + ".png",
          blocks: blocksCount,
          channel: usedChannel,
          duration: durationSec + "s",
          status: "success",
        };
        const prev = localStorage.getItem("ysn_translate_history");
        const list = prev ? JSON.parse(prev) : [];
        list.unshift(record);
        if (list.length > 50) list.pop();
        localStorage.setItem("ysn_translate_history", JSON.stringify(list));
        window.dispatchEvent(new Event("ysn_translate_history_updated"));
      } catch (e) {
        console.warn("Failed to save translate history", e);
      }
      setIsTranslatingSync(false);
    } catch (e: any) {
      console.error("Local Translation Error:", e);
      const msg = normalizeScreenshotTranslateError(e);
      message.error({ content: `\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u5931\u8d25\uff1a${msg}`, key: "translate", duration: 3 });
      setIsTranslatingSync(false);
      setTranslatedResult(null);

      try {
        const durationSec = ((performance.now() - startTime) / 1000).toFixed(2);
        const record = {
          id: "rec-" + Date.now(),
          time: new Date().toLocaleString(),
          filename: "Screenshot_" + Date.now() + ".png",
          blocks: 0,
          channel: configRef.current.channel || configRef.current.targetLang || "auto",
          duration: durationSec + "s",
          status: "error",
          error: msg,
        };
        const prev = localStorage.getItem("ysn_translate_history");
        const list = prev ? JSON.parse(prev) : [];
        list.unshift(record);
        if (list.length > 50) list.pop();
        localStorage.setItem("ysn_translate_history", JSON.stringify(list));
        window.dispatchEvent(new Event("ysn_translate_history_updated"));
      } catch (err) {}
    }
  }, [captureRegionBase64, draw, getTextSourceBlocksForCurrentSelection, normalizeScreenshotTranslateError, prewarmLocalOcrWorker, rectRef, setIsTranslatingSync, translatedImgRef]);

  const handleShowTranslateResult = useCallback(async () => {
    if (!translatePairs || translatePairs.length === 0) return;
    const statusLabel = (status?: TranslatePair["status"]) => {
      if (status === "preserved") return "\u72b6\u6001\uff1a\u5df2\u6309\u6280\u672f\u6807\u8bc6\u4fdd\u7559";
      if (status === "untranslated") return "\u72b6\u6001\uff1a\u672a\u8fd4\u56de\u6709\u6548\u8bd1\u6587";
      return "\u72b6\u6001\uff1a\u5df2\u7ffb\u8bd1";
    };
    
    await openOcrResultWindow({
      selection: rectRef.current,
      text: translatePairs.map((p) => `${p.o}\n\n${statusLabel(p.status)}\n${p.t}`).join("\n\n---\n\n"),
      previewBase64: translateResultPreviewBase64 || "",
      margin: FLOATING_PANEL_MARGIN,
      gap: FLOATING_PANEL_GAP,
      windowSize: OCR_WINDOW_SIZE,
      title: "\u7ffb\u8bd1\u7ed3\u679c\u660e\u7ec6",
    });
    resetScreenshotState();
    await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
  }, [rectRef, resetScreenshotState, translatePairs, translateResultPreviewBase64]);

  const resetOcrState = useCallback(() => {
    setIsOCRingSync(false);
    setIsTranslatingSync(false);
    setTranslatePairs(null);
    setTranslatedResult(null);
    setTranslateResultPreviewBase64(null);
  }, [setIsOCRingSync, setIsTranslatingSync]);

  return {
    isOCRing,
    isTranslating,
    translatePairs,
    translatedResult,
    translateResultPreviewBase64,
    prewarmLocalOcrWorker,
    handleOCR,
    handleTranslate,
    handleShowTranslateResult,
    resetOcrState,
    setTranslatedResult,
    setTranslatePairs,
    isOCRingRef,
    isTranslatingRef,
  };
}
