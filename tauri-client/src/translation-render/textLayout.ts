import type { FittedText } from "./types";

export const getTranslationFontFamily = () => "'Microsoft YaHei', -apple-system, BlinkMacSystemFont, sans-serif";

export const wrapText = (ctx: CanvasRenderingContext2D, text: string, maxWidth: number) => {
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

export const fitText = (ctx: CanvasRenderingContext2D, text: string, width: number, height: number, baseFontSize: number): FittedText => {
  const family = getTranslationFontFamily();
  for (let fontSize = baseFontSize; fontSize >= 10; fontSize -= 1) {
    ctx.font = `${fontSize}px ${family}`;
    const lines = wrapText(ctx, text, width);
    const lineHeight = fontSize * 1.16;
    if (lines.length * lineHeight <= height + 1) return { fontSize, lines, lineHeight };
  }

  const fontSize = 10;
  ctx.font = `${fontSize}px ${family}`;
  return { fontSize, lines: wrapText(ctx, text, width), lineHeight: fontSize * 1.16 };
};
