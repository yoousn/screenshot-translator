import type { OcrBlock } from "../types/screenshot";
import { sampleBackground, getReadableTextColor } from "./background";
import { clamp } from "./geometry";
import { buildRenderBlocks } from "./renderBlockLayout";
import { fitText, getTranslationFontFamily } from "./textLayout";

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
        const fontColor = getReadableTextColor(background);
        const paddingX = Math.max(4, Math.round(rawHeight * 0.18));
        const paddingY = Math.max(2, Math.round(rawHeight * 0.12));
        const baseFontSize = Math.max(11, Math.min(36, Math.round(rawHeight * 0.82)));
        const isVertical = rawHeight / rawWidth > 2.6 && !/[A-Za-z0-9]{2,}/.test(block.text);
        ctx.font = `${baseFontSize}px ${getTranslationFontFamily()}`;
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
        ctx.textAlign = block.direction === "rtl" ? "right" : "left";
        ctx.direction = block.direction === "rtl" ? "rtl" : "ltr";

        const totalTextHeight = lines.length * lineHeight;
        let y = eraseY + paddingY + drawHeight / 2 - totalTextHeight / 2 + lineHeight / 2;
        const textX = block.direction === "rtl" ? eraseRight - paddingX : eraseX + paddingX;
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
