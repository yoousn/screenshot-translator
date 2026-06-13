export interface Config {
  serverUrl?: string;
  lanServerUrl?: string;
  preferLanServer?: boolean;
  clientToken?: string;
  hotkey?: string;
  translateHotkey?: string;
  recordingHotkey?: string;
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  localOcrTimeoutMs?: number;
  translationTimeoutMs?: number;
  rapidOcrWorkerEnabled?: boolean;
  rapidOcrModelVersion?: "v6" | "v5" | "v4";
  targetLang?: string;
  channel?: string;
  enableUiControlDetection?: boolean;
  enableVisualDetection?: boolean;
  detectionBorderWidth?: number;
  toolbarButtonGap?: number;
  visualDetectionSensitivity?: number;
  imageSaveNamePrefix?: string;
  imageSaveNameFormat?: string;
  imageSaveDefaultDir?: string;
  imageSaveRememberLastDir?: boolean;
  imageSaveLastDir?: string;
  edgeSnapEnabled?: boolean;
  edgeSnapDistance?: number;
  enableMagnifier?: boolean;        // default true
  enableColorPicker?: boolean;      // default true
  enablePreciseSelection?: boolean; // default true
}
