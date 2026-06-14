import { Card, Col, Form, InputNumber, Row, Slider, Typography } from "antd";
import type { CSSProperties } from "react";
import { useI18n } from "../../i18n";

const { Text } = Typography;

const sectionStyle: CSSProperties = { marginTop: 16 };
const itemStyle: CSSProperties = { marginBottom: 6 };
const descStyle: CSSProperties = { fontSize: 11, display: "block", lineHeight: 1.45 };
const numberStyle: CSSProperties = { width: "100%", height: 32 };
const sliderStyle: CSSProperties = { width: "100%", margin: "8px 0" };
const sliderMarks = { 0: "0", 8: "8px", 20: "20" };

export default function ScreenshotRecognitionCard() {
  const { text } = useI18n();
  const labels = text.settings;

  return (
    <Card title={labels.screenshotRecognition} variant="borderless">
      <Row gutter={[24, 16]}>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.detectionBorderWidth} name="detectionBorderWidth" style={itemStyle}>
            <InputNumber min={1} max={6} placeholder="2" style={numberStyle} />
          </Form.Item>
          <Text type="secondary" style={descStyle}>
            {labels.detectionBorderDesc}
          </Text>
        </Col>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.toolbarButtonGap} name="toolbarButtonGap" style={itemStyle}>
            <InputNumber min={0} max={16} placeholder="6" style={numberStyle} />
          </Form.Item>
          <Text type="secondary" style={descStyle}>
            {labels.toolbarButtonGapDesc}
          </Text>
        </Col>
      </Row>
      <Row gutter={[24, 16]} style={sectionStyle}>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.visualDetectionSensitivity} name="visualDetectionSensitivity" style={itemStyle}>
            <InputNumber min={1} max={5} placeholder="3" style={numberStyle} />
          </Form.Item>
          <Text type="secondary" style={descStyle}>
            {labels.visualDetectionSensitivityDesc}
          </Text>
        </Col>
        <Col xs={24} sm={12}>
          <Form.Item label="边缘吸附阈值" name="edgeSnapDistance" style={itemStyle}>
            <Slider min={0} max={20} step={1} marks={sliderMarks} style={sliderStyle} />
          </Form.Item>
          <Text type="secondary" style={descStyle}>
            推荐 6–10px，数值越大越容易吸附；设为 0 等于关闭。开关在「功能开关」中。
          </Text>
        </Col>
      </Row>
    </Card>
  );
}
