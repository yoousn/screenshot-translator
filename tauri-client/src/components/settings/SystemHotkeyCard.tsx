import { AppstoreOutlined } from "@ant-design/icons";
import { Button, Card, Col, Form, Input, Row, Space, Switch, Typography } from "antd";
import { useI18n } from "../../i18n";
import { hotkeyPattern } from "./settingsOptions";
import type { SettingsForm } from "./types";

const { Text } = Typography;

type SystemHotkeyCardProps = {
  form: SettingsForm;
  onRestoreDefaultHotkeys: () => void;
};

export default function SystemHotkeyCard({ form, onRestoreDefaultHotkeys }: SystemHotkeyCardProps) {
  const { text } = useI18n();
  const labels = text.settings;

  return (
    <Card title={<span><AppstoreOutlined style={{ marginRight: 8 }} />{labels.systemHotkeys}</span>} bordered={false}>
      <Row gutter={[24, 16]}>
        <Col xs={24} sm={12}>
          <Form.Item label={labels.autostart} name="autostart" valuePropName="checked" style={{ marginBottom: 6 }}>
            <Switch />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.autostartDesc}
          </Text>
        </Col>
        <Col xs={24} sm={12}>
          <Form.Item
            label={labels.screenshotHotkey}
            name="hotkey"
            rules={[{ pattern: hotkeyPattern, message: labels.hotkeyFormatAltA }]}
            style={{ marginBottom: 12 }}
          >
            <Input placeholder={labels.hotkeyPlaceholderAltA} style={{ height: 32, fontFamily: "monospace", textAlign: "center" }} />
          </Form.Item>
          <Form.Item
            label={labels.translateHotkey}
            name="translateHotkey"
            rules={[{ pattern: hotkeyPattern, message: labels.hotkeyFormatAltT }]}
            style={{ marginBottom: 12 }}
          >
            <Input placeholder={labels.hotkeyPlaceholderAltT} style={{ height: 32, fontFamily: "monospace", textAlign: "center" }} />
          </Form.Item>
          <Space wrap>
            <Button onClick={() => form.setFieldsValue({ hotkey: "" })}>{labels.clearScreenshotHotkey}</Button>
            <Button onClick={() => form.setFieldsValue({ translateHotkey: "" })}>{labels.clearTranslateHotkey}</Button>
            <Button onClick={onRestoreDefaultHotkeys}>{labels.restoreDefaults}</Button>
          </Space>
          <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: 8, lineHeight: 1.45 }}>
            {labels.hotkeyDesc}
          </Text>
        </Col>
      </Row>
    </Card>
  );
}
