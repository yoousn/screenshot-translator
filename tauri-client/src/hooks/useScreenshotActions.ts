import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";
import type { Rect, Annotation } from "../types/screenshot";
import { cropSelectionFromLoadedImage, getPhysicalSelection, renderEditedSelectionBase64 } from "../utils/screenshotImage";
import { openPinWindow } from "../utils/pinWindows";

const MIN_SELECTION_CONFIRM_AGE_MS = 120;

const logScreenshotPerf = (messageText: string) => {
  invoke("log_screenshot_perf", { message: messageText }).catch(() => {});
};

const logSaveBaseline = (phase: string, elapsedMs: number, detail = "") => {
  logScreenshotPerf(`[baseline] session=save-as phase=${phase} elapsed_ms=${Math.round(elapsedMs)} ${detail}`);
};

interface UseScreenshotActionsProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  imageRef: React.RefObject<HTMLImageElement | null>;
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

  const captureRegionBase64 = async () => {
    const { x, y, w, h } = getPhysicalSelection({
      canvas: canvasRef.current,
      image: imageRef.current as any,
      rect: rectRef.current,
    });
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

  const buildOutputBase64 = async () => (
    annotationsRef.current.length > 0 ? await renderCurrentEditedSelectionBase64() : (translatedResult || await captureRegionBase64())
  );

  const getOutputBase64 = async () => {
    const signature = getOutputSignature();
    if (outputCacheRef.current?.signature === signature) {
      logScreenshotPerf("output cache hit");
      return outputCacheRef.current.base64;
    }
    const startedAt = performance.now();
    const base64 = await buildOutputBase64();
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
      buildOutputBase64()
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
      await openPinWindow(await getOutputBase64(), { x, y, w, h });
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
    resetScreenshotState();
    await invoke("force_close_screenshots").catch(() => {});
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
          await screenshotWindow?.setAlwaysOnTop(true).catch(() => {});
          await screenshotWindow?.setFocus().catch(() => {});
          return;
        }
        const outputStartedAt = performance.now();
        logSaveBaseline("output_render_start", outputStartedAt - actionStartedAt);
        const base64 = await getOutputBase64();
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
        resetScreenshotState();
        await invoke("cancel_screenshot", { label: screenshotWindow?.label || getCurrentWindow().label, restoreMain: false });
        logSaveBaseline("overlay_exit_after_save", performance.now() - actionStartedAt);
        return;
      }
      const base64 = await getOutputBase64();
      await emit("screenshot-captured", base64);
      if (action === "copy" || action === "both") {
        await invoke("copy_image_to_clipboard", { imageBase64: base64 });
      }
      message.destroy();
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label });

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
    getSelectionConfirmDelayMs,
    canConfirmCurrentSelection,
    handlePin,
    forceCloseScreenshots,
    confirmScreenshot,
  };
}
