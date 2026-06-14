import { useCallback, useRef } from "react";

const clampNumber = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
const FALLBACK_PIXEL_HEX = "#000000";

/**
 * F1+F2: PixPin-style square magnifier + HEX pixel color picker.
 * Samples from the source image (physical pixels), not the canvas (logical pixels).
 * Uses 1×1 drawImage sampling — no full-image getImageData per frame.
 */
export function useScreenshotMagnifier(
  imageRef: React.RefObject<HTMLImageElement | HTMLCanvasElement | null>,
  canvasRef: React.RefObject<HTMLCanvasElement | null>,
  analysisImageDataRef?: React.RefObject<ImageData | null>,
) {
  const lastPixelInfoRef = useRef<{ hex: string; x: number; y: number } | null>(null);
  const overlayContextRef = useRef<{
    canvas: HTMLCanvasElement;
    context: CanvasRenderingContext2D;
  } | null>(null);
  const decorationCanvasRef = useRef<{
    key: string;
    canvas: HTMLCanvasElement;
  } | null>(null);

  /** Get HEX color at a logical canvas coordinate */
  const getPixelHex = useCallback((clientX: number, clientY: number): { hex: string; x: number; y: number } | null => {
    const img = imageRef.current;
    const cv = canvasRef.current;
    if (!img || !cv) return null;
    const cachedImageData = analysisImageDataRef?.current;
    if (cachedImageData) {
      const cacheX = clampNumber(Math.round(clientX * (cachedImageData.width / cv.width)), 0, Math.max(0, cachedImageData.width - 1));
      const cacheY = clampNumber(Math.round(clientY * (cachedImageData.height / cv.height)), 0, Math.max(0, cachedImageData.height - 1));
      const baseIndex = (cacheY * cachedImageData.width + cacheX) * 4;
      const r = cachedImageData.data[baseIndex] ?? 0;
      const g = cachedImageData.data[baseIndex + 1] ?? 0;
      const b = cachedImageData.data[baseIndex + 2] ?? 0;
      const hex = "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase();
      const info = { hex, x: cacheX, y: cacheY };
      lastPixelInfoRef.current = info;
      return info;
    }
    const iw = img instanceof HTMLImageElement ? img.naturalWidth : img.width;
    const ih = img instanceof HTMLImageElement ? img.naturalHeight : img.height;
    const sx = clampNumber(Math.round(clientX * (iw / cv.width)), 0, Math.max(0, iw - 1));
    const sy = clampNumber(Math.round(clientY * (ih / cv.height)), 0, Math.max(0, ih - 1));
    return { hex: lastPixelInfoRef.current?.hex || FALLBACK_PIXEL_HEX, x: sx, y: sy };
  }, [imageRef, canvasRef, analysisImageDataRef]);

  /** Draw PixPin-style square magnifier to an overlay canvas element */
  const drawMagnifier = useCallback((
    overlay: HTMLCanvasElement | null,
    clientX: number,
    clientY: number,
    zoom = 8,
    box = 120,
  ) => {
    const img = imageRef.current;
    const cv = canvasRef.current;
    if (!overlay || !img || !cv) return;
    let ctx = overlayContextRef.current?.canvas === overlay
      ? overlayContextRef.current.context
      : null;
    if (!ctx) {
      ctx = overlay.getContext("2d");
      if (ctx) overlayContextRef.current = { canvas: overlay, context: ctx };
    }
    if (!ctx) return;
    const iw = img instanceof HTMLImageElement ? img.naturalWidth : img.width;
    const ih = img instanceof HTMLImageElement ? img.naturalHeight : img.height;
    const sx = clampNumber(clientX * (iw / cv.width), 0, Math.max(0, iw - 1));
    const sy = clampNumber(clientY * (ih / cv.height), 0, Math.max(0, ih - 1));
    const srcSize = box / zoom;
    const sourceX = clampNumber(sx - srcSize / 2, 0, Math.max(0, iw - srcSize));
    const sourceY = clampNumber(sy - srcSize / 2, 0, Math.max(0, ih - srcSize));

    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, overlay.width, overlay.height);

    ctx.drawImage(img, sourceX, sourceY, srcSize, srcSize, 0, 0, box, box);

    const decorationKey = `${box}:${zoom}`;
    let decoration = decorationCanvasRef.current?.key === decorationKey
      ? decorationCanvasRef.current.canvas
      : null;
    if (!decoration) {
      decoration = document.createElement("canvas");
      decoration.width = box;
      decoration.height = box;
      const dctx = decoration.getContext("2d");
      if (dctx) {
        dctx.strokeStyle = "rgba(0,0,0,0.12)";
        dctx.lineWidth = 0.5;
        for (let i = 0; i <= box; i += zoom) {
          dctx.beginPath();
          dctx.moveTo(i, 0);
          dctx.lineTo(i, box);
          dctx.stroke();
          dctx.beginPath();
          dctx.moveTo(0, i);
          dctx.lineTo(box, i);
          dctx.stroke();
        }
        dctx.strokeStyle = "#1677ff";
        dctx.lineWidth = 1.5;
        dctx.strokeRect(box / 2 - zoom / 2, box / 2 - zoom / 2, zoom, zoom);
        dctx.strokeStyle = "rgba(0,0,0,0.25)";
        dctx.lineWidth = 1;
        dctx.strokeRect(0, 0, box, box);
      }
      decorationCanvasRef.current = { key: decorationKey, canvas: decoration };
    }
    ctx.drawImage(decoration, 0, 0);
  }, [imageRef, canvasRef, analysisImageDataRef]);

  return { getPixelHex, drawMagnifier };
}
