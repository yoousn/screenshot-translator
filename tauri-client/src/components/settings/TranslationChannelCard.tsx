import { Alert, Button, Card, Col, Form, Input, Row, Select, Space, Tag, Tooltip, Typography } from "antd";
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

const { Text } = Typography;

type TranslationChannelCardProps = Pick<
  SettingsControllerState,
  | "currentChannel"
  | "availableModels"
  | "isFetchingModels"
  | "isTestingBaidu"
  | "isTestingNewApi"
  | "channelTestStatuses"
  | "serverChannelStatus"
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
  availableModels,
  isFetchingModels,
  isTestingBaidu,
  isTestingNewApi,
  channelTestStatuses,
  serverChannelStatus,
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

  const baiduAppId = baiduAppIdWatch ?? form.getFieldValue("baiduAppId");
  const baiduSecretKey = baiduSecretWatch ?? form.getFieldValue("baiduSecretKey");
  const newApiBase = newApiBaseWatch ?? form.getFieldValue("newApiBase");
  const newApiKey = newApiKeyWatch ?? form.getFieldValue("newApiKey");
  const newApiModel = newApiModelWatch ?? form.getFieldValue("newApiModel");

  const baiduMissing = [
    !hasValue(baiduAppId) ? "App ID" : "",
    !hasValue(baiduSecretKey) ? labels.baiduSecret : "",
  ].filter(Boolean);
  const newApiMissing = [
    !hasValue(newApiBase) ? labels.relayServiceUrl : "",
    !hasValue(newApiKey) ? "API Key" : "",
    !hasValue(newApiModel) ? labels.modelName : "",
  ].filter(Boolean);

  const serverSummary = serverChannelStatus.error
    ? labels.channelHealthServerFailed.replace("{error}", serverChannelStatus.error)
    : serverChannelStatus.activeChannel
      ? labels.channelHealthServerActive
          .replace("{channel}", serverChannelStatus.activeChannel)
          .replace("{url}", serverChannelStatus.serviceUrl || "-")
      : labels.channelHealthServerUnknown;

  const renderConfiguredTag = (configured: boolean) => (
    <Tag color={configured ? "success" : "warning"} icon={configured ? <CheckCircleOutlined /> : <WarningOutlined />}>
      {configured ? labels.channelHealthConfigured : labels.channelHealthMissing}
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
  ];

  return (
    <Card title={<span><GlobalOutlined style={{ marginRight: 8 }} />{labels.translationChannel}</span>} bordered={false}>
      <Form.Item label={<Text strong style={{ fontSize: 12 }}>{labels.activeChannel}</Text>} name="channel" initialValue="google">
        <Select options={getChannelOptions(labels)} style={{ height: 32 }} />
      </Form.Item>

      <Form.Item label={<Text strong style={{ fontSize: 12 }}>{labels.targetLanguage}</Text>} name="targetLang" initialValue="zh">
        <Select options={getTargetLangOptions(labels)} style={{ height: 32 }} />
      </Form.Item>
      <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -10 }}>
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
                gap: 8,
                paddingTop: index === 0 ? 2 : 8,
                borderTop: index === 0 ? "none" : "1px solid #eef1f4",
              }}
            >
              <div style={{ minWidth: 0 }}>
                <Space size={4} wrap>
                  <Text strong style={{ fontSize: 12 }}>{row.label}</Text>
                  {currentChannel === row.channel && <Tag color="blue">{labels.channelHealthCurrent}</Tag>}
                </Space>
                <Text type="secondary" style={{ display: "block", fontSize: 11, marginTop: 2 }}>
                  {row.detail}
                </Text>
              </div>
              <Space size={4} wrap style={{ justifyContent: "flex-end" }}>
                {renderConfiguredTag(row.configured)}
                {row.risk ? (
                  <Tag color="warning" icon={<WarningOutlined />}>
                    {labels.channelHealthGoogleRisk}
                  </Tag>
                ) : renderTestTag(row.testStatus)}
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
          style={{ marginTop: 12 }}
        />
      )}

      {currentChannel === "baidu" && (
        <Card type="inner" title={labels.baiduParams} style={{ marginTop: 12 }}>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item label="App ID" name="baiduAppId">
                <Input placeholder="2026011900..." style={{ height: 32 }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item label={labels.baiduSecret} name="baiduSecretKey">
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
            <Col span={12}>
              <Form.Item label={labels.relayServiceUrl} name="newApiBase">
                <Input placeholder="api.yousn.me" style={{ height: 32 }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item label="API Key" name="newApiKey">
                <Input.Password placeholder="sk-..." style={{ height: 32 }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item label={labels.modelName}>
            <Space style={{ width: "100%" }}>
              <Form.Item name="newApiModel" noStyle>
                {availableModels.length > 0 ? (
                  <Select options={availableModels.map((model) => ({ value: model, label: model }))} style={{ height: 32, width: 280 }} />
                ) : (
                  <Input placeholder="gemini-3.5-flash" style={{ height: 32, width: 280 }} />
                )}
              </Form.Item>
              <Button icon={<SyncOutlined spin={isFetchingModels} />} onClick={fetchModels} style={{ height: 32 }}>
                {labels.fetchModels}
              </Button>
            </Space>
          </Form.Item>
          <Button type="dashed" onClick={() => testChannel("new-api")} loading={isTestingNewApi} block style={{ height: 32 }}>
            {labels.testAndEnable}
          </Button>
        </Card>
      )}
    </Card>
  );
}
