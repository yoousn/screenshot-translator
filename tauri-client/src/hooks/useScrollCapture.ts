import { useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";
import type { Rect } from "../types/screenshot";
import { getPhysicalSelection, loadPngImage } from "../utils/screenshotImage";

export type ScrollCaptureMode = "idle" | "ready" | "capturing";

interface UseScrollCaptureProps {
  rectRef: React.MutableRefObject<Rect>;
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  imageRef: React.MutableRefObject<HTMLImageElement | null>;
  triggerRender: () => void;
  resetScreenshotState: () => void;
}

const getImageDataFromImage = (image: HTMLImageElement) => {
  const canvas = document.createElement("canvas");
  canvas.width = image.width;
  canvas.height = image.height;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("Canvas unavailable");
  ctx.drawImage(image, 0, 0);
  return ctx.getImageData(0, 0, image.width, image.height);
};

const sampledRegionDiff = (
  prev: ImageData,
  next: ImageData,
  prevStartY: number,
  nextStartY: number,
  height: number,
  sampleCols = 36,
  sampleRows = 28
) => {
  const width = Math.min(prev.width, next.width);
  if (width <= 0 || height <= 0) return Number.POSITIVE_INFINITY;
  let total = 0;
  let count = 0;
  for (let row = 0; row < sampleRows; row += 1) {
    const yRatio = sampleRows === 1 ? 0 : row / (sampleRows - 1);
    const prevY = Math.min(
      prev.height - 1,
      Math.max(0, Math.round(prevStartY + yRatio * (height - 1)))
    );
    const nextY = Math.min(
      next.height - 1,
      Math.max(0, Math.round(nextStartY + yRatio * (height - 1)))
    );
    for (let col = 0; col < sampleCols; col += 1) {
      const xRatio = sampleCols === 1 ? 0 : col / (sampleCols - 1);
      const x = Math.min(width - 1, Math.max(0, Math.round(xRatio * (width - 1))));
      const prevOffset = (prevY * prev.width + x) * 4;
      const nextOffset = (nextY * next.width + x) * 4;
      total += Math.abs(prev.data[prevOffset] - next.data[nextOffset]);
      total += Math.abs(prev.data[prevOffset + 1] - next.data[nextOffset + 1]);
      total += Math.abs(prev.data[prevOffset + 2] - next.data[nextOffset + 2]);
      count += 3;
    }
  }
  return count ? total / count : Number.POSITIVE_INFINITY;
};

const findScrollOverlap = (prev: ImageData, next: ImageData) => {
  const comparableHeight = Math.min(prev.height, next.height);
  const minOverlap = Math.max(24, Math.round(comparableHeight * 0.08));
  const maxOverlap = Math.max(minOverlap, Math.round(comparableHeight * 0.58));
  const step = Math.max(4, Math.round(comparableHeight / 80));
  let bestOverlap = Math.round(comparableHeight * 0.2);
  let bestScore = Number.POSITIVE_INFINITY;
  for (let overlap = minOverlap; overlap <= maxOverlap; overlap += step) {
    const score = sampledRegionDiff(prev, next, prev.height - overlap, 0, overlap);
    if (score < bestScore) {
      bestScore = score;
      bestOverlap = overlap;
    }
  }
  return { overlap: bestOverlap, score: bestScore };
};

const isSameScrollFrame = (prev: ImageData, next: ImageData) => {
  const height = Math.min(prev.height, next.height);
  const score = sampledRegionDiff(prev, next, 0, 0, height, 40, 32);
  return score < 2;
};

const stitchScrollFrames = async (frames: string[]) => {
  if (frames.length === 0) throw new Error("未采集到滚动截图帧");
  const images = await Promise.all(frames.map((frame) => loadPngImage(frame)));
  const imageDataList = images.map(getImageDataFromImage);
  const segments: Array<{ image: HTMLImageElement; cropTop: number; drawHeight: number }> = [
    { image: images[0], cropTop: 0, drawHeight: images[0].height },
  ];

  for (let index = 1; index < images.length; index += 1) {
    const previousData = imageDataList[index - 1];
    const currentData = imageDataList[index];
    if (isSameScrollFrame(previousData, currentData)) continue;
    const { overlap, score } = findScrollOverlap(previousData, currentData);
    const fallbackOverlap = Math.round(images[index].height * 0.2);
    const cropTop = score <= 42 ? overlap : fallbackOverlap;
    const drawHeight = Math.max(1, images[index].height - cropTop);
    segments.push({ image: images[index], cropTop, drawHeight });
  }

  const width = Math.max(...segments.map(({ image }) => image.width));
  const height = segments.reduce((sum, segment) => sum + segment.drawHeight, 0);
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas unavailable");
  let offsetY = 0;
  segments.forEach(({ image, cropTop, drawHeight }) => {
    ctx.drawImage(image, 0, cropTop, image.width, drawHeight, 0, offsetY, image.width, drawHeight);
    offsetY += drawHeight;
  });
  return canvas.toDataURL("image/png").replace(/^data:image\/png;base64,/, "");
};

export function useScrollCapture({
  rectRef,
  canvasRef,
  imageRef,
  triggerRender,
  resetScreenshotState,
}: UseScrollCaptureProps) {
  const [isScrollCapturing, setIsScrollCapturing] = useState(false);
  const [scrollCaptureMode, setScrollCaptureModeState] = useState<ScrollCaptureMode>("idle");
  const [scrollPreviewBase64, setScrollPreviewBase64] = useState("");

  const isScrollCapturingRef = useRef(false);
  const scrollCaptureModeRef = useRef<ScrollCaptureMode>("idle");
  const scrollFramesRef = useRef<string[]>([]);
  const scrollTimerRef = useRef<number | null>(null);
  const isScrollFramePendingRef = useRef(false);

  const setScrollCaptureMode = (mode: ScrollCaptureMode) => {
    scrollCaptureModeRef.current = mode;
    setScrollCaptureModeState(mode);
  };

  const getCurrentPhysicalSelection = () => getPhysicalSelection({
    canvas: canvasRef.current,
    image: imageRef.current as any,
    rect: rectRef.current,
  });

  const captureManualScrollFrame = async () => {
    if (isScrollFramePendingRef.current || scrollCaptureModeRef.current !== "capturing") return;
    const selection = getCurrentPhysicalSelection();
    if (selection.w <= 0 || selection.h <= 0) return;
    try {
      isScrollFramePendingRef.current = true;
      const frame = await invoke<string>("capture_live_region", {
        x: Math.round(selection.x),
        y: Math.round(selection.y),
        w: Math.round(selection.w),
        h: Math.round(selection.h),
      });
      const frames = scrollFramesRef.current;
      if (frames.length === 0) {
        scrollFramesRef.current = [frame];
        setScrollPreviewBase64(frame);
      } else {
        const [prev, next] = await Promise.all([
          loadPngImage(frames[frames.length - 1]),
          loadPngImage(frame),
        ]);
        const diff = sampledRegionDiff(
          getImageDataFromImage(prev),
          getImageDataFromImage(next),
          0,
          0,
          Math.min(prev.height, next.height),
          24,
          18
        );
        if (diff > 1.2) {
          scrollFramesRef.current = [...frames, frame];
          setScrollPreviewBase64(frame);
        }
      }
      message.loading({
        content: `手动滚动采集中，已采集 ${scrollFramesRef.current.length} 帧`,
        key: "scroll-shot",
        duration: 0,
      });
      if (scrollFramesRef.current.length >= 30) await finishManualScrollCapture();
    } catch (error: any) {
      message.error({
        content: `采集滚动帧失败：${error?.message || error}`,
        key: "scroll-shot",
        duration: 3,
      });
    } finally {
      isScrollFramePendingRef.current = false;
    }
  };

  const handleScrollCapture = (hasSelected: boolean) => {
    if (!hasSelected || isScrollCapturingRef.current || scrollCaptureModeRef.current !== "idle") return;
    setScrollCaptureMode("ready");
    setScrollPreviewBase64("");
    message.info("已进入滚动截图模式，请点击“开始采集”后手动滚动目标窗口。 ");
    triggerRender();
  };

  const scrollSelectedRegionDown = async () => {
    const selection = getCurrentPhysicalSelection();
    if (selection.w <= 0 || selection.h <= 0) return;
    await invoke("scroll_mouse_at", {
      x: Math.round(selection.x + selection.w / 2),
      y: Math.round(selection.y + selection.h / 2),
      delta: -520,
    }).catch(() => {});
  };

  const startManualScrollCapture = async () => {
    if (scrollCaptureModeRef.current !== "ready") return;
    scrollFramesRef.current = [];
    setScrollPreviewBase64("");
    setScrollCaptureMode("capturing");
    isScrollCapturingRef.current = true;
    setIsScrollCapturing(true);
    await invoke("set_window_capture_excluded", {
      label: getCurrentWindow().label,
      excluded: true,
    }).catch(() => {});
    message.loading({
      content: "手动滚动采集中，请自己滚动目标窗口...",
      key: "scroll-shot",
      duration: 0,
    });
    await captureManualScrollFrame();
    await scrollSelectedRegionDown();
    scrollTimerRef.current = window.setInterval(async () => {
      await captureManualScrollFrame();
      await scrollSelectedRegionDown();
    }, 760);
    triggerRender();
  };

  const finishManualScrollCapture = async () => {
    if (scrollTimerRef.current) {
      window.clearInterval(scrollTimerRef.current);
      scrollTimerRef.current = null;
    }
    try {
      const frames = [...scrollFramesRef.current];
      if (frames.length === 0) throw new Error("还没有采集到滚动截图帧");
      const stitched = frames.length === 1 ? frames[0] : await stitchScrollFrames(frames);
      await invoke("copy_image_to_clipboard", { imageBase64: stitched });
      await emit("screenshot-captured", stitched);
      message.success({
        content: `滚动截图已复制，共 ${frames.length} 帧`,
        key: "scroll-shot",
        duration: 3,
      });
    } catch (error: any) {
      message.error({
        content: `滚动截图失败：${error?.message || error}`,
        key: "scroll-shot",
        duration: 4,
      });
    } finally {
      await invoke("set_window_capture_excluded", {
        label: getCurrentWindow().label,
        excluded: false,
      }).catch(() => {});
      scrollFramesRef.current = [];
      setScrollPreviewBase64("");
      isScrollCapturingRef.current = false;
      setIsScrollCapturing(false);
      setScrollCaptureMode("idle");
      triggerRender();
    }
  };

  const cancelManualScrollCapture = () => {
    if (scrollTimerRef.current) {
      window.clearInterval(scrollTimerRef.current);
      scrollTimerRef.current = null;
    }
    scrollFramesRef.current = [];
    setScrollPreviewBase64("");
    isScrollCapturingRef.current = false;
    setIsScrollCapturing(false);
    setScrollCaptureMode("idle");
    message.destroy("scroll-shot");
    message.info("已取消滚动截图");
    triggerRender();
  };

  const clearScrollCaptureState = () => {
    if (scrollTimerRef.current) {
      window.clearInterval(scrollTimerRef.current);
      scrollTimerRef.current = null;
    }
    scrollFramesRef.current = [];
    setScrollPreviewBase64("");
    isScrollCapturingRef.current = false;
    setIsScrollCapturing(false);
    setScrollCaptureMode("idle");
  };

  return {
    isScrollCapturing,
    scrollCaptureMode,
    scrollPreviewBase64,
    isScrollCapturingRef,
    scrollCaptureModeRef,
    scrollFramesRef,
    scrollTimerRef,
    handleScrollCapture,
    startManualScrollCapture,
    finishManualScrollCapture,
    cancelManualScrollCapture,
    clearScrollCaptureState,
  };
}
