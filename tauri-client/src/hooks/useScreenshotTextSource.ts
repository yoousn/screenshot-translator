import { useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Rect } from "../types/screenshot";
import { getPhysicalSelection } from "../utils/screenshotImage";

export type TextSourceElement = {
  text?: string;
  x?: number;
  y?: number;
  w?: number;
  h?: number;
};

export type TextSourceSnapshot = {
  status?: string;
  capturedAt?: string;
  screen?: { x?: number; y?: number; w?: number; h?: number };
  timings?: { totalMs?: number };
  elements?: TextSourceElement[];
};

interface UseScreenshotTextSourceProps {
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  imageRef: React.RefObject<HTMLImageElement | null>;
  rectRef: React.RefObject<Rect>;
}

export function useScreenshotTextSource({
  canvasRef,
  imageRef,
  rectRef,
}: UseScreenshotTextSourceProps) {
  const textSourceSnapshotPromiseRef = useRef<Promise<TextSourceSnapshot | null> | null>(null);

  const sleep = (ms: number) => new Promise<void>((resolve) => window.setTimeout(resolve, ms));

  const readTextSourceSnapshot = async (timeoutMs = 80): Promise<TextSourceSnapshot | null> => {
    const deadline = performance.now() + timeoutMs;
    let latest: TextSourceSnapshot | null = null;
    while (performance.now() <= deadline) {
      try {
        latest = await invoke<TextSourceSnapshot>("get_text_source_snapshot");
        if (latest?.status && latest.status !== "pending") return latest;
      } catch {
        return latest;
      }
      await sleep(12);
    }
    return latest;
  };

  const primeTextSourceSnapshot = (reason: string, timeoutMs = 120) => {
    const promise = readTextSourceSnapshot(timeoutMs).then((snapshot) => {
      if (snapshot?.status === "success") {
        console.info("[Text Source Snapshot]", reason, {
          elements: snapshot.elements?.length || 0,
          timings: snapshot.timings,
        });
      }
      return snapshot;
    });
    textSourceSnapshotPromiseRef.current = promise;
    return promise;
  };

  const buildTextSourceBlocksForSelection = (snapshot: TextSourceSnapshot | null, selection: Rect) => {
    if (!snapshot || snapshot.status !== "success" || !snapshot.screen || !snapshot.elements?.length) {
      return { blocks: [], maxElementCoverage: 0, maxSelectionCoverage: 0, matchedRawCount: 0, rejectedRawCount: 0, rejectedAggregateCount: 0 };
    }
    let physicalSelection: Rect;
    try {
      physicalSelection = getPhysicalSelection({
        canvas: canvasRef.current,
        image: imageRef.current as any,
        rect: selection,
      });
    } catch {
      return { blocks: [], maxElementCoverage: 0, maxSelectionCoverage: 0, matchedRawCount: 0, rejectedRawCount: 0, rejectedAggregateCount: 0 };
    }
    const result = (window as any).buildTextSourceBlocksForPhysicalSelection 
      ? (window as any).buildTextSourceBlocksForPhysicalSelection(snapshot.elements, snapshot.screen, physicalSelection)
      : { blocks: [], maxElementCoverage: 0, maxSelectionCoverage: 0, matchedRawCount: 0, rejectedRawCount: 0, rejectedAggregateCount: 0 };
    return result;
  };

  const getTextSourceBlocksForCurrentSelection = async (timeoutMs = 80) => {
    const started = performance.now();
    const snapshot = await Promise.race([
      textSourceSnapshotPromiseRef.current || readTextSourceSnapshot(timeoutMs),
      sleep(timeoutMs).then(() => null),
    ]);
    const textSourceSelection = buildTextSourceBlocksForSelection(snapshot, rectRef.current);
    const blocks = textSourceSelection.blocks;
    const charCount = blocks.reduce((sum: number, block: any) => sum + block.text.length, 0);
    const usable = blocks.length > 0 && charCount >= 2 && textSourceSelection.maxElementCoverage >= 0.55;
    return {
      usable,
      blocks: usable ? blocks : [],
      elapsedMs: Math.round(performance.now() - started),
      status: snapshot?.status || "empty",
      rawCount: snapshot?.elements?.length || 0,
      matchedRawCount: textSourceSelection.matchedRawCount,
      rejectedRawCount: textSourceSelection.rejectedRawCount,
      rejectedAggregateCount: textSourceSelection.rejectedAggregateCount,
      maxElementCoverage: Number(textSourceSelection.maxElementCoverage.toFixed(3)),
      maxSelectionCoverage: Number(textSourceSelection.maxSelectionCoverage.toFixed(3)),
    };
  };

  return {
    textSourceSnapshotPromiseRef,
    readTextSourceSnapshot,
    primeTextSourceSnapshot,
    getTextSourceBlocksForCurrentSelection,
  };
}
