import type { FormInstance } from "antd";

export type SettingsForm = FormInstance<any>;

export type TranslationChannel = "baidu" | "new-api" | "deepl";
export type TranslationChannelId = "google" | TranslationChannel;

export type TranslationChannelTestStatus = {
  status: "testing" | "passed" | "failed";
  message?: string;
  serviceUrl?: string;
  testedAt?: string;
};

export type TranslationChannelTestStatuses = Partial<Record<TranslationChannel, TranslationChannelTestStatus>>;

export type TranslationChannelActivationStatus = {
  channel?: TranslationChannelId;
  status: "idle" | "testing" | "passed" | "failed";
  message?: string;
  serviceUrl?: string;
  testedAt?: string;
};

export type ServerChannelStatus = {
  activeChannel?: string;
  serviceUrl?: string;
  checkedAt?: string;
  error?: string;
};

export type SettingsControllerState = {
  isSaving: boolean;
  isActivatingGoogle: boolean;
  isTestingBaidu: boolean;
  isTestingNewApi: boolean;
  isTestingDeepl: boolean;
  isFetchingModels: boolean;
  availableModels: string[];
  currentChannel: string;
  activeChannel: string;
  channelDraftDirty: Partial<Record<TranslationChannelId, boolean>>;
  channelActivationStatus: TranslationChannelActivationStatus;
  channelTestStatuses: TranslationChannelTestStatuses;
  serverChannelStatus: ServerChannelStatus;
  fetchModels: () => void;
  activateGoogleChannel: () => void;
  saveAndEnableChannel: (channel: TranslationChannelId) => void;
  testChannel: (channel: TranslationChannel) => void;
  restoreDefaultHotkeys: () => void;
  updateHotkeyValue: (field: "hotkey" | "translateHotkey" | "recordingHotkey", value: string) => void;
  clearScreenshotHotkey: () => void;
  clearTranslateHotkey: () => void;
  clearRecordingHotkey: () => void;
};
