import { Alert, AutoComplete, Button, Card, Col, Form, Input, Row, Select, Space, Tag, Tooltip, Typography } from "antd";
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  ClockCircleOutlined,
  GlobalOutlined,
  SyncOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { useI18n } from "../../i18n";
import { getChannelOptions, getTargetLangOptions } from "./settingsOptions";
import type {
  SettingsControllerState,
  SettingsForm,
  TranslationChannelId,
  TranslationChannelTestStatus,
} from "./types";
import { DEFAULT_LLM_TRANSLATION_DOMAIN, DEFAULT_LLM_TRANSLATION_PROMPT } from "../../utils/defaultTranslationPrompt";

const { Text } = Typography;
const { TextArea } = Input;

type TranslationChannelCardProps = Pick<
  SettingsControllerState,
  | "currentChannel"
  | "isActivatingGoogle"
  | "availableModels"
  | "isFetchingModels"
  | "isTestingBaidu"
  | "isTestingNewApi"
  | "isTestingDeepl"
  | "channelTestStatuses"
  | "serverChannelStatus"
  | "activateGoogleChannel"
  | "fetchModels"
  | "testChannel"
> & {
  form: SettingsForm;
};

const hasValue = (value: unknown) => String(value || "").trim().length > 0;

const formatMissingParts = (template: string, parts: string[]) => template.replace("{parts}", parts.join(", "));

export default function TranslationChannelCard({
  form,
  currentChannel,
  isActivatingGoogle,
  availableModels,
  isFetchingModels,
  isTestingBaidu,
  isTestingNewApi,
  isTestingDeepl,
  channelTestStatuses,
  serverChannelStatus,
  activateGoogleChannel,
  fetchModels,
  testChannel,
}: TranslationChannelCardProps) {
  const { text } = useI18n();
  const labels = text.settings;

  const baiduAppIdWatch = Form.useWatch("baiduAppId", form);
  const baiduSecretWatch = Form.useWatch("baiduSecretKey", form);
  const newApiBaseWatch = Form.useWatch("newApiBase", form);
  const newApiKeyWatch = Form.useWatch("newApiKey", form);
  const newApiModelWatch = Form.useWatch("newApiModel", form);
  const deeplEndpointWatch = Form.useWatch("deeplEndpoint", form);
  const deeplApiKeyWatch = Form.useWatch("deeplApiKey", form);

  const baiduAppId = baiduAppIdWatch ?? form.getFieldValue("baiduAppId");
  const baiduSecretKey = baiduSecretWatch ?? form.getFieldValue("baiduSecretKey");
  const newApiBase = newApiBaseWatch ?? form.getFieldValue("newApiBase");
  const newApiKey = newApiKeyWatch ?? form.getFieldValue("newApiKey");
  const newApiModel = newApiModelWatch ?? form.getFieldValue("newApiModel");
  const deeplEndpoint = deeplEndpointWatch ?? form.getFieldValue("deeplEndpoint");
  const deeplApiKey = deeplApiKeyWatch ?? form.getFieldValue("deeplApiKey");

  const baiduMissing = [
    !hasValue(baiduAppId) ? "App ID" : "",
    !hasValue(baiduSecretKey) ? labels.baiduSecret : "",
  ].filter(Boolean);
  const newApiMissing = [
    !hasValue(newApiBase) ? labels.relayServiceUrl : "",
    !hasValue(newApiKey) ? "API Key" : "",
    !hasValue(newApiModel) ? labels.modelName : "",
  ].filter(Boolean);
  const deeplMissing = [
    !hasValue(deeplEndpoint) ? labels.deeplEndpoint : "",
    !hasValue(deeplApiKey) ? "API Key" : "",
  ].filter(Boolean);

  const serverSummary = serverChannelStatus.error
    ? labels.channelHealthServerFailed.replace("{error}", serverChannelStatus.error)
    : serverChannelStatus.activeChannel
      ? labels.channelHealthServerActive
          .replace("{channel}", serverChannelStatus.activeChannel)
          .replace("{url}", serverChannelStatus.serviceUrl || "-")
      : labels.channelHealthServerUnknown;

  const renderConfiguredTag = (configured: boolean) => (
    <Tag color={configured ? "success" : "warning"} icon={configured ? <CheckCircleOutlined /> : <WarningOutlined />} style={{ marginInlineEnd: 0 }}>
      {configured ? labels.channelHealthConfigured : labels.channelHealthMissing}
    </Tag>
  );

  const renderUnavailableTag = () => (
    <Tag color="default" icon={<WarningOutlined />} style={{ marginInlineEnd: 0 }}>
      {labels.temporarilyUnavailable || "Unavailable"}
    </Tag>
  );

  const renderTestTag = (status?: TranslationChannelTestStatus) => {
    if (!status) {
      return <Tag icon={<ClockCircleOutlined />}>{labels.channelHealthUntested}</Tag>;
    }
    if (status.status === "testing") {
      return (
        <Tag color="processing" icon={<SyncOutlined spin />}>
          {labels.channelHealthTesting}
        </Tag>
      );
    }
    if (status.status === "passed") {
      return (
        <Tooltip title={status.serviceUrl || status.message}>
          <Tag color="success" icon={<CheckCircleOutlined />}>
            {labels.channelHealthPassed}
          </Tag>
        </Tooltip>
      );
    }
    return (
      <Tooltip title={status.message}>
        <Tag color="error" icon={<CloseCircleOutlined />}>
          {labels.channelHealthFailed}
        </Tag>
      </Tooltip>
    );
  };

  const rows: Array<{
    channel: TranslationChannelId;
    label: string;
    detail: string;
    configured: boolean;
    testStatus?: TranslationChannelTestStatus;
    risk?: boolean;
    unavailable?: boolean;
  }> = [
    {
      channel: "google",
      label: labels.channelGoogle,
      detail: labels.channelHealthNoSecret,
      configured: true,
      risk: true,
    },
    {
      channel: "baidu",
      label: labels.channelBaidu,
      detail: baiduMissing.length > 0 ? formatMissingParts(labels.channelHealthMissingParts, baiduMissing) : labels.channelHealthBaiduDesc,
      configured: baiduMissing.length === 0,
      testStatus: channelTestStatuses.baidu,
    },
    {
      channel: "new-api",
      label: labels.channelNewApi,
      detail: newApiMissing.length > 0 ? formatMissingParts(labels.channelHealthMissingParts, newApiMissing) : labels.channelHealthNewApiDesc,
      configured: newApiMissing.length === 0,
      testStatus: channelTestStatuses["new-api"],
    },
    {
      channel: "deepl",
      label: labels.channelDeepL,
      detail: labels.deeplUnavailableDesc || labels.channelHealthDeepLDesc,
      configured: deeplMissing.length === 0,
      testStatus: channelTestStatuses.deepl,
      unavailable: true,
    },
  ];

  return (
    <Card title={<span><GlobalOutlined style={{ marginRight: 8 }} />{labels.translationChannel}</span>} bordered={false}>
      <Form.Item label={<Text strong style={{ fontSize: 12 }}>{labels.activeChannel}</Text>} name="channel" initialValue="google" style={{ marginBottom: 6 }}>
        <Select options={getChannelOptions(labels)} style={{ height: 32 }} />
      </Form.Item>
      <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.5, marginBottom: 12 }}>
        {labels.channelApplyHint || "Choose a channel, then save settings to apply it."}
      </Text>

      <Form.Item label={<Text strong style={{ fontSize: 12 }}>{labels.targetLanguage}</Text>} name="targetLang" initialValue="zh" style={{ marginBottom: 6 }}>
        <Select options={getTargetLangOptions(labels)} style={{ height: 32 }} />
      </Form.Item>
      <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.5 }}>
        {labels.sourceAutoHint}
      </Text>

      <div style={{ marginTop: 12, padding: 10, border: "1px solid #eef1f4", borderRadius: 8, background: "#fbfcfe" }}>
        <Space direction="vertical" size={6} style={{ width: "100%" }}>
          <div>
            <Text strong style={{ fontSize: 12 }}>{labels.channelHealthTitle}</Text>
            <Text type="secondary" style={{ display: "block", fontSize: 11, marginTop: 2 }}>
              {labels.channelHealthDesc}
            </Text>
            <Text type={serverChannelStatus.error ? "danger" : "secondary"} style={{ display: "block", fontSize: 11, marginTop: 4 }}>
              {serverSummary}
            </Text>
          </div>
          {rows.map((row, index) => (
            <div
              key={row.channel}
              style={{
                display: "flex",
                alignItems: "flex-start",
                justifyContent: "space-between",
                flexWrap: "wrap",
                gap: 8,
                paddingTop: index === 0 ? 2 : 8,
                borderTop: index === 0 ? "none" : "1px solid #eef1f4",
              }}
            >
              <div style={{ minWidth: 220, flex: "1 1 260px" }}>
                <Space size={4} wrap>
                  <Text strong style={{ fontSize: 12, lineHeight: 1.4 }}>{row.label}</Text>
                  {currentChannel === row.channel && <Tag color="blue" style={{ marginInlineEnd: 0 }}>{labels.channelHealthCurrent}</Tag>}
                </Space>
                <Text type="secondary" style={{ display: "block", fontSize: 11, marginTop: 2, lineHeight: 1.45, wordBreak: "break-word" }}>
                  {row.detail}
                </Text>
              </div>
              <Space size={4} wrap style={{ justifyContent: "flex-end", flex: "0 1 280px" }}>
                {renderConfiguredTag(row.configured)}
                {row.risk ? (
                  <Tag color="warning" icon={<WarningOutlined />} style={{ marginInlineEnd: 0 }}>
                    {labels.channelHealthGoogleRisk}
                  </Tag>
                ) : row.unavailable ? renderUnavailableTag() : renderTestTag(row.testStatus)}
              </Space>
            </div>
          ))}
        </Space>
      </div>

      {currentChannel === "google" && (
        <Alert
          type="warning"
          showIcon
          message={labels.googleQualityWarningTitle}
          description={labels.googleQualityWarningDesc}
          action={
            <Button size="small" type="primary" ghost loading={isActivatingGoogle} onClick={activateGoogleChannel}>
              {labels.setAsActiveChannel || "Set as active"}
            </Button>
          }
          style={{ marginTop: 12 }}
        />
      )}

      {currentChannel === "baidu" && (
        <Card type="inner" title={labels.baiduParams} style={{ marginTop: 12 }}>
          <Row gutter={16}>
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
          <Button type="dashed" onClick={() => testChannel("baidu")} loading={isTestingBaidu} block style={{ height: 32 }}>
            {labels.testAndEnable}
          </Button>
        </Card>
      )}

      {currentChannel === "new-api" && (
        <Card type="inner" title={labels.newApiConfig} style={{ marginTop: 12 }}>
          <Row gutter={16}>
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
                  <Input placeholder="gemini-3.5-flash" style={{ height: 32 }} />
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
          <Button type="dashed" onClick={() => testChannel("new-api")} loading={isTestingNewApi} block style={{ height: 32 }}>
            {labels.testAndEnable}
          </Button>
        </Card>
      )}

      {currentChannel === "deepl" && (
        <Card type="inner" title={labels.deeplConfig} style={{ marginTop: 12 }}>
          <Alert
            type="warning"
            showIcon
            message={labels.deeplUnavailableTitle || "DeepL channel is unavailable"}
            description={labels.deeplUnavailableDesc || "Use Google, Baidu, or the LLM translation channel in this build."}
            style={{ marginBottom: 12 }}
          />
          <Row gutter={16}>
            <Col xs={24} sm={12}>
              <Form.Item label={labels.deeplEndpoint} name="deeplEndpoint" style={{ marginBottom: 12 }}>
                <Input disabled placeholder="https://api-free.deepl.com" style={{ height: 32 }} />
              </Form.Item>
            </Col>
            <Col xs={24} sm={12}>
              <Form.Item label="API Key" name="deeplApiKey" style={{ marginBottom: 12 }}>
                <Input.Password disabled placeholder="DeepL-Auth-Key" style={{ height: 32 }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item label={labels.deeplFormality} name="deeplFormality" style={{ marginBottom: 12 }}>
            <Select
              disabled
              style={{ height: 32 }}
              options={[
                { value: "default", label: labels.deeplFormalityDefault || "Default" },
                { value: "prefer_more", label: labels.deeplFormalityMore || "More formal" },
                { value: "prefer_less", label: labels.deeplFormalityLess || "Less formal" },
              ]}
            />
          </Form.Item>
          <Button disabled block style={{ height: 32 }}>
            {labels.temporarilyUnavailable || "Unavailable"}
          </Button>
        </Card>
      )}
    </Card>
  );
}
