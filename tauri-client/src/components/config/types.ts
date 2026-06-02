import type { ReactNode } from "react";
import type { GitHubAsset, GitHubRelease, LocalConfig, ProgressPayload, StatusResult } from "../../utils/ocrConfigHelpers";

export type FfmpegReleaseInfo = {
  tag: string;
  pageUrl?: string | null;
  assetName: string;
  downloadUrl: string;
  size?: number | null;
  installDir: string;
};

export type FfmpegProgress = {
  phase: string;
  downloaded: number;
  total?: number | null;
  percent: number;
};

export type RecordingInfo = {
  ffmpegFound: boolean;
  ffmpegPath?: string;
  isRecording: boolean;
  audioDevices: string[];
};

export type OcrRuntimePanelProps = {
  config: LocalConfig;
  status: StatusResult | null;
  statusTag: ReactNode;
  saving: boolean;
  checkingStatus: boolean;
  onSetConfig: (config: LocalConfig) => void;
  onSaveConfig: () => void;
  onChooseRuntimeDir: () => void;
  onCheckStatus: () => void;
  onOpenRuntimeDir: () => void;
};

export type CompatibilityRuntimePanelProps = {
  config: LocalConfig;
  latest: GitHubRelease | null;
  latestAsset: GitHubAsset | null;
  checking: boolean;
  downloading: boolean;
  movingDir: boolean;
  hasUpdate: boolean;
  downloadSize?: number;
  downloadProgress: ProgressPayload | null;
  onCheckLatest: () => void;
  onDownloadLatest: () => void;
  onOpenRepo: () => void;
  onOpenReleaseNotes: (url: string) => void;
  onMoveRuntimeDir: () => void;
};

export type RecordingDependencyPanelProps = {
  ffmpegPath: string;
  defaultVideoDir: string;
  ffmpegRelease: FfmpegReleaseInfo | null;
  ffmpegProgress: FfmpegProgress | null;
  recordingInfo: RecordingInfo | null;
  checkingFfmpeg: boolean;
  checkingRecordingInfo: boolean;
  downloadingFfmpeg: boolean;
  onSetFfmpegPath: (path: string) => void;
  onSaveFfmpegPath: () => void;
  onChooseFfmpegPath: () => void;
  onCheckFfmpegRelease: () => void;
  onCheckRecordingInfo: () => void;
  onDownloadFfmpeg: () => void;
  onOpenFfmpegRepo: () => void;
  onOpenFfmpegDir: () => void;
  onOpenVideoDir: () => void;
};

export type TranslationLanguagePanelProps = {
  targetLang?: string;
  onTargetLangChange: (language: string) => void;
};
