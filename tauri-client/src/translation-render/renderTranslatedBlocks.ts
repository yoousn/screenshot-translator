import type { OcrBlock } from "../types/screenshot";
import { sampleBackground, getReadableTextColor } from "./background";
import { buildTranslationEraseRegion, shouldRenderTranslationBlock } from "./renderGeometry";
import { buildRenderBlocks } from "./renderBlockLayout";
import { getTranslationFontFamily } from "./textLayout";

const estimateOriginalFontSize = (rawHeight: number) => {
  const scale = rawHeight <= 18 ? 0.96 : 0.82;
  return Math.max(10, Math.min(64, Math.round(rawHeight * scale)));
};

const splitRenderLines = (text: string) => (
  text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
);

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
        const baseFontSize = estimateOriginalFontSize(rawHeight);
        const isVertical = rawHeight / rawWidth > 2.6 && !/[A-Za-z0-9]{2,}/.test(block.text);
        ctx.font = `${baseFontSize}px ${getTranslationFontFamily()}`;
        const lines = splitRenderLines(text);
        const lineHeight = Math.round(baseFontSize * 1.12);
        const measuredWidth = Math.ceil(Math.max(...lines.map((line) => ctx.measureText(line).width), 1)) + paddingX * 2;
        const maxExpandedWidth = Math.max(rawWidth, img.width - block.minX);
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

        ctx.font = `${baseFontSize}px ${getTranslationFontFamily()}`;
        ctx.fillStyle = fontColor;
        ctx.textBaseline = "middle";
        ctx.textAlign = block.direction === "rtl" ? "right" : "left";
        ctx.direction = block.direction === "rtl" ? "rtl" : "ltr";

        const totalTextHeight = lines.length * lineHeight;
        let y = lines.length <= 1
          ? block.minY + rawHeight / 2
          : block.minY + Math.max(baseFontSize / 2, totalTextHeight / 2 - lineHeight / 2);
        const textX = block.direction === "rtl" ? block.maxX : block.minX;
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
