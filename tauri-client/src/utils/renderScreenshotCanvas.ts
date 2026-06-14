import type { Annotation, Rect } from "../types/screenshot";
import { clamp } from "./annotationGeometry";
import { drawAnnotation } from "./renderAnnotations";

type RenderScreenshotCanvasOptions = {
  canvas: HTMLCanvasElement | null;
  image: (HTMLImageElement | HTMLCanvasElement) | null;
  maskedCanvas: HTMLCanvasElement | null;
  hoverRect: Rect | null;
  hoverCandidatesCount: number;
  hoverCandidateIndex: number;
  hasSelected: boolean;
  selection: Rect;
  translatedImg: (HTMLImageElement | HTMLCanvasElement) | null;
  overrideTranslatedImg?: HTMLImageElement | HTMLCanvasElement;
  annotations: Annotation[];
  draftAnnotation: Annotation | null;
  selectedAnnotationIndex: number | null;
  detectionBorderWidth: number;
  selectionBorderColor?: string;
  selectionLabelColor?: string;
  selectionOnly?: boolean;
  previousSelection?: Rect | null;
};

const getImageWidth = (image: HTMLImageElement | HTMLCanvasElement) => image instanceof HTMLImageElement ? image.naturalWidth : image.width;
const getImageHeight = (image: HTMLImageElement | HTMLCanvasElement) => image instanceof HTMLImageElement ? image.naturalHeight : image.height;
const hasArea = (rect: Rect | null | undefined) => !!rect && rect.w > 0 && rect.h > 0;

const clampCanvasRect = (
  rect: { x: number; y: number; w: number; h: number },
  canvas: HTMLCanvasElement,
) => {
  const x1 = Math.max(0, Math.floor(rect.x));
  const y1 = Math.max(0, Math.floor(rect.y));
  const x2 = Math.min(canvas.width, Math.ceil(rect.x + rect.w));
  const y2 = Math.min(canvas.height, Math.ceil(rect.y + rect.h));
  const w = x2 - x1;
  const h = y2 - y1;
  return w > 0 && h > 0 ? { x: x1, y: y1, w, h } : null;
};

const unionCanvasRects = (
  canvas: HTMLCanvasElement,
  first: { x: number; y: number; w: number; h: number } | null,
  second: { x: number; y: number; w: number; h: number } | null,
) => {
  if (!first) return second ? clampCanvasRect(second, canvas) : null;
  if (!second) return clampCanvasRect(first, canvas);
  const x = Math.min(first.x, second.x);
  const y = Math.min(first.y, second.y);
  const right = Math.max(first.x + first.w, second.x + second.w);
  const bottom = Math.max(first.y + first.h, second.y + second.h);
  return clampCanvasRect({ x, y, w: right - x, h: bottom - y }, canvas);
};

const getSelectionDecorationRect = (
  ctx: CanvasRenderingContext2D,
  canvas: HTMLCanvasElement,
  rect: Rect | null | undefined,
) => {
  if (!rect) return null;
  const pad = 12;
  if (!hasArea(rect)) {
    return clampCanvasRect({ x: rect.x - pad, y: rect.y - pad, w: pad * 2, h: pad * 2 }, canvas);
  }
  const x = Math.min(rect.x, rect.x + rect.w);
  const y = Math.min(rect.y, rect.y + rect.h);
  const w = Math.abs(rect.w);
  const h = Math.abs(rect.h);
  ctx.font = "12px sans-serif";
  const label = `${Math.round(w)} x ${Math.round(h)}`;
  const labelWidth = ctx.measureText(label).width + 12;
  const tipY = y - 22 >= 0 ? y - 22 : y + h + 4;
  const left = Math.min(x, x - pad);
  const top = Math.min(y, tipY, y - pad);
  const right = Math.max(x + w, x + labelWidth, x + w + pad);
  const bottom = Math.max(y + h, tipY + 20, y + h + pad);
  return clampCanvasRect({ x: left, y: top, w: right - left, h: bottom - top }, canvas);
};

const getHandlePoints = (x: number, y: number, w: number, h: number) => [
  { x, y },
  { x: x + w, y },
  { x, y: y + h },
  { x: x + w, y: y + h },
  { x: x + w / 2, y },
  { x: x + w / 2, y: y + h },
  { x, y: y + h / 2 },
  { x: x + w, y: y + h / 2 },
];

export const renderScreenshotCanvas = ({
  canvas,
  image,
  maskedCanvas,
  hoverRect,
  hoverCandidatesCount,
  hoverCandidateIndex,
  hasSelected,
  selection,
  translatedImg,
  overrideTranslatedImg,
  annotations,
  draftAnnotation,
  selectedAnnotationIndex,
  detectionBorderWidth,
  selectionBorderColor = "#1677ff",
  selectionLabelColor = selectionBorderColor,
  selectionOnly = false,
  previousSelection,
}: RenderScreenshotCanvasOptions) => {
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const sourceImage = image;
  const activeImg = overrideTranslatedImg || translatedImg;
  const canUseDirtySelection =
    !selectionOnly
    && previousSelection !== undefined
    && maskedCanvas
    && sourceImage
    && !hoverRect
    && !activeImg
    && annotations.length === 0
    && !draftAnnotation;
  const dirtyRect = canUseDirtySelection
    ? unionCanvasRects(
      canvas,
      getSelectionDecorationRect(ctx, canvas, previousSelection),
      getSelectionDecorationRect(ctx, canvas, selection),
    )
    : null;

  if (dirtyRect && maskedCanvas) {
    ctx.drawImage(maskedCanvas, dirtyRect.x, dirtyRect.y, dirtyRect.w, dirtyRect.h, dirtyRect.x, dirtyRect.y, dirtyRect.w, dirtyRect.h);
  } else if (selectionOnly) ctx.clearRect(0, 0, canvas.width, canvas.height);
  else if (maskedCanvas) ctx.drawImage(maskedCanvas, 0, 0);
  else if (sourceImage) {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(sourceImage, 0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "rgba(0, 0, 0, 0.45)";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
  } else {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
  }

  const preview = hoverRect;
  if (!hasSelected && preview && preview.w > 0 && preview.h > 0) {
    if (sourceImage) {
      const scaleX = getImageWidth(sourceImage) / canvas.width;
      const scaleY = getImageHeight(sourceImage) / canvas.height;
      ctx.clearRect(preview.x, preview.y, preview.w, preview.h);
      ctx.drawImage(sourceImage, preview.x * scaleX, preview.y * scaleY, preview.w * scaleX, preview.h * scaleY, preview.x, preview.y, preview.w, preview.h);
    } else {
      ctx.fillStyle = "rgba(255, 255, 255, 0.08)";
      ctx.fillRect(preview.x, preview.y, preview.w, preview.h);
    }
    ctx.strokeStyle = "#1677ff";
    ctx.lineWidth = clamp(detectionBorderWidth || 2, 1, 6);
    ctx.setLineDash([]);
    ctx.strokeRect(preview.x, preview.y, preview.w, preview.h);
    ctx.fillStyle = "#1677ff";
    const hs = 7;
    const halfHs = hs / 2;
    for (const point of getHandlePoints(preview.x, preview.y, preview.w, preview.h)) ctx.fillRect(point.x - halfHs, point.y - halfHs, hs, hs);
    const layerText = hoverCandidatesCount > 1 ? ` / ${hoverCandidateIndex + 1}/${hoverCandidatesCount} / Tab切换` : "";
    const kindLabel = preview.kind === "control" ? "控件" : preview.kind === "visual" ? "视觉" : preview.kind === "window" ? "窗口" : "";
    const kindText = kindLabel ? ` / ${kindLabel}` : "";
    const sizeText = `${Math.round(preview.w)} x ${Math.round(preview.h)}${kindText}${layerText} / Enter确认`;
    ctx.font = "12px sans-serif";
    const sizeWidth = ctx.measureText(sizeText).width;
    const labelY = preview.y - 24 >= 0 ? preview.y - 24 : preview.y + 4;
    ctx.fillStyle = "#1677ff";
    ctx.fillRect(preview.x, labelY, sizeWidth + 12, 20);
    ctx.fillStyle = "#ffffff";
    ctx.fillText(sizeText, preview.x + 6, labelY + 14);
  }

  const { x, y, w, h } = selection;
  if (w > 0 && h > 0) {
    if (!selectionOnly) ctx.clearRect(x, y, w, h);
    if (!selectionOnly && activeImg) ctx.drawImage(activeImg, x, y, w, h);
    else if (!selectionOnly && sourceImage) {
      const scaleX = getImageWidth(sourceImage) / canvas.width;
      const scaleY = getImageHeight(sourceImage) / canvas.height;
      ctx.drawImage(sourceImage, x * scaleX, y * scaleY, w * scaleX, h * scaleY, x, y, w, h);
    }
    if (!selectionOnly) [...annotations, ...(draftAnnotation ? [draftAnnotation] : [])].forEach((annotation, index) => drawAnnotation(ctx, annotation, { index, selectedIndex: selectedAnnotationIndex }));
    ctx.strokeStyle = selectionBorderColor;
    ctx.lineWidth = clamp(detectionBorderWidth || 2, 1, 6);
    ctx.strokeRect(x, y, w, h);
    ctx.fillStyle = "#ffffff";
    ctx.strokeStyle = selectionBorderColor;
    const hs = 6;
    const halfHs = 3;
    for (const point of getHandlePoints(x, y, w, h)) {
      ctx.fillRect(point.x - halfHs, point.y - halfHs, hs, hs);
      ctx.strokeRect(point.x - halfHs, point.y - halfHs, hs, hs);
    }
    ctx.fillStyle = selectionLabelColor;
    ctx.font = "12px sans-serif";
    const text = `${Math.round(w)} x ${Math.round(h)}`;
    const textWidth = ctx.measureText(text).width;
    const tipY = y - 22 >= 0 ? y - 22 : y + h + 4;
    ctx.fillRect(x, tipY, textWidth + 12, 20);
    ctx.fillStyle = "#ffffff";
    ctx.fillText(text, x + 6, tipY + 14);
  }
};
