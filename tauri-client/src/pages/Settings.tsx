import { Form, Space } from "antd";
import useSettingsController from "../hooks/useSettingsController";
import SettingsPageHeader from "../components/settings/SettingsPageHeader";
import TranslationServiceCard from "../components/settings/TranslationServiceCard";
import TranslationChannelCard from "../components/settings/TranslationChannelCard";
import ScreenshotRecognitionCard from "../components/settings/ScreenshotRecognitionCard";
import SystemHotkeyCard from "../components/settings/SystemHotkeyCard";
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
        hotkey: "Alt+A",
        translateHotkey: "Alt+T",
        serverUrl: DEFAULT_TRANSLATION_SERVICE_URL,
        channel: "google",
        targetLang: "zh",
      }}
      onFinish={controller.onFinish}
      onValuesChange={controller.handleFormChange}
      requiredMark={false}
      style={{ maxWidth: 800, margin: "0 auto" }}
    >
      <SettingsPageHeader saving={controller.isSaving} />
      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <TranslationServiceCard />
        <TranslationChannelCard
          currentChannel={controller.currentChannel}
          availableModels={controller.availableModels}
          isFetchingModels={controller.isFetchingModels}
          isTestingBaidu={controller.isTestingBaidu}
          isTestingNewApi={controller.isTestingNewApi}
          fetchModels={controller.fetchModels}
          testChannel={controller.testChannel}
        />
        <ScreenshotRecognitionCard />
        <SystemHotkeyCard form={form} onRestoreDefaultHotkeys={controller.restoreDefaultHotkeys} />
      </Space>
    </Form>
  );
}
