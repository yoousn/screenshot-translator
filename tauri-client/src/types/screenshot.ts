export type Rect = { x: number; y: number; w: number; h: number; kind?: "window" | "control" | "taskbar" | "display" | "visual" };
export type AnnotationTool = "rect" | "circle" | "mosaic" | "arrow" | "text" | "brush";
export type Point = { x: number; y: number };
export type Annotation = { type: AnnotationTool; rect: Rect; points?: Point[]; text?: string; color?: string; size?: number };
export type EditingTextDraft = { x: number; y: number; value: string; targetIndex: number | null } | null;
export type TranslatePair = { o: string; t: string; status?: "translated" | "preserved" | "untranslated" };

export type ScreenshotPhysicalBounds = { x: number; y: number; width: number; height: number };

export type ScreenshotUpdatedPayload = string | {
  kind?: "file" | "base64" | "memory" | "rgba";
  path?: string;
  base64?: string;
  bytes?: number;
  width?: number;
  height?: number;
  mode?: string;
  sessionId?: string;
  physicalBounds?: ScreenshotPhysicalBounds;
};

export type DxgiOutputCandidateEvidence = {
  adapterIndex?: number | null;
  outputIndex?: number | null;
  desktopBounds?: ScreenshotPhysicalBounds | null;
};

export type DxgiRankedOutputEvidence = DxgiOutputCandidateEvidence & {
  rank?: number | null;
  intersectionBounds: ScreenshotPhysicalBounds | null;
  intersectionArea?: number | null;
  intersectionRatio?: number | null;
  containsSelectionCenter?: boolean | null;
  selectable?: boolean | null;
  selected?: boolean | null;
  rejectionReason: string | null;
};

export type DxgiOutputRankingEvidence = {
  policyVersion?: number | null;
  rankingPolicy?: string | null;
  requestedBounds?: ScreenshotPhysicalBounds | null;
  selectionCenter: Point | null;
  candidateCount?: number | null;
  selectedRank: number | null;
  selectedOutput: DxgiOutputCandidateEvidence | null;
  rankedOutputs?: DxgiRankedOutputEvidence[];
  persistentHandleExposed?: boolean | null;
} | null;

export type SelectedPngEvidence = {
  pngWidth?: number | null;
  pngHeight?: number | null;
  pngByteLen?: number | null;
  pngFingerprint?: string | null;
  sourceWidth?: number | null;
  sourceHeight?: number | null;
  crop?: ScreenshotDiagnosticRect | null;
  selectedOnlyPng?: boolean | null;
  dimensionsMatchCrop?: boolean | null;
  decodedRgbaByteLenExpected?: number | null;
};

export type WgcSelectedOutputFakeSinkAcceptance = {
  ok?: boolean | null;
  source?: string | null;
  diagnosticOnly?: boolean | null;
  readinessChanged?: boolean | null;
  altAChanged?: boolean | null;
  persistentHandleExposed?: boolean | null;
  wgcSelectedPngEvidencePresent?: boolean | null;
  fakeSinkCopyAccepted?: boolean | null;
  sink?: string | null;
  sinkCalls?: number | null;
  selectedOnlyPng?: boolean | null;
  pngByteLen?: number | null;
  copiedPngByteLen?: number | null;
  effect?: {
    action?: unknown;
    target?: unknown;
    format?: unknown;
    selectedOnlyPng?: boolean | null;
    pngByteLen?: number | null;
    copiedToClipboard?: boolean | null;
    saveInvoked?: boolean | null;
    ocrInvoked?: boolean | null;
    translationInvoked?: boolean | null;
    copyOnly?: boolean | null;
  } | null;
  error?: string | null;
  scope?: string | null;
};

export type WgcSelectedMonitorFrameEvidence = {
  diagnosticOnly?: boolean | null;
  requestedBoundsPhysical: ScreenshotPhysicalBounds | null;
  targetMonitorBoundsPhysical: ScreenshotPhysicalBounds | null;
  framepoolSizeSource?: string | null;
  frameWidth: number | null;
  frameHeight: number | null;
  frameMatchesTargetMonitorBounds?: boolean | null;
  selectedCropWithinFrame?: boolean | null;
  selectedPngProduced?: boolean | null;
  readbackBytesPresent?: boolean | null;
  persistentHandleExposed?: boolean | null;
  readinessChanged?: boolean | null;
};

export type WgcSelectedFrameEvidence = {
  diagnosticOnly?: boolean | null;
  requestedBoundsPhysical: ScreenshotPhysicalBounds | null;
  targetMonitorBoundsPhysical: ScreenshotPhysicalBounds | null;
  framepoolSizeSource?: string | null;
  frameAcquired?: boolean | null;
  frameId?: number | null;
  frameWidth: number | null;
  frameHeight: number | null;
  requestedSessionWidth?: number | null;
  requestedSessionHeight?: number | null;
  dimensionsMatchSession?: boolean | null;
  frameMatchesTargetMonitorBounds?: boolean | null;
  selectedCropWithinFrame?: boolean | null;
  format: unknown;
  source: unknown;
  textureMetadataPresent?: boolean | null;
  stagingReadbackPresent?: boolean | null;
  readbackBytesPresent?: boolean | null;
  readbackByteLen: number | null;
  selectedPngEvidence: SelectedPngEvidence | null;
  selectedPngProduced?: boolean | null;
  persistentHandleExposed?: boolean | null;
  readinessChanged?: boolean | null;
  scope?: string | null;
} | null;

export type ScreenshotDiagnosticImageBounds = { width?: number | null; height?: number | null };

export type ScreenshotDiagnosticRect = { x?: number | null; y?: number | null; width?: number | null; height?: number | null };

export type ScreenshotSelectedReadbackPlan = {
  diagnosticOnly?: boolean | null;
  readinessChanged?: boolean | null;
  backend?: string | null;
  status?: "planned" | "failed" | string | null;
  requestedBoundsPhysical?: ScreenshotPhysicalBounds | null;
  targetBoundsPhysical?: {
    known?: boolean | null;
    source?: string | null;
    bounds?: ScreenshotPhysicalBounds | null;
  } | null;
  outputFrameBounds?: ScreenshotDiagnosticImageBounds | null;
  mapping?: {
    status?: "planned" | string | null;
    desktopSelection?: ScreenshotDiagnosticRect | null;
    monitorLocalSelection?: ScreenshotDiagnosticRect | null;
    crop?: ScreenshotDiagnosticRect | null;
    wasClampedToMonitorOrFrame?: boolean | null;
    frameMatchesMonitorBounds?: boolean | null;
  } | null;
  cropOverflowPhysical?: { left?: number | null; top?: number | null; right?: number | null; bottom?: number | null } | null;
  cropWithinTargetMonitor?: boolean | null;
  requestedTargetIntersectionRatio?: number | null;
  framepool?: {
    requestedSize?: ScreenshotDiagnosticImageBounds | null;
    source?: string | null;
    matchesRequestedBounds?: boolean | null;
    matchesTargetBounds?: boolean | null;
  } | null;
  captureItemExpectedSize?: ScreenshotDiagnosticImageBounds | null;
  mismatches?: Record<string, boolean | null> | null;
  errorCode?: string | null;
  error?: string | null;
  selectedOutputReadyPlanningOnly?: boolean | null;
  scope?: string | null;
} | null;

export type ScreenshotLatestPayloadSummary = {
  latestPayloadPresent?: boolean | null;
  sessionId?: string | null;
  captureWidth?: number | null;
  captureHeight?: number | null;
};

export type NativeWgcMonitorTargetDiagnosticResponse = {
  ok?: boolean;
  valid?: boolean;
  boundsSource?: string;
  latestPayload?: ScreenshotLatestPayloadSummary;
  bounds?: ScreenshotPhysicalBounds;
  resolution?: unknown;
  validation?: unknown;
  selectedReadbackPlan?: ScreenshotSelectedReadbackPlan;
  diagnosticOnly?: boolean;
  persistentHandleExposed?: boolean;
  readinessChanged?: boolean;
  attemptedRealWgcApi?: boolean;
  frameCaptureAttempted?: boolean;
  frameCaptureConfirmed?: boolean;
  error?: string | null;
  scope?: string | null;
};

export type NativeDxgiOutputRankingFragment = {
  outputRanking?: DxgiOutputRankingEvidence;
};

export type NativeWgcSelectedFrameEvidenceFragment = {
  selectedMonitorFrameEvidence?: WgcSelectedMonitorFrameEvidence | null;
  selectedFrameEvidence?: WgcSelectedFrameEvidence;
};

export type NativeWgcSelectedOutputBoundsRequest = ScreenshotPhysicalBounds & {
  explicitOptIn?: boolean;
  allowRealDxgiApi?: boolean;
};

export type NativeWgcSelectedOutputClipboardAcceptanceRequest = {
  bounds: NativeWgcSelectedOutputBoundsRequest;
  explicitOptIn: boolean;
  allowRealWgcApi: boolean;
  allowFakeClipboardSink: boolean;
  allowRealClipboard: boolean;
  frameTimeoutMs?: number;
  includeCursor?: boolean;
  requireBorder?: boolean;
  bufferCount?: number;
  validateTarget?: boolean;
  includeSelectedPngBase64?: boolean;
  allowFileWrite?: boolean;
  savePath?: string;
};

export type NativeWgcSelectedOutputClipboardAcceptanceResponse = NativeWgcMonitorSessionSmokeResponse & {
  attempted?: boolean;
  stage?: string;
  requestedBounds?: ScreenshotPhysicalBounds | null;
  selectedPngEvidence?: SelectedPngEvidence | null;
  selectedOutputEffectConfirmed?: boolean;
  realClipboardAttempted?: boolean;
  realClipboardVerified?: boolean;
  clipboardReadbackAttempted?: boolean;
  clipboardReadbackConfirmed?: boolean;
  explicitSelectionDiagnostic?: boolean;
  latestFallbackRejected?: boolean;
  requiresExplicitRequestBounds?: boolean;
  sink?: unknown;
  receipt?: unknown;
  selectedPngBase64?: string | null;
  selectedFile?: unknown;
  acceptanceError?: string | null;
};
export type NativeWgcMonitorSessionSmokeResponse = {
  ok?: boolean;
  valid?: boolean;
  boundsSource?: string;
  latestPayload?: ScreenshotLatestPayloadSummary;
  bounds?: ScreenshotPhysicalBounds;
  sessionBounds?: ScreenshotPhysicalBounds;
  resolution?: unknown;
  validation?: unknown;
  selectedReadbackPlan?: ScreenshotSelectedReadbackPlan;
  diagnosticOnly?: boolean;
  persistentHandleExposed?: boolean;
  readinessChanged?: boolean;
  attemptedRealWgcApi?: boolean;
  frameCaptureAttempted?: boolean;
  frameCaptureConfirmed?: boolean;
  selectedOutputFakeSinkAcceptance?: WgcSelectedOutputFakeSinkAcceptance | null;
  error?: string | null;
  scope?: string | null;
  session?: {
    state?: string;
    attemptedRealWgcApi?: boolean;
    acquiredFrame?: boolean;
    selectedMonitorFrameEvidence?: WgcSelectedMonitorFrameEvidence;
    selectedFrameEvidence?: WgcSelectedFrameEvidence;
    selectedPngEvidence?: SelectedPngEvidence | null;
    selectedPngProduced?: boolean;
    selectedOutputFakeSinkAcceptance?: WgcSelectedOutputFakeSinkAcceptance | null;
    error?: string | null;
  } | null;
};

export type NativeDxgiFrameInfoProbePathResponse = {
  path?: string;
  attempted?: boolean;
  ok?: boolean;
  outputBounds?: ScreenshotPhysicalBounds | null;
  adapterIndex?: number | null;
  outputIndex?: number | null;
  outputRanking?: DxgiOutputRankingEvidence;
};

export interface NativeScreenshotDiagnosticsStatus {
  gpuPlan?: {
    primaryStatus?: string | null;
    primaryFallback?: string | null;
  } | null;
  d3d11?: {
    capability?: {
      status?: string | null;
      fallback?: string | null;
    } | null;
  } | null;
  wgc?: {
    nativeApi?: {
      isSupported?: boolean | null;
    } | null;
  } | null;
  dxgi?: {
    nativeApi?: {
      reason?: string | null;
    } | null;
  } | null;
}

export interface OcrBlock {
  text: string;
  confidence: number;
  box_coords: [number, number][];
}
