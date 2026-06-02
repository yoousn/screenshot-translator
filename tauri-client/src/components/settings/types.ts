import type { FormInstance } from "antd";

export type SettingsForm = FormInstance<any>;

export type TranslationChannel = "baidu" | "new-api";

export type SettingsControllerState = {
  isSaving: boolean;
  isTestingBaidu: boolean;
  isTestingNewApi: boolean;
  isFetchingModels: boolean;
  availableModels: string[];
  currentChannel: string;
  fetchModels: () => void;
  testChannel: (channel: TranslationChannel) => void;
  restoreDefaultHotkeys: () => void;
};
