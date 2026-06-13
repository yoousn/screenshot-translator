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

const pixelateBlock = (ctx: CanvasRenderingContext2D, x: number, y: number, size: number) => {
  const sourceX = Math.max(0, Math.round(x - size / 2));
  const sourceY = Math.max(0, Math.round(y - size / 2));
  const sourceW = Math.max(1, Math.min(Math.round(size), ctx.canvas.width - sourceX));
  const sourceH = Math.max(1, Math.min(Math.round(size), ctx.canvas.height - sourceY));
  if (sourceW <= 0 || sourceH <= 0) return;
  const temp = document.createElement("canvas");
  temp.width = 1;
  temp.height = 1;
  const tempCtx = temp.getContext("2d");
  if (!tempCtx) return;
  tempCtx.imageSmoothingEnabled = false;
  tempCtx.drawImage(ctx.canvas, sourceX, sourceY, sourceW, sourceH, 0, 0, 1, 1);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(temp, 0, 0, 1, 1, sourceX, sourceY, sourceW, sourceH);
  ctx.imageSmoothingEnabled = true;
};

const drawMosaic = (ctx: CanvasRenderingContext2D, annotation: Annotation, fallbackSize: number) => {
  const blockSize = Math.max(8, Math.round((annotation.size || fallbackSize) * 1.6));
  const points = annotation.points || [];
  if (points.length > 0) {
    points.forEach((point) => pixelateBlock(ctx, point.x, point.y, blockSize));
    return;
  }
  const { x, y, w, h } = annotation.rect;
  for (let py = y; py <= y + h; py += blockSize) {
    for (let px = x; px <= x + w; px += blockSize) pixelateBlock(ctx, px, py, blockSize);
  }
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

  if (annotation.type === "number") {
    const d = annotation.rect.w || (size + 14);
    const ncx = annotation.rect.x + d / 2;
    const ncy = annotation.rect.y + d / 2;
    const r = d / 2;
    ctx.fillStyle = color;
    ctx.beginPath();
    if (annotation.markerShape === "square") {
      ctx.rect(annotation.rect.x, annotation.rect.y, d, d);
      ctx.fill();
    } else if (annotation.markerShape === "drop") {
      ctx.moveTo(ncx, annotation.rect.y + d);
      ctx.quadraticCurveTo(annotation.rect.x, ncy, ncx, annotation.rect.y);
      ctx.quadraticCurveTo(annotation.rect.x + d, ncy, ncx, annotation.rect.y + d);
      ctx.fill();
    } else {
      ctx.arc(ncx, ncy, r, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.fillStyle = "#ffffff";
    ctx.font = `bold ${Math.round(d * 0.58)}px sans-serif`;
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(String(annotation.markerIndex ?? "?"), ncx, ncy + 1);
    ctx.textAlign = "start";
    ctx.textBaseline = "alphabetic";
    return;
  }

  if (w <= 0 || h <= 0) return;

  if (annotation.type === "mosaic") {
    drawMosaic(ctx, annotation, size);
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

  if (options.index !== undefined && options.selectedIndex === options.index) {
    ctx.save();
    ctx.setLineDash([4, 3]);
    ctx.strokeStyle = "#1677ff";
    ctx.lineWidth = 1;
    ctx.strokeRect(x - 4, y - 4, w + 8, h + 8);
    ctx.restore();
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
    } else if (annotation.type === "number") {
      const d = Math.max(20, Math.round(aw || (annotation.size || 16) * Math.max(scaleX, scaleY) + 14));
      const cx = ax + d / 2;
      const cy = ay + d / 2;
      ctx.fillStyle = color;
      ctx.beginPath();
      if (annotation.markerShape === "square") { ctx.rect(ax, ay, d, d); }
      else { ctx.arc(cx, cy, d / 2, 0, Math.PI * 2); }
      ctx.fill();
      ctx.fillStyle = "#ffffff";
      ctx.font = `bold ${Math.round(d * 0.58)}px sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(String(annotation.markerIndex ?? "?"), cx, cy + 1);
      ctx.textAlign = "start";
      ctx.textBaseline = "alphabetic";
    } else if (annotation.type === "mosaic") {
      drawMosaic(ctx, { ...annotation, rect: { x: ax, y: ay, w: aw, h: ah }, points: annotation.points?.map(mapPoint), size: scaleStroke(annotation) }, fallbackSize);
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
