import type { Annotation, Rect } from "../types/screenshot";
import { renderExportAnnotations } from "./renderAnnotations";

type PhysicalSelectionInput = {
  canvas: HTMLCanvasElement | null;
  image: (HTMLImageElement | HTMLCanvasElement) | null;
  rect: Rect;
};

export const getPhysicalSelection = ({ canvas, image, rect }: PhysicalSelectionInput) => {
  if (!canvas || !image || rect.w <= 0 || rect.h <= 0) throw new Error("选区范围无效");
  const imageWidth = image instanceof HTMLImageElement ? image.naturalWidth : image.width;
  const imageHeight = image instanceof HTMLImageElement ? image.naturalHeight : image.height;
  const scaleX = imageWidth / canvas.width;
  const scaleY = imageHeight / canvas.height;
  const x = Math.max(0, Math.min(imageWidth - 1, Math.round(rect.x * scaleX)));
  const y = Math.max(0, Math.min(imageHeight - 1, Math.round(rect.y * scaleY)));
  const w = Math.max(1, Math.min(imageWidth - x, Math.round(rect.w * scaleX)));
  const h = Math.max(1, Math.min(imageHeight - y, Math.round(rect.h * scaleY)));
  return { x, y, w, h };
};

export const cropSelectionFromLoadedImage = (input: PhysicalSelectionInput) => {
  if (!input.image) throw new Error("截图图片未加载");
  const { x, y, w, h } = getPhysicalSelection(input);
  const cropCanvas = document.createElement("canvas");
  cropCanvas.width = w;
  cropCanvas.height = h;
  const ctx = cropCanvas.getContext("2d");
  if (!ctx) throw new Error("Canvas 不可用");
  ctx.drawImage(input.image, x, y, w, h, 0, 0, w, h);
  return { base64: cropCanvas.toDataURL("image/png").split(",")[1] || "", x, y, w, h };
};

export const loadPngImage = (base64: string) => new Promise<HTMLImageElement>((resolve, reject) => {
  const img = new Image();
  img.onload = () => resolve(img);
  img.onerror = reject;
  img.src = "data:image/png;base64," + base64;
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
  if (!image) throw new Error("截图图片未加载");
  const physical = getPhysicalSelection({ canvas, image, rect });
  const cropCanvas = document.createElement("canvas");
  cropCanvas.width = physical.w;
  cropCanvas.height = physical.h;
  const ctx = cropCanvas.getContext("2d");
  if (!ctx) throw new Error("Canvas 不可用");

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
