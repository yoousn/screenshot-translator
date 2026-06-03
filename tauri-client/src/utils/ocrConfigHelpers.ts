export interface LocalConfig {
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  localOcrTimeoutMs?: number;
  targetLang?: string;
  channel?: string;
  newApiPrompt?: string;
  newApiDomain?: string;
  serverUrl?: string;
  lanServerUrl?: string;
  preferLanServer?: boolean;
  rapidOcrModelVersion?: "v5" | "v4";
  rapidOcrMode?: "auto" | "full" | "latin";
  rapidOcrRunnerPath?: string;
  rapidOcrWorkerEnabled?: boolean;
  appLanguage?: string;
  recordingFfmpegPath?: string;
}
