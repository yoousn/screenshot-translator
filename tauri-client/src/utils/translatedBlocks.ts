import type { OcrBlock } from "../types/screenshot";

type RenderBlock = {
  text: string;
  translated: string;
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
};

const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));
const median = (values: number[]) => {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
};

const getBlockBounds = (block: OcrBlock) => {
  const xs = block.box_coords.map((point) => point[0]);
  const ys = block.box_coords.map((point) => point[1]);
  return {
    minX: Math.min(...xs),
    maxX: Math.max(...xs),
    minY: Math.min(...ys),
    maxY: Math.max(...ys),
  };
};

const isLikelyVerticalText = (block: OcrBlock) => {
  const bounds = getBlockBounds(block);
  const width = Math.max(1, bounds.maxX - bounds.minX);
  const height = Math.max(1, bounds.maxY - bounds.minY);
  const compactText = block.text.replace(/\s+/g, "");
  return height / width > 2.6 && compactText.length >= 3 && !/[A-Za-z0-9]{2,}/.test(compactText);
};

const buildRenderBlocks = (blocks: OcrBlock[], translations: string[]): RenderBlock[] => {
  const items = blocks
    .map((block, index) => ({ block, translation: translations[index] || block.text, bounds: getBlockBounds(block) }))
    .filter((item) => item.block.box_coords.length >= 4)
    .sort((a, b) => (a.bounds.minY + a.bounds.maxY) / 2 - (b.bounds.minY + b.bounds.maxY) / 2 || a.bounds.minX - b.bounds.minX);

  const heights = items.map((item) => Math.max(1, item.bounds.maxY - item.bounds.minY));
  const rowTolerance = Math.max(8, median(heights) * 0.65);
  const rows: typeof items[] = [];

  for (const item of items) {
    if (isLikelyVerticalText(item.block)) {
      rows.push([item]);
      continue;
    }

    const centerY = (item.bounds.minY + item.bounds.maxY) / 2;
    const row = rows.find((candidate) => {
      if (candidate.some((entry) => isLikelyVerticalText(entry.block))) return false;
      const rowCenterY = candidate.reduce((sum, entry) => sum + (entry.bounds.minY + entry.bounds.maxY) / 2, 0) / candidate.length;
      return Math.abs(rowCenterY - centerY) <= rowTolerance;
    });

    if (row) row.push(item);
    else rows.push([item]);
  }

  return rows.flatMap((row) => {
    const sortedRow = [...row].sort((a, b) => a.bounds.minX - b.bounds.minX);
    const rowHeights = sortedRow.map((item) => Math.max(1, item.bounds.maxY - item.bounds.minY));
    const rowHeight = Math.max(1, median(rowHeights));
    const groups: typeof sortedRow[] = [];

    for (const item of sortedRow) {
      const previousGroup = groups[groups.length - 1];
      const previous = previousGroup?.[previousGroup.length - 1];
      const gap = previous ? item.bounds.minX - previous.bounds.maxX : Infinity;
      const shouldMerge = previous && !isLikelyVerticalText(item.block) && !isLikelyVerticalText(previous.block) && gap <= Math.max(18, rowHeight * 1.8);
      if (shouldMerge) previousGroup.push(item);
      else groups.push([item]);
    }

    return groups.map((group) => ({
      text: group.map((item) => item.block.text).join(" ").replace(/\s+/g, " ").trim(),
      translated: group.map((item) => item.translation).join(" ").replace(/\s+/g, " ").trim(),
      minX: Math.min(...group.map((item) => item.bounds.minX)),
      minY: Math.min(...group.map((item) => item.bounds.minY)),
      maxX: Math.max(...group.map((item) => item.bounds.maxX)),
      maxY: Math.max(...group.map((item) => item.bounds.maxY)),
    }));
  });
};

const sampleBackground = (ctx: CanvasRenderingContext2D, width: number, height: number, block: RenderBlock) => {
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

const wrapText = (ctx: CanvasRenderingContext2D, text: string, maxWidth: number) => {
  const tokens = /[\s-]/.test(text) ? text.split(/(\s+|-)/).filter(Boolean) : Array.from(text);
  const lines: string[] = [];
  let line = "";

  for (const token of tokens) {
    const candidate = line ? line + token : token.trimStart();
    if (ctx.measureText(candidate).width > maxWidth && line.trim()) {
      lines.push(line.trim());
      line = token.trimStart();
    } else {
      line = candidate;
    }
  }

  if (line.trim()) lines.push(line.trim());
  return lines.length > 0 ? lines : [text];
};

const fitText = (ctx: CanvasRenderingContext2D, text: string, width: number, height: number, baseFontSize: number) => {
  for (let fontSize = baseFontSize; fontSize >= 10; fontSize -= 1) {
    ctx.font = `${fontSize}px 'Microsoft YaHei', -apple-system, BlinkMacSystemFont, sans-serif`;
    const lines = wrapText(ctx, text, width);
    const lineHeight = fontSize * 1.16;
    if (lines.length * lineHeight <= height + 1) return { fontSize, lines, lineHeight };
  }

  const fontSize = 10;
  ctx.font = `${fontSize}px 'Microsoft YaHei', -apple-system, BlinkMacSystemFont, sans-serif`;
  return { fontSize, lines: wrapText(ctx, text, width), lineHeight: fontSize * 1.16 };
};

export const renderTranslatedBlocks = (
  base64Image: string,
  blocks: OcrBlock[],
  translations: string[]
): Promise<string> => {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.src = "data:image/png;base64," + base64Image;
    img.onload = () => {
      const canvas = document.createElement("canvas");
      canvas.width = img.width;
      canvas.height = img.height;
      const ctx = canvas.getContext("2d", { willReadFrequently: true });
      if (!ctx) {
        reject(new Error("无法创建 2D 画布上下文"));
        return;
      }

      ctx.drawImage(img, 0, 0);

      const renderBlocks = buildRenderBlocks(blocks, translations);
      renderBlocks.forEach((block) => {
        const rawWidth = Math.max(1, block.maxX - block.minX);
        const rawHeight = Math.max(1, block.maxY - block.minY);
        const text = block.translated || block.text;
        if (!text) return;

        const background = sampleBackground(ctx, img.width, img.height, block);
        const luminance = 0.299 * background.r + 0.587 * background.g + 0.114 * background.b;
        const fontColor = luminance > 128 ? "#000000" : "#ffffff";
        const paddingX = Math.max(4, Math.round(rawHeight * 0.18));
        const paddingY = Math.max(2, Math.round(rawHeight * 0.12));
        const baseFontSize = Math.max(11, Math.min(36, Math.round(rawHeight * 0.82)));
        const isVertical = rawHeight / rawWidth > 2.6 && !/[A-Za-z0-9]{2,}/.test(block.text);
        ctx.font = `${baseFontSize}px 'Microsoft YaHei', -apple-system, BlinkMacSystemFont, sans-serif`;
        const measuredWidth = Math.ceil(ctx.measureText(text).width) + paddingX * 2;
        const maxExpandedWidth = Math.min(img.width, Math.max(rawWidth, rawWidth * 3, rawHeight * 14, 360));
        const desiredWidth = isVertical ? rawWidth : Math.max(rawWidth, Math.min(maxExpandedWidth, measuredWidth));
        const extraWidth = Math.max(0, desiredWidth - rawWidth);
        const eraseX = clamp(Math.round(block.minX - extraWidth / 2 - paddingX), 0, img.width - 1);
        const eraseY = clamp(Math.round(block.minY - paddingY), 0, img.height - 1);
        const eraseRight = clamp(Math.round(block.maxX + extraWidth / 2 + paddingX), eraseX + 1, img.width);
        const eraseBottom = clamp(Math.round(block.maxY + paddingY), eraseY + 1, img.height);
        const drawWidth = Math.max(1, eraseRight - eraseX - paddingX * 2);
        const drawHeight = Math.max(1, eraseBottom - eraseY - paddingY * 2);

        ctx.fillStyle = `rgb(${background.r}, ${background.g}, ${background.b})`;
        ctx.fillRect(eraseX, eraseY, eraseRight - eraseX, eraseBottom - eraseY);

        const { lines, lineHeight } = fitText(ctx, text, drawWidth, drawHeight, baseFontSize);
        ctx.fillStyle = fontColor;
        ctx.textBaseline = "middle";
        ctx.textAlign = "left";

        const totalTextHeight = lines.length * lineHeight;
        let y = eraseY + paddingY + drawHeight / 2 - totalTextHeight / 2 + lineHeight / 2;
        const textX = eraseX + paddingX;
        for (const line of lines) {
          ctx.fillText(line, textX, y);
          y += lineHeight;
        }
      });

      const base64Png = canvas.toDataURL("image/png").replace(/^data:image\/png;base64,/, "");
      resolve(base64Png);
    };
    img.onerror = (event) => reject(new Error("原始截图解码失败：" + event));
  });
};
