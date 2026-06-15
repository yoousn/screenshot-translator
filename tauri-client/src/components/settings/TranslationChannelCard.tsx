import { AutoComplete, Button, Card, Col, Form, Input, Row, Select, Space, Typography } from "antd";
import {
  GlobalOutlined,
  SyncOutlined,
} from "@ant-design/icons";
import { useI18n } from "../../i18n";
import { getChannelOptions, getDeepLEndpointOptions, getTargetLangOptions } from "./settingsOptions";
import type {
  SettingsControllerState,
  SettingsForm,
  TranslationChannelId,
} from "./types";
import { DEFAULT_LLM_TRANSLATION_DOMAIN, DEFAULT_LLM_TRANSLATION_PROMPT } from "../../utils/defaultTranslationPrompt";

const { Text } = Typography;
const { TextArea } = Input;

type TranslationChannelCardProps = Pick<
  SettingsControllerState,
  | "currentChannel"
  | "activeChannel"
  | "channelDraftDirty"
  | "channelActivationStatus"
  | "isActivatingGoogle"
  | "availableModels"
  | "isFetchingModels"
  | "isTestingBaidu"
  | "isTestingNewApi"
  | "isTestingDeepl"
  | "fetchModels"
  | "saveAndEnableChannel"
> & {
  form: SettingsForm;
};

const channelIds: TranslationChannelId[] = ["google", "baidu", "new-api", "deepl"];

const getChannelLabel = (labels: Record<string, string>, channel: string) => {
  if (channel === "baidu") return labels.channelBaidu;
  if (channel === "new-api") return labels.channelNewApi;
  if (channel === "deepl") return labels.channelDeepL;
  return labels.channelGoogle;
};

const formatTemplate = (template: string | undefined, channelLabel: string, fallback: string) => (
  (template || fallback).replace("{channel}", channelLabel)
);

export default function TranslationChannelCard({
  form,
  currentChannel,
  activeChannel,
  channelDraftDirty,
  channelActivationStatus,
  isActivatingGoogle,
  availableModels,
  isFetchingModels,
  isTestingBaidu,
  isTestingNewApi,
  isTestingDeepl,
  fetchModels,
  saveAndEnableChannel,
}: TranslationChannelCardProps) {
  const { text } = useI18n();
  const labels = text.settings;

  const watchedChannel = Form.useWatch("channel", form) as TranslationChannelId | undefined;
  const selectedChannel = channelIds.includes(watchedChannel as TranslationChannelId)
    ? watchedChannel as TranslationChannelId
    : channelIds.includes(currentChannel as TranslationChannelId)
      ? currentChannel as TranslationChannelId
      : "google";
  const enabledChannel = channelIds.includes(activeChannel as TranslationChannelId)
    ? activeChannel as TranslationChannelId
    : "google";
  const selectedLabel = getChannelLabel(labels, selectedChannel);
  const enabledLabel = getChannelLabel(labels, enabledChannel);
  const hasDraftChanges = Boolean(channelDraftDirty[selectedChannel]);
  const selectedActivation = channelActivationStatus.channel === selectedChannel ? channelActivationStatus : undefined;
  const isFailed = selectedActivation?.status === "failed";
  const isTesting = selectedActivation?.status === "testing";
  const isEnabled = selectedChannel === enabledChannel && !hasDraftChanges && !isFailed;
  const statusKind = isFailed ? "failed" : isEnabled ? "enabled" : "pending";
  const buttonLoading =
    (selectedChannel === "google" && isActivatingGoogle) ||
    (selectedChannel === "baidu" && isTestingBaidu) ||
    (selectedChannel === "new-api" && isTestingNewApi) ||
    (selectedChannel === "deepl" && isTestingDeepl) ||
    isTesting;
  const statusText = (() => {
    if (isTesting) {
      return formatTemplate(labels.channelStatusTesting, selectedLabel, "正在测试并启用：{channel}");
    }
    if (isFailed) {
      return `${labels.channelStatusFailed || "保存启用失败"}：${selectedActivation?.message || ""}`;
    }
    if (isEnabled) {
      return formatTemplate(labels.channelStatusEnabled, selectedLabel, "当前启用：{channel}");
    }
    if (selectedChannel === enabledChannel) {
      return labels.channelStatusDirty || "配置已修改，保存并启用后生效。";
    }
    return (labels.channelStatusEditing || "正在编辑：{channel}，当前启用：{active}")
      .replace("{channel}", selectedLabel)
      .replace("{active}", enabledLabel);
  })();
  const saveLabel = formatTemplate(
    isFailed ? labels.retrySaveAndEnable : labels.saveAndEnable,
    selectedLabel,
    isFailed ? "重试保存并启用 {channel}" : "保存并启用 {channel}",
  );

  const renderSelectedConfig = () => {
    if (selectedChannel === "google") {
      return (
        <div className="translation-channel-note">
          <Text strong>{labels.channelGoogle}</Text>
          <Text type="secondary" style={{ display: "block", marginTop: 4, fontSize: 12, lineHeight: 1.55 }}>
            {labels.googleChannelDesc || "无需密钥，适合日常快速翻译；复杂多语言文本质量可能波动。"}
          </Text>
        </div>
      );
    }

    if (selectedChannel === "baidu") {
      return (
        <>
          <Text strong style={{ fontSize: 12 }}>{labels.baiduParams}</Text>
          <Row gutter={16} style={{ marginTop: 10 }}>
            <Col xs={24} sm={12}>
              <Form.Item label="App ID" name="baiduAppId" style={{ marginBottom: 12 }}>
                <Input placeholder="2026011900..." style={{ height: 32 }} />
              </Form.Item>
            </Col>
            <Col xs={24} sm={12}>
              <Form.Item label={labels.baiduSecret} name="baiduSecretKey" style={{ marginBottom: 12 }}>
                <Input.Password placeholder={labels.baiduSecretPlaceholder} style={{ height: 32 }} />
              </Form.Item>
            </Col>
          </Row>
        </>
      );
    }

    if (selectedChannel === "new-api") {
      return (
        <>
          <Text strong style={{ fontSize: 12 }}>{labels.newApiConfig}</Text>
          <Row gutter={16} style={{ marginTop: 10 }}>
            <Col xs={24} sm={12}>
              <Form.Item label={labels.relayServiceUrl} name="newApiBase" style={{ marginBottom: 12 }}>
                <Input placeholder="api.yousn.me" style={{ height: 32 }} />
              </Form.Item>
            </Col>
            <Col xs={24} sm={12}>
              <Form.Item label="API Key" name="newApiKey" style={{ marginBottom: 12 }}>
                <Input.Password placeholder="sk-..." style={{ height: 32 }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item label={labels.modelName} style={{ marginBottom: 12 }}>
            <Space wrap style={{ width: "100%" }}>
              <Form.Item name="newApiModel" noStyle>
                <AutoComplete
                  options={availableModels.map((model) => ({ value: model, label: model }))}
                  style={{ width: "min(100%, 320px)" }}
                  onSelect={(value) => form.setFieldValue("newApiModel", value)}
                  filterOption={(inputValue, option) => String(option?.value || "").toLowerCase().includes(inputValue.toLowerCase())}
                >
                  <Input placeholder="gemini-2.0-flash" style={{ height: 32 }} />
                </AutoComplete>
              </Form.Item>
              <Button icon={<SyncOutlined spin={isFetchingModels} />} onClick={fetchModels} style={{ height: 32 }}>
                {labels.fetchModels}
              </Button>
            </Space>
          </Form.Item>
          <Form.Item label={labels.translationDomain || "Translation domain"} name="newApiDomain" style={{ marginBottom: 12 }}>
            <Input placeholder={labels.translationDomainPlaceholder || DEFAULT_LLM_TRANSLATION_DOMAIN} style={{ height: 32 }} />
          </Form.Item>
          <Form.Item
            label={
              <Space style={{ width: "100%", justifyContent: "space-between" }}>
                <span>{labels.translationPrompt || "Translation prompt"}</span>
                <Button
                  size="small"
                  type="link"
                  onClick={() => form.setFieldsValue({
                    newApiPrompt: DEFAULT_LLM_TRANSLATION_PROMPT,
                    newApiDomain: DEFAULT_LLM_TRANSLATION_DOMAIN,
                  })}
                  style={{ padding: 0, height: 20 }}
                >
                  {labels.resetTranslationPrompt || "Reset default"}
                </Button>
              </Space>
            }
            name="newApiPrompt"
            style={{ marginBottom: 12 }}
          >
            <TextArea
              autoSize={{ minRows: 8, maxRows: 14 }}
              placeholder={DEFAULT_LLM_TRANSLATION_PROMPT}
              style={{ fontFamily: "Consolas, Monaco, monospace", fontSize: 12 }}
            />
          </Form.Item>
        </>
      );
    }

    return (
      <>
        <Text strong style={{ fontSize: 12 }}>{labels.deeplConfig}</Text>
        <Text type="secondary" style={{ display: "block", marginTop: 4, marginBottom: 10, fontSize: 12, lineHeight: 1.55 }}>
          {labels.deeplUnavailableDesc || "Use the Free endpoint for DeepL API Free keys and the Pro endpoint for paid API keys."}
        </Text>
        <Row gutter={16}>
          <Col xs={24} sm={12}>
            <Form.Item label={labels.deeplEndpoint} name="deeplEndpoint" style={{ marginBottom: 12 }}>
              <Select options={getDeepLEndpointOptions(labels)} style={{ height: 32 }} />
            </Form.Item>
          </Col>
          <Col xs={24} sm={12}>
            <Form.Item label="API Key" name="deeplApiKey" style={{ marginBottom: 12 }}>
              <Input.Password placeholder="DeepL-Auth-Key" style={{ height: 32 }} />
            </Form.Item>
          </Col>
        </Row>
        <Form.Item label={labels.deeplFormality} name="deeplFormality" style={{ marginBottom: 12 }}>
          <Select
            style={{ height: 32 }}
            options={[
              { value: "default", label: labels.deeplFormalityDefault || "Default" },
              { value: "prefer_more", label: labels.deeplFormalityMore || "More formal" },
              { value: "prefer_less", label: labels.deeplFormalityLess || "Less formal" },
            ]}
          />
        </Form.Item>
      </>
    );
  };

  return (
    <Card title={<span><GlobalOutlined style={{ marginRight: 8 }} />{labels.translationChannel}</span>} variant="borderless">
      <Row gutter={14}>
        <Col xs={24} md={12}>
          <Form.Item label={<Text strong style={{ fontSize: 12 }}>{labels.editChannel || "选择要配置的通道"}</Text>} name="channel" initialValue="google" style={{ marginBottom: 6 }}>
            <Select options={getChannelOptions(labels)} style={{ height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.5 }}>
            {labels.channelApplyHint || "选择只用于查看和编辑；保存并启用后才会切换实际翻译通道。"}
          </Text>
        </Col>
        <Col xs={24} md={12}>
          <Form.Item label={<Text strong style={{ fontSize: 12 }}>{labels.targetLanguage}</Text>} name="targetLang" initialValue="zh" style={{ marginBottom: 6 }}>
            <Select options={getTargetLangOptions(labels)} style={{ height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.5 }}>
            {labels.sourceAutoHint}
          </Text>
        </Col>
      </Row>

      <div className={`translation-channel-statusbar is-${statusKind}`}>
        <span className={`translation-channel-dot is-${statusKind}`} />
        <Text type={statusKind === "failed" ? "danger" : "secondary"} style={{ fontSize: 12, lineHeight: 1.45 }}>
          {statusText}
        </Text>
      </div>

      <div className="translation-channel-config-panel">
        {renderSelectedConfig()}
        <Button
          type="primary"
          block
          loading={buttonLoading}
          onClick={() => saveAndEnableChannel(selectedChannel)}
          style={{ height: 34, marginTop: 4 }}
        >
          {saveLabel}
        </Button>
      </div>
    </Card>
  );
}
