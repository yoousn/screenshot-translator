export type OcrModelProfile = "balanced" | "accurate";

export type OcrRuntimeReadinessStep = {
  id: string;
  ready: boolean;
  severity: "success" | "warning" | "error" | string;
  label: string;
  description: string;
  nextAction: string;
};

export type RapidOcrModelVersion = "v6" | "v5" | "v4";

export type RapidOcrStatus = {
  ready: boolean;
  runnerReady?: boolean;
  runtimeInferenceReady?: boolean;
  modelPacksReady?: boolean;
  activeModelsReady?: boolean;
  selfTestReady?: boolean;
  workerEnabled?: boolean;
  workerRunning?: boolean;
  worker?: {
    enabled?: boolean;
    running?: boolean;
    pid?: number;
    runnerKind?: string;
    runnerPath?: string;
    lastError?: string | null;
    cachedEngines?: Array<{ lang?: string; version?: string; modelRoot?: string }>;
    status?: Record<string, unknown>;
  } | null;
  runtime: "rapidocr" | string;
  engine?: string;
  runnerKind?: string;
  runnerPath?: string;
  runtimeVersion?: string;
  modelSetVersion?: string;
  rapidOcrModelVersion: RapidOcrModelVersion;
  modelDir: string;
  modelRoot?: string;
  missingModelFiles?: string[];
  defaultSourceLanguage: "auto";
  defaultProfile?: OcrModelProfile;
  lastError?: string | null;
  probeTimings?: Record<string, unknown> | null;
  supportedModelVersions?: RapidOcrModelVersion[];
  readinessSteps?: OcrRuntimeReadinessStep[];
};

export type RapidOcrSelfTestResult = {
  ok: boolean;
  testedAt: string;
  runtime: "rapidocr" | string;
  modelVersion: RapidOcrModelVersion;
  message: string;
  timings?: Record<string, unknown> | null;
  samples: Array<{ id: string; ok: boolean; confidence?: number; modelId?: string }>;
};

export type RapidOcrModelInstallResult = {
  ok: boolean;
  modelRoot: string;
  source?: {
    name?: string;
    docsUrl?: string;
    modelRepositoryUrl?: string;
    package?: string;
  };
  warmResult?: Record<string, unknown>;
  probeResults?: Record<string, unknown>;
  missingModelFiles?: {
    v6?: string[];
    v5?: string[];
    v4?: string[];
  };
  elapsedMs?: number;
};

export type RapidOcrModelInstallProgress = {
  phase: string;
  detail?: string;
  percent: number;
  status: "active" | "success" | "exception";
};
