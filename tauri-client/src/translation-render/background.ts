import type { RenderBlock, SampledColor } from "./types";
import { clamp } from "./geometry";

export const getReadableTextColor = (background: SampledColor) => {
  const luminance = 0.299 * background.r + 0.587 * background.g + 0.114 * background.b;
  return luminance > 128 ? "#000000" : "#ffffff";
};

export const sampleBackground = (ctx: CanvasRenderingContext2D, width: number, height: number, block: RenderBlock): SampledColor => {
  const points = [
    [block.minX + 2, block.minY + 2],
    [block.maxX - 2, block.minY + 2],
    [block.maxX - 2, block.maxY - 2],
    [block.minX + 2, block.maxY - 2],
  ];

  let sumR = 0;
  let sumG = 0;
  let sumB = 0;
  for (const [px, py] of points) {
    const x = clamp(Math.round(px), 0, width - 1);
    const y = clamp(Math.round(py), 0, height - 1);
    const pixel = ctx.getImageData(x, y, 1, 1).data;
    sumR += pixel[0];
    sumG += pixel[1];
    sumB += pixel[2];
  }

  return {
    r: Math.round(sumR / points.length),
    g: Math.round(sumG / points.length),
    b: Math.round(sumB / points.length),
  };
};
