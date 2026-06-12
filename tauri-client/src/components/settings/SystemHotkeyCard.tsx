import { AppstoreOutlined } from "@ant-design/icons";
import { Button, Card, Col, Form, Input, Row, Space, Switch, Typography } from "antd";
import type { KeyboardEvent } from "react";
import { useI18n } from "../../i18n";
import { hotkeyPattern } from "./settingsOptions";

const { Text } = Typography;

type SystemHotkeyCardProps = {
  onRestoreDefaultHotkeys: () => void;
  onHotkeyChange: (field: HotkeyField, value: string) => void;
  onClearScreenshotHotkey: () => void;
  onClearTranslateHotkey: () => void;
  onClearRecordingHotkey: () => void;
};

type HotkeyField = "hotkey" | "translateHotkey" | "recordingHotkey";

const modifierKeys = new Set(["Alt", "Control", "Ctrl", "Shift", "Meta", "OS", "Win", "Windows"]);

const normalizeMainKey = (event: KeyboardEvent<HTMLInputElement>) => {
  if (modifierKeys.has(event.key)) return null;
  if (event.key === " ") return "Space";
  if (event.key === "+") return "Plus";
  if (/^Key[A-Z]$/.test(event.code)) return event.code.slice(3);
  if (/^Digit[0-9]$/.test(event.code)) return event.code.slice(5);
  if (/^Numpad[0-9]$/.test(event.code)) return event.code.replace("Numpad", "Num");
  if (event.key.length === 1) return event.key.toUpperCase();
  return event.key;
};

const formatHotkey = (event: KeyboardEvent<HTMLInputElement>) => {
  const mainKey = normalizeMainKey(event);
  if (!mainKey) return null;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  if (event.metaKey) parts.push("Win");
  if (parts.length === 0) return null;
  parts.push(mainKey);
  return parts.join("+");
};

export default function SystemHotkeyCard({
  onRestoreDefaultHotkeys,
  onHotkeyChange,
  onClearScreenshotHotkey,
  onClearTranslateHotkey,
  onClearRecordingHotkey,
}: SystemHotkeyCardProps) {
  const { text } = useI18n();
  const labels = text.settings;
  const handleHotkeyKeyDown = (field: HotkeyField) => (event: KeyboardEvent<HTMLInputElement>) => {
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Backspace" || event.key === "Delete" || event.key === "Escape") {
      onHotkeyChange(field, "");
      return;
    }
    const nextHotkey = formatHotkey(event);
    if (nextHotkey) {
      onHotkeyChange(field, nextHotkey);
    }
  };

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
            <Input readOnly onKeyDown={handleHotkeyKeyDown("hotkey")} placeholder={labels.hotkeyPlaceholderAltA} style={{ height: 32, fontFamily: "monospace", textAlign: "center" }} />
          </Form.Item>
          <Form.Item
            label={labels.translateHotkey}
            name="translateHotkey"
            rules={[{ pattern: hotkeyPattern, message: labels.hotkeyFormatAltT }]}
            style={{ marginBottom: 12 }}
          >
            <Input readOnly onKeyDown={handleHotkeyKeyDown("translateHotkey")} placeholder={labels.hotkeyPlaceholderAltT} style={{ height: 32, fontFamily: "monospace", textAlign: "center" }} />
          </Form.Item>
          <Form.Item
            label={labels.recordingHotkey}
            name="recordingHotkey"
            rules={[{ pattern: hotkeyPattern, message: labels.hotkeyFormatAltR }]}
            style={{ marginBottom: 12 }}
          >
            <Input readOnly onKeyDown={handleHotkeyKeyDown("recordingHotkey")} placeholder={labels.hotkeyPlaceholderAltR} style={{ height: 32, fontFamily: "monospace", textAlign: "center" }} />
          </Form.Item>
          <Space wrap>
            <Button onClick={onClearScreenshotHotkey}>{labels.clearScreenshotHotkey}</Button>
            <Button onClick={onClearTranslateHotkey}>{labels.clearTranslateHotkey}</Button>
            <Button onClick={onClearRecordingHotkey}>{labels.clearRecordingHotkey}</Button>
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
