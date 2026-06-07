export type Rect = { x: number; y: number; w: number; h: number; kind?: "window" | "control" | "taskbar" | "display" | "visual" };
export type AnnotationTool = "rect" | "circle" | "mosaic" | "arrow" | "text" | "brush";
export type Point = { x: number; y: number };
export type Annotation = { type: AnnotationTool; rect: Rect; points?: Point[]; text?: string; color?: string; size?: number };
export type EditingTextDraft = { x: number; y: number; value: string; targetIndex: number | null } | null;
export type TranslatePair = { o: string; t: string; status?: "translated" | "preserved" | "untranslated" };

export interface OcrBlock {
  text: string;
  confidence: number;
  box_coords: [number, number][];
}
