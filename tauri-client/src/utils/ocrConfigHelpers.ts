export interface LocalConfig {
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  localOcrTimeoutMs?: number;
  targetLang?: string;
  channel?: string;
  appLanguage?: string;
  recordingFfmpegPath?: string;
}
