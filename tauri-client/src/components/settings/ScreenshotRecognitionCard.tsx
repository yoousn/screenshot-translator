import { Card, Col, Form, InputNumber, Row, Switch, Typography } from "antd";
import { useI18n } from "../../i18n";

const { Text } = Typography;

export default function ScreenshotRecognitionCard() {
  const { text } = useI18n();
  const labels = text.settings;

  return (
    <Card title={labels.screenshotRecognition} bordered={false}>
      <Row gutter={24} style={{ marginBottom: 16 }}>
        <Col span={12}>
          <Form.Item label={labels.enableUiControlDetection} name="enableUiControlDetection" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
            {labels.uiControlDetectionDesc}
          </Text>
        </Col>
        <Col span={12}>
          <Form.Item label={labels.enableVisualDetection} name="enableVisualDetection" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
            {labels.visualDetectionDesc}
          </Text>
        </Col>
      </Row>
      <Row gutter={24}>
        <Col span={12}>
          <Form.Item label={labels.detectionBorderWidth} name="detectionBorderWidth">
            <InputNumber min={1} max={6} placeholder="2" style={{ width: "100%", height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
            {labels.detectionBorderDesc}
          </Text>
        </Col>
        <Col span={12}>
          <Form.Item label={labels.toolbarButtonGap} name="toolbarButtonGap">
            <InputNumber min={0} max={16} placeholder="6" style={{ width: "100%", height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
            {labels.toolbarButtonGapDesc}
          </Text>
        </Col>
      </Row>
      <Row gutter={24}>
        <Col span={12}>
          <Form.Item label={labels.visualDetectionSensitivity} name="visualDetectionSensitivity">
            <InputNumber min={1} max={5} placeholder="3" style={{ width: "100%", height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
            {labels.visualDetectionSensitivityDesc}
          </Text>
        </Col>
      </Row>
    </Card>
  );
}
