import { Card, Col, Form, InputNumber, Row, Switch, Typography, Slider } from "antd";
import { useI18n } from "../../i18n";

const { Text } = Typography;

export default function ScreenshotRecognitionCard() {
  const { text } = useI18n();
  const labels = text.settings;

  return (
    <Card title={labels.screenshotRecognition} bordered={false}>
      <Row gutter={[24, 16]} style={{ marginBottom: 16 }}>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.enableUiControlDetection} name="enableUiControlDetection" valuePropName="checked" style={{ marginBottom: 6 }}>
            <Switch />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.uiControlDetectionDesc}
          </Text>
        </Col>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.enableVisualDetection} name="enableVisualDetection" valuePropName="checked" style={{ marginBottom: 6 }}>
            <Switch />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.visualDetectionDesc}
          </Text>
        </Col>
      </Row>
      <Row gutter={[24, 16]}>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.detectionBorderWidth} name="detectionBorderWidth" style={{ marginBottom: 6 }}>
            <InputNumber min={1} max={6} placeholder="2" style={{ width: "100%", height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.detectionBorderDesc}
          </Text>
        </Col>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.toolbarButtonGap} name="toolbarButtonGap" style={{ marginBottom: 6 }}>
            <InputNumber min={0} max={16} placeholder="6" style={{ width: "100%", height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.toolbarButtonGapDesc}
          </Text>
        </Col>
      </Row>
      <Row gutter={[24, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.visualDetectionSensitivity} name="visualDetectionSensitivity" style={{ marginBottom: 6 }}>
            <InputNumber min={1} max={5} placeholder="3" style={{ width: "100%", height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.visualDetectionSensitivityDesc}
          </Text>
        </Col>
      </Row>
      <Row gutter={[24, 16]} style={{ marginTop: 16 }}>
        <Col xs={24} sm={12}>
          <Form.Item label="窗口边缘吸附" name="edgeSnapEnabled" valuePropName="checked" style={{ marginBottom: 6 }}>
            <Switch />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            启用后，拖动框选时会自动贴近附近窗口边缘。
          </Text>
        </Col>
        <Col xs={24} sm={12}>
          <Form.Item noStyle shouldUpdate={(prev, curr) => prev.edgeSnapEnabled !== curr.edgeSnapEnabled}>
            {({ getFieldValue }) => {
              const enabled = getFieldValue("edgeSnapEnabled");
              return (
                <div>
                  <Form.Item label="边缘吸附阈值" name="edgeSnapDistance" style={{ marginBottom: 6 }}>
                    <Slider min={0} max={20} step={1} disabled={!enabled} marks={{ 0: "0", 8: "8px", 20: "20" }} style={{ width: "100%", margin: "8px 0" }} />
                  </Form.Item>
                  <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
                    推荐 6–10px，数值越大越容易吸附；设为 0 等于关闭。
                  </Text>
                </div>
              );
            }}
          </Form.Item>
        </Col>
      </Row>
    </Card>
  );
}
