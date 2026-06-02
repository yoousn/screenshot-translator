export type OcrModelProfile = "balanced" | "accurate";
export type OcrModelPackStatus = "not-installed" | "downloading" | "downloaded" | "verified" | "installing" | "self-testing" | "installed" | "update-available" | "download-failed" | "verify-failed" | "install-failed" | "self-test-failed" | "broken";
export type OcrModelType = "detection" | "classification" | "recognition" | "language-detector" | "postprocess-resource";
export type OcrModelScript = "cjk" | "latin" | "hangul" | "cyrillic" | "arabic" | "thai" | "mixed" | "unknown";

export type LocalizedText = {
  "zh-CN": string;
  "en-US": string;
};

export type OcrModelSource = {
  provider: string;
  url: string;
  license: string;
};

export type OcrModelSourcePolicy = {
  policyVersion: string;
  productionDownloadProvider: string;
  allowedProviderTiers: Array<{
    id: string;
    label: string;
    productionDownloadAllowed: boolean;
    description: string;
  }>;
  requiredFields: string[];
  rules: string[];
  upstreamReferences: Array<{ provider: string; purpose: string; status: string }>;
};

export type OcrModelSourceReadiness = {
  ready: boolean;
  requiredModels: number;
  configuredModels: number;
  configuredModelIds: string[];
  pendingModelIds: string[];
  issues: OcrModelManifestIssue[];
  policy: OcrModelSourcePolicy;
  nextAction: string;
};

export type OcrModelPack = {
  id: string;
  name: LocalizedText;
  profile: OcrModelProfile;
  required: boolean;
  languages: string[];
  scripts: OcrModelScript[];
  modelIds: string[];
  status: OcrModelPackStatus;
  lastSelfTestAt?: string | null;
  error?: string;
};

export type OcrModelDescriptor = {
  id: string;
  type: OcrModelType;
  engine: "onnxruntime";
  profile: OcrModelProfile;
  scripts: OcrModelScript[];
  languages: string[];
  path: string;
  dictPath?: string;
  version: string;
  source: OcrModelSource;
  sha256: string;
  size: number;
  required: boolean;
  status: OcrModelPackStatus;
};

export type OcrModelManifest = {
  schemaVersion: 1;
  runtime: "ysn-ocr-runtime";
  runtimeVersion: string;
  modelSetVersion: string;
  defaultSourceLanguage: "auto";
  defaultProfile: OcrModelProfile;
  installedAt?: string | null;
  lastSelfTestAt?: string | null;
  packs: OcrModelPack[];
  models: OcrModelDescriptor[];
  sourcePolicy?: OcrModelSourcePolicy;
};

export type OcrModelPackHealth = {
  installed: number;
  required: number;
  missing: string[];
  broken: string[];
  updateAvailable: string[];
  ready: boolean;
};

export type OcrModelManifestIssue = {
  severity: "error" | "warning";
  code: string;
  message: string;
  packId?: string;
  modelId?: string;
};

export type OcrActiveModelHealth = {
  artifactType?: "model" | "dictionary" | string;
  modelId: string;
  packId?: string;
  relativePath: string;
  activePath?: string | null;
  exists: boolean;
  expectedSha256: string;
  actualSha256?: string | null;
  sourceProvider: string;
  productionSource: boolean;
  ok: boolean;
  issues: Array<{ code: string; message: string }>;
};

export type OcrRuntimeReadinessStep = {
  id: string;
  ready: boolean;
  severity: "success" | "warning" | "error" | string;
  label: string;
  description: string;
  nextAction: string;
};

export type YsnOcrRuntimeStatus = {
  ready: boolean;
  sourceReady?: boolean;
  manifestReady?: boolean;
  modelPacksReady?: boolean;
  activeModelsReady?: boolean;
  runtimeInferenceReady?: boolean;
  selfTestReady?: boolean;
  readinessSteps?: OcrRuntimeReadinessStep[];
  runtime: string;
  runtimeVersion: string;
  modelSetVersion: string;
  modelDir: string;
  defaultSourceLanguage: "auto";
  defaultProfile: OcrModelProfile;
  installedRequiredPacks: number;
  requiredPacks: number;
  brokenPacks: string[];
  activeModelHealth?: OcrActiveModelHealth[];
  activeModelIssues?: OcrActiveModelHealth[];
  manifestIssues?: OcrModelManifestIssue[];
  sourceReadiness?: OcrModelSourceReadiness;
  manifest: OcrModelManifest;
  implementationStatus?: string;
};

export type YsnOcrSelfTestResult = {
  ok: boolean;
  modelPacksReady?: boolean;
  runtimeInferenceReady?: boolean;
  testedAt: string;
  runtime: string;
  message: string;
  manifestIssues?: OcrModelManifestIssue[];
  missingActiveModels?: string[];
  samples: Array<{ id: string; ok: boolean; confidence?: number; modelId?: string }>;
};

export type YsnOcrModelPackCommandResult = {
  ok: boolean;
  packId: string;
  modelDir: string;
  status: string;
  message: string;
  operationId?: string;
  phase?: string;
  recoverable?: boolean;
  nextAction?: string;
};

export type YsnOcrManagedSourceImportResult = {
  ok: boolean;
  indexPath: string;
  manifestPath: string;
  updatedCount: number;
  updatedModels: string[];
  sourceReadiness?: OcrModelSourceReadiness;
  message: string;
};

export type YsnOcrManagedSourceTemplateResult = {
  ok: boolean;
  templatePath: string;
  templateDir: string;
  modelCount: number;
  message: string;
};

export type YsnOcrManagedSourceDryRunPackPlan = {
  packId: string;
  ok: boolean;
  blocker?: string | null;
  modelCount?: number;
  downloadPlan: Array<{
    modelId: string;
    url: string;
    sha256: string;
    relativePath: string;
    size: number;
    packId: string;
    provider: string;
    license: string;
    version: string;
  }>;
};

export type YsnOcrManagedSourceDryRunResult = {
  ok: boolean;
  indexPath: string;
  packId?: string | null;
  result: {
    ok: boolean;
    mode: "dry-run";
    wouldWriteManifest: false;
    wouldActivateModels: false;
    importResult?: {
      updatedCount?: number;
      updatedModels?: string[];
      sourceReadiness?: OcrModelSourceReadiness;
    };
    sourceReadiness?: OcrModelSourceReadiness;
    packPlans: YsnOcrManagedSourceDryRunPackPlan[];
    publishLayout?: Record<string, any>;
  };
  message: string;
};
