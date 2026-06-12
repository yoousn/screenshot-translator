import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";
import type { Annotation, NativeWgcSelectedOutputClipboardAcceptanceRequest, NativeWgcSelectedOutputClipboardAcceptanceResponse, Rect, ScreenshotPhysicalBounds } from "../types/screenshot";
import { cropSelectionFromLoadedImage, getDesktopPhysicalSelection, getPhysicalSelection, renderEditedSelectionBase64 } from "../utils/screenshotImage";
import { openPinWindow } from "../utils/pinWindows";
import { logScreenshotPerf } from "../utils/debugLog";

const MIN_SELECTION_CONFIRM_AGE_MS = 120;
const WGC_SELECTED_OUTPUT_COPY_CANDIDATE_ENABLED = import.meta.env.VITE_YSN_WGC_SELECTED_OUTPUT_COPY_CANDIDATE === "1";
const WGC_SELECTED_OUTPUT_SAVE_CANDIDATE_ENABLED = import.meta.env.VITE_YSN_WGC_SELECTED_OUTPUT_SAVE_CANDIDATE === "1";

const logSaveBaseline = (phase: string, elapsedMs: number, detail = "") => {
  logScreenshotPerf(`[baseline] session=save-as phase=${phase} elapsed_ms=${Math.round(elapsedMs)} ${detail}`);
};

type NativeSelectedImageBridgeResponse = {
  pngBase64?: unknown;
  diagnostics?: {
    pngSignatureValid?: unknown;
    selectedOnlyPng?: unknown;
    isValidBridge?: unknown;
  };
};

type NativeSelectedImageBridgeSuccess = NativeSelectedImageBridgeResponse & {
  pngBase64: string;
};

type SelectedImageBridgeAction = "copy" | "save" | "ocr" | "translate";

interface UseScreenshotActionsProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  imageRef: React.RefObject<HTMLImageElement | HTMLCanvasElement | null>;
  displayedSessionIdRef: React.RefObject<string | null>;
  displayedPhysicalBoundsRef: React.RefObject<ScreenshotPhysicalBounds | null>;
  rectRef: React.RefObject<Rect>;
  rect: Rect;
  hasSelected: boolean;
  translatedResult: string | null;
  annotationsRef: React.RefObject<Annotation[]>;
  annotationColorRef: React.RefObject<string>;
  annotationSizeRef: React.RefObject<number>;
  overlayVisibleRef: React.RefObject<boolean>;
  selectionCompletedAtRef: React.RefObject<number>;
  pendingConfirmTimerRef: React.RefObject<number | null>;
  recordingSegmentsRef: React.RefObject<string[]>;
  interactionStateRef: React.RefObject<{
    hasSelected: boolean;
    isSelecting: boolean;
    isDragging: boolean;
    isResizing: boolean;
  }>;
  annotationStateRef: React.RefObject<{
    isDrawing: boolean;
    isDragging: boolean;
    isResizing: boolean;
  }>;
  resetScreenshotState: () => void;
  cancelScreenshot: () => void;
}

export function useScreenshotActions({
  canvasRef,
  imageRef,
  displayedSessionIdRef,
  displayedPhysicalBoundsRef,
  rectRef,
  rect,
  hasSelected,
  translatedResult,
  annotationsRef,
  annotationColorRef,
  annotationSizeRef,
  overlayVisibleRef,
  selectionCompletedAtRef,
  pendingConfirmTimerRef,
  recordingSegmentsRef,
  interactionStateRef,
  annotationStateRef,
  resetScreenshotState,
  cancelScreenshot,
}: UseScreenshotActionsProps) {
  const outputCacheRef = React.useRef<{ signature: string; base64: string } | null>(null);
  const outputWarmupTimerRef = React.useRef<number | null>(null);

  const getOutputSignature = () => JSON.stringify({
    rect: rectRef.current,
    translatedResult: translatedResult || "",
    annotations: annotationsRef.current,
    fallbackColor: annotationColorRef.current,
    fallbackSize: annotationSizeRef.current,
  });

  const cropCurrentSelectionFromLoadedImage = () => cropSelectionFromLoadedImage({
    canvas: canvasRef.current,
    image: imageRef.current as any,
    rect: rectRef.current,
  });

  const canUseNativeSelectedImageBridge = () => {
    const image = imageRef.current;
    return image instanceof HTMLCanvasElement;
  };

  const isValidNativeSelectedImageBridge = (result: NativeSelectedImageBridgeResponse | null | undefined): result is NativeSelectedImageBridgeSuccess => (
    result?.diagnostics?.isValidBridge === true
    && result.diagnostics.selectedOnlyPng === true
    && result.diagnostics.pngSignatureValid === true
    && typeof result.pngBase64 === "string"
    && result.pngBase64.length > 0
  );

  const buildNativeSelectedImageBase64 = async (action: SelectedImageBridgeAction): Promise<string | null> => {
    if (!canUseNativeSelectedImageBridge()) return null;

    const { x, y, w, h } = getPhysicalSelection({
      canvas: canvasRef.current,
      image: imageRef.current as any,
      rect: rectRef.current,
    });
    if (w <= 0 || h <= 0) return null;

    try {
      const result = await invoke<NativeSelectedImageBridgeResponse>("build_native_selected_image_bridge", {
        request: { action, x, y, width: w, height: h, sessionId: displayedSessionIdRef.current },
      });
      if (isValidNativeSelectedImageBridge(result)) {
        logScreenshotPerf(`native selected-image bridge hit action=${action} bytes=${Math.round(result.pngBase64.length * 0.75)}`);
        return result.pngBase64;
      }
      console.warn("[ScreenshotPage] native selected-image bridge returned invalid diagnostics, falling back", result?.diagnostics);
    } catch (error) {
      console.warn("[ScreenshotPage] native selected-image bridge unavailable, falling back", error);
    }
    return null;
  };

  const runGuardedWgcExplicitSelectionDiagnostic = async (
    options: Partial<Omit<NativeWgcSelectedOutputClipboardAcceptanceRequest, "bounds">> = {}
  ): Promise<NativeWgcSelectedOutputClipboardAcceptanceResponse | null> => {
    const currentHasSelected = hasSelected || interactionStateRef.current.hasSelected;
    if (!overlayVisibleRef.current || !currentHasSelected || rectRef.current.w <= 0 || rectRef.current.h <= 0) return null;
    const bounds = getDesktopPhysicalSelection({
      canvas: canvasRef.current,
      image: imageRef.current as any,
      rect: rectRef.current,
      physicalBounds: displayedPhysicalBoundsRef.current,
    });
    const request: NativeWgcSelectedOutputClipboardAcceptanceRequest = {
      bounds: { ...bounds, explicitOptIn: true, allowRealDxgiApi: false },
      explicitOptIn: options.explicitOptIn ?? true,
      allowRealWgcApi: options.allowRealWgcApi ?? true,
      allowFakeClipboardSink: options.allowFakeClipboardSink ?? true,
      allowRealClipboard: options.allowRealClipboard ?? false,
      frameTimeoutMs: options.frameTimeoutMs ?? 500,
      includeCursor: options.includeCursor ?? false,
      requireBorder: options.requireBorder ?? false,
      bufferCount: options.bufferCount ?? 1,
      validateTarget: options.validateTarget ?? true,
      includeSelectedPngBase64: options.includeSelectedPngBase64 ?? false,
      allowFileWrite: options.allowFileWrite ?? false,
      savePath: options.savePath,
    };
    try {
      const response = await invoke<NativeWgcSelectedOutputClipboardAcceptanceResponse>(
        "run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke",
        { request }
      );
      logScreenshotPerf(`wgc explicit-selection diagnostic ok=${response?.ok === true} attempted=${response?.attempted === true} stage=${response?.stage || "unknown"}`);
      return response;
    } catch (error) {
      console.warn("[ScreenshotPage] WGC explicit-selection diagnostic failed", error);
      logScreenshotPerf(`wgc explicit-selection diagnostic invoke failed error=${error instanceof Error ? error.message : String(error)}`);
      return null;
    }
  };

  const canTryWgcSelectedOutputCandidate = (action: "copy" | "save") => (
    (action === "copy" ? WGC_SELECTED_OUTPUT_COPY_CANDIDATE_ENABLED : WGC_SELECTED_OUTPUT_SAVE_CANDIDATE_ENABLED)
    && overlayVisibleRef.current
    && (hasSelected || interactionStateRef.current.hasSelected)
    && rectRef.current.w > 0
    && rectRef.current.h > 0
    && annotationsRef.current.length === 0
    && !translatedResult
  );

  const tryWgcSelectedOutputBase64Candidate = async (action: "copy" | "save"): Promise<string | null> => {
    if (!canTryWgcSelectedOutputCandidate(action)) return null;
    const writesRealClipboard = action === "copy";
    const response = await runGuardedWgcExplicitSelectionDiagnostic({
      explicitOptIn: true,
      allowRealWgcApi: true,
      allowFakeClipboardSink: !writesRealClipboard,
      allowRealClipboard: writesRealClipboard,
      includeSelectedPngBase64: true,
      frameTimeoutMs: 500,
      includeCursor: false,
      requireBorder: false,
      bufferCount: 1,
      validateTarget: true,
    });
    const clipboardAccepted = writesRealClipboard
      ? response?.realClipboardAttempted === true && response.realClipboardVerified === true
      : response?.realClipboardAttempted === false;
    if (
      response?.ok === true
      && response.selectedOutputEffectConfirmed === true
      && clipboardAccepted
      && typeof response.selectedPngBase64 === "string"
      && response.selectedPngBase64.length > 0
    ) {
      logScreenshotPerf(`wgc selected-output ${action} candidate hit bytes=${Math.round(response.selectedPngBase64.length * 0.75)}`);
      return response.selectedPngBase64;
    }
    if (response) {
      logScreenshotPerf(`wgc selected-output ${action} candidate fallback ok=${response.ok === true} realClipboardVerified=${response.realClipboardVerified === true} stage=${response.stage || "unknown"}`);
    }
    return null;
  };

  const captureRegionBase64 = async (action: SelectedImageBridgeAction = "ocr"): Promise<string> => {
    const { x, y, w, h } = getPhysicalSelection({
      canvas: canvasRef.current,
      image: imageRef.current as any,
      rect: rectRef.current,
    });
    const nativeBase64 = await buildNativeSelectedImageBase64(action);
    if (nativeBase64) return nativeBase64;
    try {
      const cropped = cropCurrentSelectionFromLoadedImage();
      if (cropped.base64) return cropped.base64;
    } catch (error) {
      console.warn("[ScreenshotPage] client-side crop failed, falling back to Rust crop", error);
    }
    return await invoke<string>("capture_region", { x, y, w, h });
  };

  const renderCurrentEditedSelectionBase64 = async () => renderEditedSelectionBase64({
    canvas: canvasRef.current,
    image: imageRef.current as any,
    rect: rectRef.current,
    translatedResult,
    annotations: annotationsRef.current,
    fallbackColor: annotationColorRef.current,
    fallbackSize: annotationSizeRef.current,
  });

  const buildOutputBase64 = async (action: SelectedImageBridgeAction = "copy"): Promise<string> => (
    annotationsRef.current.length > 0 ? await renderCurrentEditedSelectionBase64() : (translatedResult || await captureRegionBase64(action))
  );

  const getOutputBase64 = async (action: SelectedImageBridgeAction = "copy"): Promise<string> => {
    const signature = getOutputSignature();
    if (outputCacheRef.current?.signature === signature) {
      logScreenshotPerf("output cache hit");
      return outputCacheRef.current.base64;
    }
    const startedAt = performance.now();
    const base64 = await buildOutputBase64(action);
    outputCacheRef.current = { signature, base64 };
    logScreenshotPerf(`output built ${Math.round(performance.now() - startedAt)}ms bytes=${Math.round(base64.length * 0.75)}`);
    return base64;
  };

  React.useEffect(() => {
    if (outputWarmupTimerRef.current !== null) {
      window.clearTimeout(outputWarmupTimerRef.current);
      outputWarmupTimerRef.current = null;
    }
    outputCacheRef.current = null;
    if (!overlayVisibleRef.current || !hasSelected || rect.w <= 5 || rect.h <= 5 || !imageRef.current) return;
    outputWarmupTimerRef.current = window.setTimeout(() => {
      outputWarmupTimerRef.current = null;
      const signature = getOutputSignature();
      const startedAt = performance.now();
      buildOutputBase64("copy")
        .then((base64) => {
          outputCacheRef.current = { signature, base64 };
          logScreenshotPerf(`output warmed ${Math.round(performance.now() - startedAt)}ms bytes=${Math.round(base64.length * 0.75)}`);
        })
        .catch((error) => console.warn("[screenshot-perf] output warmup failed", error));
    }, 24);
    return () => {
      if (outputWarmupTimerRef.current !== null) {
        window.clearTimeout(outputWarmupTimerRef.current);
        outputWarmupTimerRef.current = null;
      }
    };
  }, [hasSelected, rect.x, rect.y, rect.w, rect.h, translatedResult, overlayVisibleRef.current]);

  const getSelectionConfirmDelayMs = (minAgeMs = MIN_SELECTION_CONFIRM_AGE_MS) => {
    if (
      !overlayVisibleRef.current
      || !hasSelected
      || rectRef.current.w <= 5
      || rectRef.current.h <= 5
      || interactionStateRef.current.isSelecting
      || interactionStateRef.current.isDragging
      || interactionStateRef.current.isResizing
      || annotationStateRef.current.isDrawing
      || annotationStateRef.current.isDragging
      || annotationStateRef.current.isResizing
    ) {
      return null;
    }
    const ageMs = performance.now() - selectionCompletedAtRef.current;
    return Math.max(0, minAgeMs - ageMs);
  };

  const canConfirmCurrentSelection = (minAgeMs = MIN_SELECTION_CONFIRM_AGE_MS) => (
    getSelectionConfirmDelayMs(minAgeMs) === 0
  );

  const handlePin = async () => {
    if (!hasSelected || rect.w <= 0 || rect.h <= 0) return;
    const { base64, x, y, w, h } = cropCurrentSelectionFromLoadedImage();
    if (!base64) return;

    try {
      await openPinWindow(await getOutputBase64("copy"), { x, y, w, h });
      cancelScreenshot();
    } catch (error) {
      console.error("Failed to create pin window", error);
      message.error("钉图失败");
    }
  };

  const forceCloseScreenshots = async () => {
    message.destroy();
    const segments = [...recordingSegmentsRef.current];
    invoke("cancel_recording_process").catch(() => {});
    if (segments.length > 0) invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
    await invoke("force_close_screenshots").catch(() => {});
    resetScreenshotState();
  };

  const confirmScreenshot = async (action: "copy" | "save" | "both") => {
    const actionStartedAt = performance.now();
    const screenshotWindow = action === "save" ? getCurrentWindow() : null;
    if (action === "save") logSaveBaseline("save_invoked", 0, `action=${action}`);
    const confirmDelayMs = getSelectionConfirmDelayMs(action === "save" ? 0 : MIN_SELECTION_CONFIRM_AGE_MS);
    if (confirmDelayMs === null) return;
    if (confirmDelayMs > 0) {
      if (pendingConfirmTimerRef.current !== null) {
        window.clearTimeout(pendingConfirmTimerRef.current);
      }
      pendingConfirmTimerRef.current = window.setTimeout(() => {
        pendingConfirmTimerRef.current = null;
        confirmScreenshot(action);
      }, confirmDelayMs + 16);
      return;
    }
    if (pendingConfirmTimerRef.current !== null) {
      window.clearTimeout(pendingConfirmTimerRef.current);
      pendingConfirmTimerRef.current = null;
    }
    try {
      if (action === "save") {
        const chooseStartedAt = performance.now();
        logSaveBaseline("dialog_open_start", chooseStartedAt - actionStartedAt);
        await screenshotWindow?.setAlwaysOnTop(false).catch(() => {});
        const savePath = await invoke<string | null>("choose_image_save_path");
        const chooseMs = Math.round(performance.now() - chooseStartedAt);
        logSaveBaseline("dialog_open_end", performance.now() - actionStartedAt, `choose_ms=${chooseMs} cancelled=${!savePath}`);
        if (!savePath) {
          message.destroy();
          await invoke("force_close_screenshots").catch(() => {});
          resetScreenshotState();
          logSaveBaseline("dialog_cancel_exit", performance.now() - actionStartedAt);
          return;
        }
        const outputStartedAt = performance.now();
        logSaveBaseline("output_render_start", outputStartedAt - actionStartedAt);
        const base64 = await tryWgcSelectedOutputBase64Candidate("save") ?? await getOutputBase64("save");
        const outputMs = Math.round(performance.now() - outputStartedAt);
        logSaveBaseline("output_render_end", performance.now() - actionStartedAt, `output_ms=${outputMs} bytes=${Math.round(base64.length * 0.75)}`);
        const writeStartedAt = performance.now();
        logSaveBaseline("file_write_start", writeStartedAt - actionStartedAt, `path=${savePath}`);
        const savedPath = await invoke<string>("write_image_to_file", { imageBase64: base64, path: savePath });
        const writeMs = Math.round(performance.now() - writeStartedAt);
        logSaveBaseline("file_write_end", performance.now() - actionStartedAt, `write_ms=${writeMs} path=${savedPath}`);
        logScreenshotPerf(`save-as total=${Math.round(performance.now() - actionStartedAt)}ms choose=${chooseMs}ms output=${outputMs}ms write=${writeMs}ms path=${savedPath}`);
        await emit("screenshot-captured", base64);
        message.destroy();
        await invoke("cancel_screenshot", { label: screenshotWindow?.label || getCurrentWindow().label, restoreMain: false });
        resetScreenshotState();
        logSaveBaseline("overlay_exit_after_save", performance.now() - actionStartedAt);
        return;
      }
      const wgcCopiedBase64 = action === "copy" ? await tryWgcSelectedOutputBase64Candidate("copy") : null;
      const base64 = wgcCopiedBase64 ?? await getOutputBase64(action === "both" ? "copy" : action);
      await emit("screenshot-captured", base64);
      if ((action === "copy" || action === "both") && !wgcCopiedBase64) {
        await invoke("copy_image_to_clipboard", { imageBase64: base64 });
      }
      message.destroy();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label });
      resetScreenshotState();

    } catch (e: any) {
      if (action === "save") {
        await screenshotWindow?.setAlwaysOnTop(true).catch(() => {});
        await screenshotWindow?.setFocus().catch(() => {});
      }
      message.error(`\u622a\u56fe\u64cd\u4f5c\u5931\u8d25\uff1a${e?.message || e?.toString?.() || e}`);
    }
  };

  return {
    cropCurrentSelectionFromLoadedImage,
    captureRegionBase64,
    renderCurrentEditedSelectionBase64,
    getOutputBase64,
    runGuardedWgcExplicitSelectionDiagnostic,
    getSelectionConfirmDelayMs,
    canConfirmCurrentSelection,
    handlePin,
    forceCloseScreenshots,
    confirmScreenshot,
  };
}
