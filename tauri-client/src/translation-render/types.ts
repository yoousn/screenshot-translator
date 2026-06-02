export type RenderBlock = {
  text: string;
  translated: string;
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
  direction?: "ltr" | "rtl" | "auto";
};

export type FittedText = {
  fontSize: number;
  lines: string[];
  lineHeight: number;
};

export type SampledColor = {
  r: number;
  g: number;
  b: number;
};
