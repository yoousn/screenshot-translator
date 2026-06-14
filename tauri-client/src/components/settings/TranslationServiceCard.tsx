import { useEffect, useState } from "react";
import { App as AntdApp, Button, Card, Col, Row, Space, Typography } from "antd";
import { DatabaseOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";
import { clearTranslationMemory, getTranslationMemoryStorageStats, type TranslationMemoryStorageStats } from "../../utils/translationMemory";

const { Text } = Typography;

export default function TranslationServiceCard() {
  const { text } = useI18n();
  const { message } = AntdApp.useApp();
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
    <Card title={<span><DatabaseOutlined style={{ marginRight: 8 }} />{labels.translationCache}</span>} variant="borderless">
      <Row gutter={[16, 12]}>
        <Col xs={24} md={16}>
          <Text strong style={{ fontSize: 12 }}>{labels.translationCache}</Text>
          <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: 4, lineHeight: 1.45 }}>
            {labels.translationCacheDesc}
          </Text>
          <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: 4, lineHeight: 1.45 }}>
            {labels.translationCacheStatus
              .replace("{entries}", String(cacheStats.entries))
              .replace("{maxEntries}", String(cacheStats.maxEntries))
              .replace("{ttlDays}", String(cacheStats.ttlDays))}
          </Text>
        </Col>
        <Col xs={24} md={8} style={{ display: "flex", alignItems: "flex-end", justifyContent: "flex-end" }}>
          <Space wrap style={{ justifyContent: "flex-end" }}>
            <Button size="small" onClick={refreshCacheStats}>{labels.refreshCacheStatus}</Button>
            <Button size="small" danger onClick={handleClearTranslationMemory}>{labels.clearTranslationCache}</Button>
          </Space>
        </Col>
      </Row>
    </Card>
  );
}
