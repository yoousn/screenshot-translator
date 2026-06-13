import { useCallback, useRef } from "react";

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

  /** Get HEX color at a logical canvas coordinate */
  const getPixelHex = useCallback((clientX: number, clientY: number): { hex: string; x: number; y: number } | null => {
    const img = imageRef.current;
    const cv = canvasRef.current;
    if (!img || !cv) return null;
    const iw = img instanceof HTMLImageElement ? img.naturalWidth : img.width;
    const ih = img instanceof HTMLImageElement ? img.naturalHeight : img.height;
    const sx = Math.round(clientX * (iw / cv.width));
    const sy = Math.round(clientY * (ih / cv.height));
    if (!sampleCanvasRef.current) sampleCanvasRef.current = document.createElement("canvas");
    const s = sampleCanvasRef.current;
    s.width = 1;
    s.height = 1;
    const sctx = s.getContext("2d", { willReadFrequently: true });
    if (!sctx) return null;
    sctx.imageSmoothingEnabled = false;
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
    const ctx = overlay.getContext("2d");
    if (!ctx) return;
    const iw = img instanceof HTMLImageElement ? img.naturalWidth : img.width;
    const ih = img instanceof HTMLImageElement ? img.naturalHeight : img.height;
    const sx = clientX * (iw / cv.width);
    const sy = clientY * (ih / cv.height);
    const srcSize = box / zoom;

    ctx.imageSmoothingEnabled = false;
    ctx.clearRect(0, 0, overlay.width, overlay.height);

    // Draw zoomed pixels
    ctx.drawImage(img, sx - srcSize / 2, sy - srcSize / 2, srcSize, srcSize, 0, 0, box, box);

    // Pixel grid
    ctx.strokeStyle = "rgba(0,0,0,0.12)";
    ctx.lineWidth = 0.5;
    for (let i = 0; i <= box; i += zoom) {
      ctx.beginPath();
      ctx.moveTo(i, 0);
      ctx.lineTo(i, box);
      ctx.stroke();
      ctx.beginPath();
      ctx.moveTo(0, i);
      ctx.lineTo(box, i);
      ctx.stroke();
    }

    // Center pixel crosshair highlight
    ctx.strokeStyle = "#1677ff";
    ctx.lineWidth = 1.5;
    ctx.strokeRect(box / 2 - zoom / 2, box / 2 - zoom / 2, zoom, zoom);

    // Border
    ctx.strokeStyle = "rgba(0,0,0,0.25)";
    ctx.lineWidth = 1;
    ctx.strokeRect(0, 0, box, box);
  }, [imageRef, canvasRef]);

  return { getPixelHex, drawMagnifier };
}
