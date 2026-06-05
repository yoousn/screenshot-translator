import React from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";
import type { Rect, Annotation } from "../types/screenshot";
import { cropSelectionFromLoadedImage, getPhysicalSelection, renderEditedSelectionBase64 } from "../utils/screenshotImage";
import { openPinWindow } from "../utils/pinWindows";

const MIN_SELECTION_CONFIRM_AGE_MS = 120;

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

  const getOutputBase64 = async () => (
    annotationsRef.current.length > 0 ? await renderCurrentEditedSelectionBase64() : (translatedResult || await captureRegionBase64())
  );

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
    const confirmDelayMs = getSelectionConfirmDelayMs();
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
      const base64 = await getOutputBase64();
      await emit("screenshot-captured", base64);
      if (action === "copy" || action === "both") {
        await invoke("copy_image_to_clipboard", { imageBase64: base64 });
      }
      message.destroy();
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label });
      if (action === "save") {
        try {
          await invoke<string>("save_image_to_file", { imageBase64: base64 });
        } catch (saveErr: any) {
          if (saveErr !== "用户取消了保存") {
            message.error("保存失败：" + (saveErr.message || saveErr.toString()));
          }
        }
      }
    } catch (e: any) {
      message.error("截图操作失败：" + (e.message || e.toString()));
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
