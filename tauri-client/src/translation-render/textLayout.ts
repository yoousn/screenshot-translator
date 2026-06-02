import type { FittedText } from "./types";

export const getTranslationFontFamily = () => "'Microsoft YaHei', -apple-system, BlinkMacSystemFont, sans-serif";

const truncateToWidth = (ctx: CanvasRenderingContext2D, text: string, maxWidth: number) => {
  const ellipsis = "...";
  const trimmed = text.trim();
  if (ctx.measureText(trimmed).width <= maxWidth) return trimmed;
  let next = trimmed;
  while (next.length > 0 && ctx.measureText(next + ellipsis).width > maxWidth) {
    next = next.slice(0, -1).trimEnd();
  }
  return next ? next + ellipsis : ellipsis;
};

export const wrapText = (ctx: CanvasRenderingContext2D, text: string, maxWidth: number, maxLines = Number.POSITIVE_INFINITY) => {
  const tokens = /[\s-]/.test(text) ? text.split(/(\s+|-)/).filter(Boolean) : Array.from(text);
  const lines: string[] = [];
  let line = "";

  const appendLine = (value: string) => {
    const clean = value.trim();
    if (!clean) return true;
    if (lines.length >= maxLines) {
      const lastIndex = Math.max(0, lines.length - 1);
      const joined = [lines[lastIndex], clean].filter(Boolean).join(" ");
      lines[lastIndex] = truncateToWidth(ctx, joined, maxWidth);
      return false;
    }
    lines.push(clean);
    return true;
  };

  for (const token of tokens) {
    const candidate = line ? line + token : token.trimStart();
    if (ctx.measureText(candidate).width > maxWidth && line.trim()) {
      if (!appendLine(line)) return lines;
      line = token.trimStart();
    } else {
      line = candidate;
    }
  }

  if (line.trim()) appendLine(line);
  return lines.length > 0 ? lines : [text];
};

type FitTextOptions = {
  maxLines?: number;
  minFontSize?: number;
};

export const fitText = (
  ctx: CanvasRenderingContext2D,
  text: string,
  width: number,
  height: number,
  baseFontSize: number,
  options: FitTextOptions = {},
): FittedText => {
  const family = getTranslationFontFamily();
  const minFontSize = options.minFontSize || 8;
  for (let fontSize = baseFontSize; fontSize >= minFontSize; fontSize -= 1) {
    ctx.font = `${fontSize}px ${family}`;
    const lineHeight = fontSize * 1.16;
    const maxLines = options.maxLines || Math.max(1, Math.floor((height + 1) / lineHeight));
    const lines = wrapText(ctx, text, width, maxLines);
    const widestLine = Math.max(...lines.map((line) => ctx.measureText(line).width), 0);
    if (lines.length * lineHeight <= height + 1 && widestLine <= width + 1) return { fontSize, lines, lineHeight };
  }

  const fontSize = minFontSize;
  ctx.font = `${fontSize}px ${family}`;
  const lineHeight = fontSize * 1.16;
  const maxLines = options.maxLines || Math.max(1, Math.floor((height + 1) / lineHeight));
  return { fontSize, lines: wrapText(ctx, text, width, maxLines), lineHeight };
};
