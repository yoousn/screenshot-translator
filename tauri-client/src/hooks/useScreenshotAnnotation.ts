import { useState, useRef, useCallback } from "react";
import type { Annotation, AnnotationTool, EditingTextDraft } from "../types/screenshot";
import { makeTextAnnotation } from "../utils/annotationGeometry";

export const DEFAULT_ANNOTATION_COLOR = "#ff4d4f";
export const DEFAULT_ANNOTATION_TOOL: AnnotationTool = "rect";
export const DEFAULT_ANNOTATION_SIZES: Record<AnnotationTool, number> = { rect: 4, circle: 4, mosaic: 16, arrow: 4, text: 4, brush: 4 };

export function useScreenshotAnnotation(onRenderNeeded: () => void) {
  const [annotationTool, setAnnotationToolState] = useState<AnnotationTool | null>(null);
  const [annotationColor, setAnnotationColor] = useState(DEFAULT_ANNOTATION_COLOR);
  const [annotationSize, setAnnotationSizeState] = useState(DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL]);
  const [selectedAnnotationIndex, setSelectedAnnotationIndex] = useState<number | null>(null);
  const [editingTextDraft, setEditingTextDraft] = useState<EditingTextDraft>(null);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [annotationHistory, setAnnotationHistory] = useState<Annotation[][]>([]);
  const [redoAnnotations, setRedoAnnotations] = useState<Annotation[][]>([]);
  const [draftAnnotation, setDraftAnnotation] = useState<Annotation | null>(null);

  const annotationToolRef = useRef<AnnotationTool>(DEFAULT_ANNOTATION_TOOL);
  const annotationColorRef = useRef(DEFAULT_ANNOTATION_COLOR);
  const annotationSizeRef = useRef(DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL]);
  const annotationSizesRef = useRef<Record<AnnotationTool, number>>({ ...DEFAULT_ANNOTATION_SIZES });
  
  const selectedAnnotationIndexRef = useRef<number | null>(null);
  const annotationsRef = useRef<Annotation[]>([]);
  const annotationHistoryRef = useRef<Annotation[][]>([]);
  const redoAnnotationsRef = useRef<Annotation[][]>([]);
  const draftAnnotationRef = useRef<Annotation | null>(null);
  const editingTextDraftRef = useRef<EditingTextDraft>(null);

  // Sync refs that might be accessed by Canvas synchronously
  selectedAnnotationIndexRef.current = selectedAnnotationIndex;
  annotationsRef.current = annotations;
  annotationHistoryRef.current = annotationHistory;
  redoAnnotationsRef.current = redoAnnotations;
  draftAnnotationRef.current = draftAnnotation;
  editingTextDraftRef.current = editingTextDraft;

  const setAnnotationTool = useCallback((tool: AnnotationTool | null) => {
    setAnnotationToolState(tool);
    if (tool) annotationToolRef.current = tool;
  }, []);

  const setAnnotationSize = useCallback((size: number) => {
    setAnnotationSizeState(size);
    annotationSizeRef.current = size;
  }, []);

  const applyAnnotations = useCallback((next: Annotation[]) => {
    annotationsRef.current = next;
    setAnnotations(next);
    onRenderNeeded();
  }, [onRenderNeeded]);

  const pushAnnotationHistory = useCallback((snapshot = annotationsRef.current) => {
    const nextHistory = [...annotationHistoryRef.current, snapshot];
    annotationHistoryRef.current = nextHistory;
    redoAnnotationsRef.current = [];
    setAnnotationHistory(nextHistory);
    setRedoAnnotations([]);
  }, []);

  const replaceAnnotations = useCallback((next: Annotation[]) => {
    pushAnnotationHistory();
    applyAnnotations(next);
  }, [applyAnnotations, pushAnnotationHistory]);

  const undoAnnotation = useCallback(() => {
    const history = annotationHistoryRef.current;
    if (history.length === 0) return;
    const next = history[history.length - 1];
    const historyNext = history.slice(0, -1);
    const redoNext = [...redoAnnotationsRef.current, annotationsRef.current];
    annotationHistoryRef.current = historyNext;
    redoAnnotationsRef.current = redoNext;
    setAnnotationHistory(historyNext);
    setRedoAnnotations(redoNext);
    applyAnnotations(next);
    setSelectedAnnotationIndex(null);
  }, [applyAnnotations]);

  const redoAnnotation = useCallback(() => {
    const redo = redoAnnotationsRef.current;
    if (redo.length === 0) return;
    const restored = redo[redo.length - 1];
    const historyNext = [...annotationHistoryRef.current, annotationsRef.current];
    const redoNext = redo.slice(0, -1);
    annotationHistoryRef.current = historyNext;
    redoAnnotationsRef.current = redoNext;
    setAnnotationHistory(historyNext);
    setRedoAnnotations(redoNext);
    applyAnnotations(restored);
    setSelectedAnnotationIndex(null);
  }, [applyAnnotations]);

  const commitAnnotation = useCallback((annotation: Annotation) => {
    pushAnnotationHistory();
    const next = [...annotationsRef.current, annotation];
    applyAnnotations(next);
  }, [pushAnnotationHistory, applyAnnotations]);

  const setAnnotationDraft = useCallback((annotation: Annotation | null) => {
    draftAnnotationRef.current = annotation;
    setDraftAnnotation(annotation);
  }, []);

  const cancelTextDraft = useCallback(() => {
    setEditingTextDraft(null);
  }, []);

  const commitTextDraft = useCallback(() => {
    const draft = editingTextDraftRef.current;
    if (!draft) return;
    const value = draft.value.trim();
    if (!value) {
      setEditingTextDraft(null);
      return;
    }
    if (draft.targetIndex !== null) {
      const current = annotationsRef.current[draft.targetIndex];
      if (current) {
        const next = [...annotationsRef.current];
        const fontSize = current.size || Math.max(14, annotationSizeRef.current + 14);
        next[draft.targetIndex] = { ...current, text: value, rect: { ...current.rect, w: Math.max(48, value.length * fontSize * 0.72 + 12), h: fontSize + 8 } };
        replaceAnnotations(next);
      }
    } else {
      commitAnnotation(makeTextAnnotation({ x: draft.x + 90, y: draft.y + 17 }, value, annotationColorRef.current, annotationSizeRef.current));
    }
    setEditingTextDraft(null);
  }, [replaceAnnotations, commitAnnotation]);

  const deleteSelectedAnnotation = useCallback(() => {
    const selectedIndex = selectedAnnotationIndexRef.current;
    if (selectedIndex === null) return;
    const current = annotationsRef.current;
    if (!current[selectedIndex]) return;
    replaceAnnotations(current.filter((_, index) => index !== selectedIndex));
    setSelectedAnnotationIndex(null);
  }, [replaceAnnotations]);

  const resetAnnotations = useCallback(() => {
    setAnnotationToolState(null);
    annotationSizesRef.current = { ...DEFAULT_ANNOTATION_SIZES };
    setAnnotations([]);
    setAnnotationHistory([]);
    setRedoAnnotations([]);
    setAnnotationColor(DEFAULT_ANNOTATION_COLOR);
    setAnnotationSizeState(DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL]);
    annotationToolRef.current = DEFAULT_ANNOTATION_TOOL;
    annotationColorRef.current = DEFAULT_ANNOTATION_COLOR;
    annotationSizeRef.current = DEFAULT_ANNOTATION_SIZES[DEFAULT_ANNOTATION_TOOL];
    selectedAnnotationIndexRef.current = null;
    annotationsRef.current = [];
    annotationHistoryRef.current = [];
    redoAnnotationsRef.current = [];
    draftAnnotationRef.current = null;
    editingTextDraftRef.current = null;
    setSelectedAnnotationIndex(null);
  }, []);

  return {
    annotationTool,
    setAnnotationTool,
    annotationColor,
    setAnnotationColor,
    annotationSize,
    setAnnotationSize,
    selectedAnnotationIndex,
    setSelectedAnnotationIndex,
    editingTextDraft,
    setEditingTextDraft,
    annotations,
    setAnnotations,
    annotationHistory,
    setAnnotationHistory,
    redoAnnotations,
    setRedoAnnotations,
    draftAnnotation,
    setAnnotationDraft,

    annotationToolRef,
    annotationColorRef,
    annotationSizeRef,
    annotationSizesRef,
    selectedAnnotationIndexRef,
    annotationsRef,
    annotationHistoryRef,
    redoAnnotationsRef,
    draftAnnotationRef,
    editingTextDraftRef,

    pushAnnotationHistory,
    undoAnnotation,
    redoAnnotation,
    commitAnnotation,
    cancelTextDraft,
    commitTextDraft,
    deleteSelectedAnnotation,
    applyAnnotations,
    replaceAnnotations,
    resetAnnotations,
  };
}
