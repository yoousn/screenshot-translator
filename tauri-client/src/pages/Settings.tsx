import { Form, Space } from "antd";
import useSettingsController from "../hooks/useSettingsController";
import SettingsPageHeader from "../components/settings/SettingsPageHeader";
import TranslationServiceCard from "../components/settings/TranslationServiceCard";
import TranslationChannelCard from "../components/settings/TranslationChannelCard";
import ScreenshotRecognitionCard from "../components/settings/ScreenshotRecognitionCard";
import ImageSaveSettingsCard from "../components/settings/ImageSaveSettingsCard";
import SystemHotkeyCard from "../components/settings/SystemHotkeyCard";
import { DEFAULT_LLM_TRANSLATION_DOMAIN, DEFAULT_LLM_TRANSLATION_PROMPT } from "../utils/defaultTranslationPrompt";
import { DEFAULT_TRANSLATION_SERVICE_URL } from "../utils/translationService";

interface SettingsProps {
  onConfigSaved: () => void;
}

export default function Settings({ onConfigSaved }: SettingsProps) {
  const [form] = Form.useForm();
  const controller = useSettingsController(form, onConfigSaved);

  return (
    <Form
      form={form}
      layout="vertical"
      initialValues={{
        enableUiControlDetection: false,
        enableVisualDetection: false,
        detectionBorderWidth: 2,
        visualDetectionSensitivity: 3,
        useLocalOcr: true,
        fallbackToRemoteOcr: false,
        localOcrTimeoutMs: 5000,
        toolbarButtonGap: 6,
        imageSaveNamePrefix: "Ysn_",
        imageSaveNameFormat: "yyyyMMdd_HHmmss",
        imageSaveDefaultDir: "",
        imageSaveRememberLastDir: false,
        hotkey: "Alt+A",
        translateHotkey: "Alt+T",
        recordingHotkey: "Alt+R",
        serverUrl: DEFAULT_TRANSLATION_SERVICE_URL,
        lanServerUrl: "",
        preferLanServer: false,
        channel: "google",
        targetLang: "zh",
        newApiModel: "gemini-2.0-flash",
        newApiPrompt: DEFAULT_LLM_TRANSLATION_PROMPT,
        newApiDomain: DEFAULT_LLM_TRANSLATION_DOMAIN,
        deeplEndpoint: "https://api-free.deepl.com",
        deeplFormality: "default",
        edgeSnapEnabled: true,
        edgeSnapDistance: 8,
      }}
      onFinish={controller.onFinish}
      onValuesChange={controller.handleFormChange}
      requiredMark={false}
      style={{ width: "min(100%, 960px)", margin: "0 auto", paddingBottom: 24 }}
    >
      <SettingsPageHeader saving={controller.isSaving} />
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <TranslationServiceCard />
        <TranslationChannelCard
          form={form}
          currentChannel={controller.currentChannel}
          availableModels={controller.availableModels}
          isFetchingModels={controller.isFetchingModels}
          isTestingBaidu={controller.isTestingBaidu}
          isTestingNewApi={controller.isTestingNewApi}
          isTestingDeepl={controller.isTestingDeepl}
          channelTestStatuses={controller.channelTestStatuses}
          serverChannelStatus={controller.serverChannelStatus}
          isActivatingGoogle={controller.isActivatingGoogle}
          fetchModels={controller.fetchModels}
          testChannel={controller.testChannel}
          activateGoogleChannel={controller.activateGoogleChannel}
        />
        <ScreenshotRecognitionCard />
        <ImageSaveSettingsCard form={form} />
        <SystemHotkeyCard
          onRestoreDefaultHotkeys={controller.restoreDefaultHotkeys}
          onHotkeyChange={controller.updateHotkeyValue}
          onClearScreenshotHotkey={controller.clearScreenshotHotkey}
          onClearTranslateHotkey={controller.clearTranslateHotkey}
          onClearRecordingHotkey={controller.clearRecordingHotkey}
        />
      </Space>
    </Form>
  );
}
