import type { FormInstance } from "antd";

export type SettingsForm = FormInstance<any>;

export type TranslationChannel = "baidu" | "new-api";
export type TranslationChannelId = "google" | TranslationChannel;

export type TranslationChannelTestStatus = {
  status: "testing" | "passed" | "failed";
  message?: string;
  serviceUrl?: string;
  testedAt?: string;
};

export type TranslationChannelTestStatuses = Partial<Record<TranslationChannel, TranslationChannelTestStatus>>;

export type ServerChannelStatus = {
  activeChannel?: string;
  serviceUrl?: string;
  checkedAt?: string;
  error?: string;
};

export type SettingsControllerState = {
  isSaving: boolean;
  isTestingBaidu: boolean;
  isTestingNewApi: boolean;
  isFetchingModels: boolean;
  availableModels: string[];
  currentChannel: string;
  channelTestStatuses: TranslationChannelTestStatuses;
  serverChannelStatus: ServerChannelStatus;
  fetchModels: () => void;
  testChannel: (channel: TranslationChannel) => void;
  restoreDefaultHotkeys: () => void;
};
