import type { Annotation, Point, Rect } from "../types/screenshot";
const drawSharpArrow = (ctx: CanvasRenderingContext2D, start: Point, end: Point, color: string, lineWidth: number) => {
  const angle = Math.atan2(end.y - start.y, end.x - start.x);
  const headLength = Math.max(14, lineWidth * 4.5);
  const headAngle = Math.PI / 7;
  const lineEnd = {
    x: end.x - Math.cos(angle) * Math.min(headLength * 0.45, Math.hypot(end.x - start.x, end.y - start.y) * 0.35),
    y: end.y - Math.sin(angle) * Math.min(headLength * 0.45, Math.hypot(end.x - start.x, end.y - start.y) * 0.35),
  };

  ctx.strokeStyle = color;
  ctx.fillStyle = color;
  ctx.lineWidth = lineWidth;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(start.x, start.y);
  ctx.lineTo(lineEnd.x, lineEnd.y);
  ctx.stroke();

  ctx.beginPath();
  ctx.moveTo(end.x, end.y);
  ctx.lineTo(end.x - headLength * Math.cos(angle - headAngle), end.y - headLength * Math.sin(angle - headAngle));
  ctx.lineTo(end.x - headLength * 0.52 * Math.cos(angle), end.y - headLength * 0.52 * Math.sin(angle));
  ctx.lineTo(end.x - headLength * Math.cos(angle + headAngle), end.y - headLength * Math.sin(angle + headAngle));
  ctx.closePath();
  ctx.fill();
  ctx.lineCap = "butt";
};

export const drawAnnotation = (
  ctx: CanvasRenderingContext2D,
  annotation: Annotation,
  options: { index?: number; selectedIndex?: number | null } = {},
) => {
  const { x, y, w, h } = annotation.rect;
  const color = annotation.color || "#ff4d4f";
  const size = annotation.size || 4;

  if (annotation.type === "brush") {
    const points = annotation.points || [];
    if (points.length < 2) return;
    ctx.strokeStyle = color;
    ctx.lineWidth = size;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    for (const point of points.slice(1)) ctx.lineTo(point.x, point.y);
    ctx.stroke();
    ctx.lineCap = "butt";
    return;
  }

  if (annotation.type === "arrow") {
    const points = annotation.points || [];
    if (points.length < 2) return;
    const [start, end] = points;
    drawSharpArrow(ctx, start, end, color, size);
    return;
  }

  if (annotation.type === "text") {
    if (!annotation.text) return;
    const fontSize = annotation.size || 18;
    ctx.font = fontSize + "px Microsoft YaHei, sans-serif";
    ctx.fillStyle = "rgba(255,255,255,0.72)";
    const width = ctx.measureText(annotation.text).width + 14;
    const height = fontSize + 10;
    ctx.fillRect(x, y, width, height);
    ctx.strokeStyle = color;
    ctx.lineWidth = 1;
    ctx.strokeRect(x, y, width, height);
    ctx.fillStyle = color;
    ctx.fillText(annotation.text, x + 7, y + fontSize + 2);
    annotation.rect.w = width;
    annotation.rect.h = height;
    return;
  }

  if (w <= 0 || h <= 0) return;

  if (annotation.type === "mosaic") {
    const block = 10;
    const temp = document.createElement("canvas");
    temp.width = Math.max(1, Math.ceil(w / block));
    temp.height = Math.max(1, Math.ceil(h / block));
    const tempCtx = temp.getContext("2d");
    if (tempCtx) {
      tempCtx.imageSmoothingEnabled = false;
      tempCtx.drawImage(ctx.canvas, x, y, w, h, 0, 0, temp.width, temp.height);
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(temp, 0, 0, temp.width, temp.height, x, y, w, h);
      ctx.imageSmoothingEnabled = true;
    }
    ctx.strokeStyle = "rgba(250, 84, 28, 0.85)";
    ctx.lineWidth = 1;
    ctx.strokeRect(x, y, w, h);
    return;
  }

  ctx.strokeStyle = color;
  ctx.lineWidth = size;
  ctx.setLineDash([]);
  if (annotation.type === "circle") {
    ctx.beginPath();
    ctx.ellipse(x + w / 2, y + h / 2, Math.max(1, w / 2), Math.max(1, h / 2), 0, 0, Math.PI * 2);
    ctx.stroke();
  } else {
    ctx.strokeRect(x, y, w, h);
  }
};

type RenderExportAnnotationsOptions = {
  ctx: CanvasRenderingContext2D;
  cropCanvas: HTMLCanvasElement;
  annotations: Annotation[];
  selection: Rect;
  canvasWidth: number;
  canvasHeight: number;
  imageWidth: number;
  imageHeight: number;
  fallbackColor: string;
  fallbackSize: number;
};

export const renderExportAnnotations = ({
  ctx,
  cropCanvas,
  annotations,
  selection,
  canvasWidth,
  canvasHeight,
  imageWidth,
  imageHeight,
  fallbackColor,
  fallbackSize,
}: RenderExportAnnotationsOptions) => {
  const scaleX = imageWidth / canvasWidth;
  const scaleY = imageHeight / canvasHeight;
  const scaleStroke = (annotation: Annotation) => Math.max(1, Math.round((annotation.size || fallbackSize) * Math.max(scaleX, scaleY)));
  const mapPoint = (point: Point) => ({
    x: Math.round((point.x - selection.x) * scaleX),
    y: Math.round((point.y - selection.y) * scaleY),
  });

  for (const annotation of annotations) {
    const ax = Math.max(0, Math.round((annotation.rect.x - selection.x) * scaleX));
    const ay = Math.max(0, Math.round((annotation.rect.y - selection.y) * scaleY));
    const aw = Math.max(1, Math.round(annotation.rect.w * scaleX));
    const ah = Math.max(1, Math.round(annotation.rect.h * scaleY));
    const color = annotation.color || fallbackColor;

    if (annotation.type === "brush") {
      const points = (annotation.points || []).map(mapPoint);
      if (points.length < 2) continue;
      ctx.strokeStyle = color;
      ctx.lineWidth = scaleStroke(annotation);
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      ctx.beginPath();
      ctx.moveTo(points[0].x, points[0].y);
      for (const point of points.slice(1)) ctx.lineTo(point.x, point.y);
      ctx.stroke();
      ctx.lineCap = "butt";
    } else if (annotation.type === "arrow") {
      const points = (annotation.points || []).map(mapPoint);
      if (points.length < 2) continue;
      const [start, end] = points;
      drawSharpArrow(ctx, start, end, color, scaleStroke(annotation));
    } else if (annotation.type === "text") {
      if (!annotation.text) continue;
      const fontSize = Math.max(12, Math.round((annotation.size || 18) * scaleY));
      ctx.font = `${fontSize}px Microsoft YaHei, sans-serif`;
      const width = ctx.measureText(annotation.text).width + 14;
      ctx.fillStyle = "rgba(255,255,255,0.92)";
      ctx.fillRect(ax, ay, width, fontSize + 12);
      ctx.strokeStyle = color;
      ctx.lineWidth = Math.max(1, Math.round(scaleY));
      ctx.strokeRect(ax, ay, width, fontSize + 12);
      ctx.fillStyle = color;
      ctx.fillText(annotation.text, ax + 7, ay + fontSize + 2);
    } else if (annotation.type === "mosaic") {
      const block = 10;
      const temp = document.createElement("canvas");
      temp.width = Math.max(1, Math.ceil(aw / block));
      temp.height = Math.max(1, Math.ceil(ah / block));
      const tempCtx = temp.getContext("2d");
      if (tempCtx) {
        tempCtx.imageSmoothingEnabled = false;
        tempCtx.drawImage(cropCanvas, ax, ay, aw, ah, 0, 0, temp.width, temp.height);
        ctx.imageSmoothingEnabled = false;
        ctx.drawImage(temp, 0, 0, temp.width, temp.height, ax, ay, aw, ah);
        ctx.imageSmoothingEnabled = true;
      }
    } else {
      ctx.strokeStyle = color;
      ctx.lineWidth = scaleStroke(annotation);
      if (annotation.type === "circle") {
        ctx.beginPath();
        ctx.ellipse(ax + aw / 2, ay + ah / 2, Math.max(1, aw / 2), Math.max(1, ah / 2), 0, 0, Math.PI * 2);
        ctx.stroke();
      } else {
        ctx.strokeRect(ax, ay, aw, ah);
      }
    }
  }
};
