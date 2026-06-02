import { Card, Col, Form, Input, Row, Typography } from "antd";
import { KeyOutlined, SlidersOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";

const { Text } = Typography;

export default function TranslationServiceCard() {
  const { text } = useI18n();
  const labels = text.settings;

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
    </Card>
  );
}
