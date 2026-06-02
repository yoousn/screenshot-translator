import { useEffect, useState } from "react";
import { Button, Card, Checkbox, Col, Form, Input, Row, Space, Typography, message } from "antd";
import { KeyOutlined, SlidersOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";
import { clearTranslationMemory, getTranslationMemoryStorageStats, type TranslationMemoryStorageStats } from "../../utils/translationMemory";

const { Text } = Typography;

export default function TranslationServiceCard() {
  const { text } = useI18n();
  const labels = text.settings;
  const [cacheStats, setCacheStats] = useState<TranslationMemoryStorageStats>(() => getTranslationMemoryStorageStats());

  const refreshCacheStats = () => setCacheStats(getTranslationMemoryStorageStats());

  useEffect(() => {
    refreshCacheStats();
  }, []);

  const handleClearTranslationMemory = () => {
    clearTranslationMemory();
    refreshCacheStats();
    message.success(labels.translationCacheCleared);
  };

  return (
    <Card title={<span><SlidersOutlined style={{ marginRight: 8 }} />{labels.translationService}</span>} bordered={false}>
      <Row gutter={16}>
        <Col span={12}>
          <Form.Item
            label={<Text strong style={{ fontSize: 12 }}>{labels.serviceUrl}</Text>}
            name="serverUrl"
            rules={[{ required: true, message: labels.serviceUrlRequired }]}
          >
            <Input placeholder={labels.serviceUrlPlaceholder} style={{ height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -10 }}>
            {labels.serviceUrlDesc}
          </Text>
        </Col>
        <Col span={12}>
          <Form.Item
            label={<Text strong style={{ fontSize: 12 }}>{labels.serviceToken}</Text>}
            name="clientToken"
            rules={[{ required: true, message: labels.serviceTokenRequired }]}
          >
            <Input.Password placeholder={labels.serviceTokenPlaceholder} prefix={<KeyOutlined />} style={{ height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -10 }}>
            {labels.serviceTokenDesc}
          </Text>
        </Col>
      </Row>
      <Row gutter={16} style={{ marginTop: 12 }}>
        <Col span={12}>
          <Form.Item
            label={<Text strong style={{ fontSize: 12 }}>{labels.lanServiceUrl || "Home LAN service URL"}</Text>}
            name="lanServerUrl"
          >
            <Input placeholder="http://192.168.1.10:8318" style={{ height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -10 }}>
            {labels.lanServiceUrlDesc || "Used first when you are on the same network as the N100 server."}
          </Text>
        </Col>
        <Col span={12}>
          <Form.Item name="preferLanServer" valuePropName="checked" style={{ marginTop: 27 }}>
            <Checkbox>{labels.preferLanServer || "Prefer home LAN service, fall back to public URL"}</Checkbox>
          </Form.Item>
        </Col>
      </Row>
      <Row gutter={16} style={{ marginTop: 12 }}>
        <Col span={16}>
          <Text strong style={{ fontSize: 12 }}>{labels.translationCache}</Text>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: 4 }}>
            {labels.translationCacheDesc}
          </Text>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: 4 }}>
            {labels.translationCacheStatus
              .replace("{entries}", String(cacheStats.entries))
              .replace("{maxEntries}", String(cacheStats.maxEntries))
              .replace("{ttlDays}", String(cacheStats.ttlDays))}
          </Text>
        </Col>
        <Col span={8} style={{ display: "flex", alignItems: "flex-end", justifyContent: "flex-end" }}>
          <Space>
            <Button size="small" onClick={refreshCacheStats}>{labels.refreshCacheStatus}</Button>
            <Button size="small" danger onClick={handleClearTranslationMemory}>{labels.clearTranslationCache}</Button>
          </Space>
        </Col>
      </Row>
    </Card>
  );
}
