import { useCallback, useRef } from "react";

const clampNumber = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

/**
 * F1+F2: PixPin-style square magnifier + HEX pixel color picker.
 * Samples from the source image (physical pixels), not the canvas (logical pixels).
 * Uses 1×1 drawImage sampling — no full-image getImageData per frame.
 */
export function useScreenshotMagnifier(
  imageRef: React.RefObject<HTMLImageElement | HTMLCanvasElement | null>,
  canvasRef: React.RefObject<HTMLCanvasElement | null>,
) {
  const sampleCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const sampleContextRef = useRef<CanvasRenderingContext2D | null>(null);
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
    const iw = img instanceof HTMLImageElement ? img.naturalWidth : img.width;
    const ih = img instanceof HTMLImageElement ? img.naturalHeight : img.height;
    const sx = clampNumber(Math.round(clientX * (iw / cv.width)), 0, Math.max(0, iw - 1));
    const sy = clampNumber(Math.round(clientY * (ih / cv.height)), 0, Math.max(0, ih - 1));
    if (!sampleCanvasRef.current) {
      sampleCanvasRef.current = document.createElement("canvas");
      sampleCanvasRef.current.width = 1;
      sampleCanvasRef.current.height = 1;
    }
    const s = sampleCanvasRef.current;
    let sctx = sampleContextRef.current;
    if (!sctx) {
      sctx = s.getContext("2d", { willReadFrequently: true });
      if (sctx) {
        sctx.imageSmoothingEnabled = false;
        sampleContextRef.current = sctx;
      }
    }
    if (!sctx) return null;
    sctx.drawImage(img, sx, sy, 1, 1, 0, 0, 1, 1);
    const [r, g, b] = sctx.getImageData(0, 0, 1, 1).data;
    const hex = "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase();
    return { hex, x: sx, y: sy };
  }, [imageRef, canvasRef]);

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
  }, [imageRef, canvasRef]);

  return { getPixelHex, drawMagnifier };
}
