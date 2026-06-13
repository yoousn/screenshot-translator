import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Rect } from "../types/screenshot";
import type { Config } from "../types/config";
import { getDetectionCandidatesAt, rectSignature } from "../utils/detectionCandidates";

const WINDOW_RECT_STALE_FALLBACK_MS = 2500;

const hasPreciseWindowRect = (rects: Rect[]) => rects.some((rect) => rect.kind !== "display");
const isReusablePreciseRect = (rect: Rect) => rect.kind !== "display" && rect.kind !== "taskbar";

const dedupeRects = (rects: Rect[]) => {
  const seen = new Set<string>();
  return rects.filter((rect) => {
    const key = `${rect.kind || "unknown"}:${rectSignature(rect)}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
};

interface UseScreenshotWindowRectsProps {
  configRef: React.MutableRefObject<Config>;
  lastMouseRef: React.MutableRefObject<{ x: number; y: number }>;
  analysisImageDataRef: React.MutableRefObject<ImageData | null>;
  interactionStateRef: React.MutableRefObject<{
    hasSelected: boolean;
    isSelecting: boolean;
    isDragging: boolean;
    isResizing: boolean;
  }>;
  triggerRender: () => void;
}

export function useScreenshotWindowRects({
  configRef,
  lastMouseRef,
  analysisImageDataRef,
  interactionStateRef,
  triggerRender,
}: UseScreenshotWindowRectsProps) {
  const [windowRects, setWindowRects] = useState<Rect[]>([]);
  const [hoverRect, setHoverRectState] = useState<Rect | null>(null);
  const [hoverCandidates, setHoverCandidates] = useState<Rect[]>([]);

  const windowRectsRef = useRef<Rect[]>([]);
  const hoverRectRef = useRef<Rect | null>(null);
  const hoverCandidatesRef = useRef<Rect[]>([]);
  const hoverCandidateIndexRef = useRef(0);
  const hoverCandidatesSignatureRef = useRef("");
  const lastRectQueryRef = useRef(0);
  const rectQueryPendingRef = useRef(false);
  const lastPreciseRectsRef = useRef<Rect[]>([]);
  const lastPreciseRectsAtRef = useRef(0);

  const setHoverCandidate = (candidate: Rect | null) => {
    hoverRectRef.current = candidate;
    setHoverRectState(candidate);
  };

  const setHoverCandidateList = (candidates: Rect[]) => {
    const signature = candidates.map(rectSignature).join("|");
    if (signature !== hoverCandidatesSignatureRef.current) {
      hoverCandidateIndexRef.current = 0;
      hoverCandidatesSignatureRef.current = signature;
    }
    hoverCandidatesRef.current = candidates;
    setHoverCandidates(candidates);
    const nextIndex =
      candidates.length === 0 ? 0 : hoverCandidateIndexRef.current % candidates.length;
    hoverCandidateIndexRef.current = nextIndex;
    setHoverCandidate(candidates[nextIndex] || null);
  };

  const loadWindowRects = async (force = false) => {
    const now = performance.now();
    if (!force && (rectQueryPendingRef.current || now - lastRectQueryRef.current < 50)) return;
    lastRectQueryRef.current = now;
    rectQueryPendingRef.current = true;
    try {
      const includeControls = Boolean(configRef.current.enableUiControlDetection);
      const nextRects = JSON.parse(await invoke<string>("get_window_rects", { includeControls })) as Rect[];
      const hasPreciseRects = hasPreciseWindowRect(nextRects);
      const reusablePreciseRects = nextRects.filter(isReusablePreciseRect);
      if (reusablePreciseRects.length > 0) {
        lastPreciseRectsRef.current = reusablePreciseRects;
        lastPreciseRectsAtRef.current = performance.now();
      }

      const recentPreciseRects =
        !hasPreciseRects
        && lastPreciseRectsRef.current.length > 0
        && performance.now() - lastPreciseRectsAtRef.current <= WINDOW_RECT_STALE_FALLBACK_MS;
      const resolvedRects = recentPreciseRects
        ? dedupeRects([...lastPreciseRectsRef.current, ...nextRects.filter((rect) => rect.kind === "display")])
        : nextRects;

      windowRectsRef.current = resolvedRects;
      setWindowRects(resolvedRects);
      
      const { hasSelected, isSelecting, isDragging, isResizing } = interactionStateRef.current;
      if (!hasSelected && !isSelecting && !isDragging && !isResizing) {
        const mouse = lastMouseRef.current;
        setHoverCandidateList(
          getDetectionCandidatesAt(
            mouse.x,
            mouse.y,
            windowRectsRef.current,
            analysisImageDataRef.current,
            configRef.current.enableVisualDetection === true,
            configRef.current.visualDetectionSensitivity || 3
          )
        );
      }
      triggerRender();
    } catch {
      const canReusePreciseRects =
        lastPreciseRectsRef.current.length > 0
        && performance.now() - lastPreciseRectsAtRef.current <= WINDOW_RECT_STALE_FALLBACK_MS;
      if (canReusePreciseRects) {
        windowRectsRef.current = lastPreciseRectsRef.current;
        setWindowRects(lastPreciseRectsRef.current);
        triggerRender();
      } else {
        windowRectsRef.current = [];
        setWindowRects([]);
      }
    } finally {
      rectQueryPendingRef.current = false;
    }
  };

  const clearWindowRects = () => {
    windowRectsRef.current = [];
    setWindowRects([]);
    setHoverCandidate(null);
    setHoverCandidates([]);
    hoverCandidatesRef.current = [];
    hoverCandidateIndexRef.current = 0;
    hoverCandidatesSignatureRef.current = "";
    lastRectQueryRef.current = 0;
    rectQueryPendingRef.current = false;
    lastPreciseRectsRef.current = [];
    lastPreciseRectsAtRef.current = 0;
  };

  return {
    windowRects,
    hoverRect,
    hoverCandidates,
    windowRectsRef,
    hoverRectRef,
    hoverCandidatesRef,
    hoverCandidateIndexRef,
    setHoverCandidate,
    setHoverCandidateList,
    loadWindowRects,
    clearWindowRects,
  };
}
