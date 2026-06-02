import { Button, Card, Col, Form, Input, Row, Select, Space, Typography } from "antd";
import { GlobalOutlined, SyncOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";
import { getChannelOptions, getTargetLangOptions } from "./settingsOptions";
import type { SettingsControllerState } from "./types";

const { Text } = Typography;

type TranslationChannelCardProps = Pick<SettingsControllerState, "currentChannel" | "availableModels" | "isFetchingModels" | "isTestingBaidu" | "isTestingNewApi" | "fetchModels" | "testChannel">;

export default function TranslationChannelCard({
  currentChannel,
  availableModels,
  isFetchingModels,
  isTestingBaidu,
  isTestingNewApi,
  fetchModels,
  testChannel,
}: TranslationChannelCardProps) {
  const { text } = useI18n();
  const labels = text.settings;

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
