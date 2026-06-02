export type RecordingInfo = {
  ffmpegFound: boolean;
  ffmpegPath?: string;
  isRecording?: boolean;
  audioDevices?: string[];
};

export type FfmpegReleaseInfo = {
  tag: string;
  assetName: string;
  downloadUrl: string;
  installDir: string;
};

export type FfmpegProgress = {
  phase: string;
  downloaded: number;
  total?: number;
  percent: number;
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
  targetLang: string;
  onTargetLangChange: (targetLang: string) => void;
};
