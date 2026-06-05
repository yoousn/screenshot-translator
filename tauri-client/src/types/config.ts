export interface Config {
  serverUrl?: string;
  lanServerUrl?: string;
  preferLanServer?: boolean;
  clientToken?: string;
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  localOcrTimeoutMs?: number;
  translationTimeoutMs?: number;
  rapidOcrWorkerEnabled?: boolean;
  targetLang?: string;
  channel?: string;
  enableUiControlDetection?: boolean;
  enableVisualDetection?: boolean;
  detectionBorderWidth?: number;
  toolbarButtonGap?: number;
  visualDetectionSensitivity?: number;
}
