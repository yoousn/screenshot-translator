import type { Annotation, Rect, ScreenshotPhysicalBounds } from "../types/screenshot";
import { renderExportAnnotations } from "./renderAnnotations";

type PhysicalSelectionInput = {
  canvas: HTMLCanvasElement | null;
  image: (HTMLImageElement | HTMLCanvasElement) | null;
  rect: Rect;
};

const clampSelectionToTarget = (
  rect: Rect,
  canvasWidth: number,
  canvasHeight: number,
  targetWidth: number,
  targetHeight: number,
) => {
  const safeCanvasWidth = Math.max(1, canvasWidth);
  const safeCanvasHeight = Math.max(1, canvasHeight);
  const safeTargetWidth = Math.max(1, targetWidth);
  const safeTargetHeight = Math.max(1, targetHeight);
  const scaleX = safeTargetWidth / safeCanvasWidth;
  const scaleY = safeTargetHeight / safeCanvasHeight;
  const left = rect.x * scaleX;
  const top = rect.y * scaleY;
  const right = (rect.x + rect.w) * scaleX;
  const bottom = (rect.y + rect.h) * scaleY;

  const x = Math.max(0, Math.min(safeTargetWidth - 1, Math.round(left)));
  const y = Math.max(0, Math.min(safeTargetHeight - 1, Math.round(top)));
  const clampedRight = Math.max(x + 1, Math.min(safeTargetWidth, Math.round(right)));
  const clampedBottom = Math.max(y + 1, Math.min(safeTargetHeight, Math.round(bottom)));

  return {
    x,
    y,
    w: Math.max(1, Math.min(safeTargetWidth - x, clampedRight - x)),
    h: Math.max(1, Math.min(safeTargetHeight - y, clampedBottom - y)),
  };
};

export const getPhysicalSelection = ({ canvas, image, rect }: PhysicalSelectionInput) => {
  if (!canvas || !image || rect.w <= 0 || rect.h <= 0) {
    throw new Error("Selection bounds are invalid");
  }
  const imageWidth = image instanceof HTMLImageElement ? image.naturalWidth : image.width;
  const imageHeight = image instanceof HTMLImageElement ? image.naturalHeight : image.height;
  return clampSelectionToTarget(rect, canvas.width, canvas.height, imageWidth, imageHeight);
};

export const getDesktopPhysicalSelection = (
  input: PhysicalSelectionInput & { physicalBounds: ScreenshotPhysicalBounds | null | undefined },
) => {
  if (!input.physicalBounds) {
    throw new Error("Screenshot physical bounds are unavailable");
  }
  const local = clampSelectionToTarget(
    input.rect,
    input.canvas?.width || window.innerWidth,
    input.canvas?.height || window.innerHeight,
    input.physicalBounds.width,
    input.physicalBounds.height,
  );
  return {
    x: input.physicalBounds.x + local.x,
    y: input.physicalBounds.y + local.y,
    width: local.w,
    height: local.h,
  };
};

export const cropSelectionFromLoadedImage = (input: PhysicalSelectionInput) => {
  if (!input.image) throw new Error("Screenshot image is unavailable");
  const { x, y, w, h } = getPhysicalSelection(input);
  const cropCanvas = document.createElement("canvas");
  cropCanvas.width = w;
  cropCanvas.height = h;
  const ctx = cropCanvas.getContext("2d");
  if (!ctx) throw new Error("Canvas is unavailable");
  ctx.drawImage(input.image, x, y, w, h, 0, 0, w, h);
  return { base64: cropCanvas.toDataURL("image/png").split(",")[1] || "", x, y, w, h };
};

export const loadPngImage = (base64: string) => new Promise<HTMLImageElement>((resolve, reject) => {
  const img = new Image();
  img.onload = () => resolve(img);
  img.onerror = reject;
  img.src = `data:image/png;base64,${base64}`;
});

type RenderEditedSelectionOptions = PhysicalSelectionInput & {
  translatedResult: string | null;
  annotations: Annotation[];
  fallbackColor: string;
  fallbackSize: number;
};

export const renderEditedSelectionBase64 = async ({
  canvas,
  image,
  rect,
  translatedResult,
  annotations,
  fallbackColor,
  fallbackSize,
}: RenderEditedSelectionOptions) => {
  if (!image) throw new Error("Screenshot image is unavailable");
  const physical = getPhysicalSelection({ canvas, image, rect });
  const cropCanvas = document.createElement("canvas");
  cropCanvas.width = physical.w;
  cropCanvas.height = physical.h;
  const ctx = cropCanvas.getContext("2d");
  if (!ctx) throw new Error("Canvas is unavailable");

  if (translatedResult) {
    const translatedImage = await loadPngImage(translatedResult);
    ctx.drawImage(translatedImage, 0, 0, physical.w, physical.h);
  } else {
    ctx.drawImage(image, physical.x, physical.y, physical.w, physical.h, 0, 0, physical.w, physical.h);
  }

  renderExportAnnotations({
    ctx,
    cropCanvas,
    annotations,
    selection: rect,
    canvasWidth: canvas?.width || window.innerWidth,
    canvasHeight: canvas?.height || window.innerHeight,
    imageWidth: image instanceof HTMLImageElement ? image.naturalWidth : image.width,
    imageHeight: image instanceof HTMLImageElement ? image.naturalHeight : image.height,
    fallbackColor,
    fallbackSize,
  });

  return cropCanvas.toDataURL("image/png").split(",")[1] || "";
};
