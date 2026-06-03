import type { OcrBlock } from "../types/screenshot";
import { sampleBackground, getReadableTextColor } from "./background";
import { buildTranslationEraseRegion, shouldRenderTranslationBlock } from "./renderGeometry";
import { buildRenderBlocks } from "./renderBlockLayout";
import { getTranslationFontFamily } from "./textLayout";

const splitRenderLines = (text: string) => (
  text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
);

const estimateTextWidthUnits = (text: string) => {
  const units = Array.from(text || "").reduce((sum, char) => {
    if (/\s/.test(char)) return sum + 0.32;
    if (/[\u3400-\u9fff\u3040-\u30ff\uac00-\ud7af]/.test(char)) return sum + 1;
    if (/[A-Za-z0-9]/.test(char)) return sum + 0.55;
    return sum + 0.42;
  }, 0);
  return Math.max(1, units);
};

export const estimateOriginalFontSize = (rawWidth: number, rawHeight: number, sourceText: string) => {
  const sourceLines = splitRenderLines(sourceText).length ? splitRenderLines(sourceText) : [sourceText || ""];
  const sourceLineCount = Math.max(1, sourceLines.length);
  const lineHeightEstimate = Math.max(1, rawHeight / sourceLineCount);
  const heightEstimate = lineHeightEstimate * (lineHeightEstimate <= 18 ? 0.96 : 0.82);
  const widestSourceUnits = Math.max(...sourceLines.map(estimateTextWidthUnits), 1);
  const widthEstimate = rawWidth / widestSourceUnits;
  const compactLength = (sourceText || "").replace(/\s+/g, "").length;
  const tallSparseBox = rawHeight >= 48 && compactLength <= 32 && rawHeight > rawWidth * 0.65;
  const maxFontSize = tallSparseBox ? 28 : 48;
  return Math.max(7, Math.min(maxFontSize, Math.round(Math.min(heightEstimate, widthEstimate * 1.12))));
};

const splitWrappableUnits = (line: string) => {
  if (/[\u3400-\u9fff\u3040-\u30ff\uac00-\ud7af]/.test(line)) return Array.from(line);
  const words = line.split(/(\s+)/).filter((item) => item.length > 0);
  return words.length > 1 ? words : Array.from(line);
};

const wrapLineToWidth = (ctx: CanvasRenderingContext2D, line: string, maxWidth: number) => {
  if (ctx.measureText(line).width <= maxWidth) return [line];
  const wrapped: string[] = [];
  let current = "";
  for (const unit of splitWrappableUnits(line)) {
    const next = current + unit;
    if (current && ctx.measureText(next.trim()).width > maxWidth) {
      wrapped.push(current.trim());
      current = unit.trimStart();
    } else {
      current = next;
    }
  }
  if (current.trim()) wrapped.push(current.trim());
  return wrapped.length ? wrapped : [line];
};

const wrapLinesToWidth = (ctx: CanvasRenderingContext2D, lines: string[], maxWidth: number) => (
  lines.flatMap((line) => wrapLineToWidth(ctx, line, maxWidth))
);

const fitTextInRegion = (
  ctx: CanvasRenderingContext2D,
  lines: string[],
  baseFontSize: number,
  drawWidth: number,
  drawHeight: number,
) => {
  let fontSize = baseFontSize;
  let renderLines = lines;
  let lineHeight = Math.max(8, Math.round(fontSize * 1.12));
  const minFontSize = Math.min(10, Math.max(6, baseFontSize - 4));
  while (fontSize > minFontSize) {
    ctx.font = `${fontSize}px ${getTranslationFontFamily()}`;
    renderLines = wrapLinesToWidth(ctx, lines, drawWidth);
    lineHeight = Math.max(8, Math.round(fontSize * 1.12));
    const maxLineWidth = Math.max(...renderLines.map((line) => ctx.measureText(line).width), 1);
    const totalTextHeight = renderLines.length * lineHeight;
    if (maxLineWidth <= drawWidth + 0.5 && totalTextHeight <= drawHeight + lineHeight * 0.35) break;
    fontSize -= 1;
  }
  ctx.font = `${fontSize}px ${getTranslationFontFamily()}`;
  renderLines = wrapLinesToWidth(ctx, lines, drawWidth);
  lineHeight = Math.max(8, Math.round(fontSize * 1.12));
  return { fontSize, renderLines, lineHeight };
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
        if (!shouldRenderTranslationBlock(block)) return;

        const background = sampleBackground(ctx, img.width, img.height, block);
        const fontColor = getReadableTextColor(background);
        const paddingX = Math.max(2, Math.round(rawHeight * 0.12));
        const paddingY = Math.max(2, Math.round(rawHeight * 0.16));
        const baseFontSize = estimateOriginalFontSize(rawWidth, rawHeight, block.text);
        const isVertical = rawHeight / rawWidth > 2.6 && !/[A-Za-z0-9]{2,}/.test(block.text);
        ctx.font = `${baseFontSize}px ${getTranslationFontFamily()}`;
        const lines = splitRenderLines(text);
        const lineHeight = Math.round(baseFontSize * 1.12);
        const measuredWidth = Math.ceil(Math.max(...lines.map((line) => ctx.measureText(line).width), 1)) + paddingX * 2;
        const maxExpandedWidth = Math.max(rawWidth, Math.min(img.width - block.minX, rawWidth + Math.max(160, rawWidth * 2.2)));
        const desiredWidth = isVertical ? rawWidth : Math.max(rawWidth, Math.min(maxExpandedWidth, measuredWidth));
        const { eraseX, eraseY, eraseRight, eraseBottom } = buildTranslationEraseRegion(
          block,
          img.width,
          img.height,
          desiredWidth,
          paddingX,
          paddingY,
        );

        ctx.fillStyle = `rgb(${background.r}, ${background.g}, ${background.b})`;
        ctx.fillRect(eraseX, eraseY, eraseRight - eraseX, eraseBottom - eraseY);

        const fitted = fitTextInRegion(
          ctx,
          lines,
          baseFontSize,
          Math.max(1, eraseRight - eraseX - paddingX * 2),
          Math.max(1, eraseBottom - eraseY - paddingY * 2),
        );
        ctx.font = `${fitted.fontSize}px ${getTranslationFontFamily()}`;
        ctx.fillStyle = fontColor;
        ctx.textBaseline = "middle";
        ctx.textAlign = block.direction === "rtl" ? "right" : "left";
        ctx.direction = block.direction === "rtl" ? "rtl" : "ltr";

        ctx.save();
        ctx.beginPath();
        ctx.rect(eraseX, eraseY, eraseRight - eraseX, eraseBottom - eraseY);
        ctx.clip();

        const totalTextHeight = fitted.renderLines.length * fitted.lineHeight;
        let y = fitted.renderLines.length <= 1
          ? block.minY + rawHeight / 2
          : Math.max(eraseY + paddingY + fitted.fontSize / 2, block.minY + Math.max(fitted.fontSize / 2, totalTextHeight / 2 - fitted.lineHeight / 2));
        const textX = block.direction === "rtl"
          ? Math.min(block.maxX, eraseRight - paddingX)
          : Math.max(block.minX, eraseX + paddingX);
        for (const line of fitted.renderLines) {
          ctx.fillText(line, textX, y);
          y += fitted.lineHeight;
        }
        ctx.restore();
      });

      const base64Png = canvas.toDataURL("image/png").replace(/^data:image\/png;base64,/, "");
      resolve(base64Png);
    };
    img.onerror = (event) => reject(new Error("原始截图解码失败：" + event));
  });
};
