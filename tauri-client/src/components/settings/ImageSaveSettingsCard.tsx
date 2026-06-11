import { invoke } from "@tauri-apps/api/core";
import { Button, Card, Col, Form, Input, Row, Space, Switch, Typography, message } from "antd";
import type { FormInstance } from "antd";
import { FolderOpenOutlined, SaveOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";

const { Text } = Typography;

interface ImageSaveSettingsCardProps {
  form: FormInstance;
}

export default function ImageSaveSettingsCard({ form }: ImageSaveSettingsCardProps) {
  const { text } = useI18n();
  const labels = text.settings;

  const chooseSaveDirectory = async () => {
    try {
      const currentDir = form.getFieldValue("imageSaveDefaultDir") || "";
      const selectedDir = await invoke<string | null>("choose_image_save_directory", {
        initialDir: currentDir,
      });
      if (selectedDir) {
        form.setFieldValue("imageSaveDefaultDir", selectedDir);
      }
    } catch (error: any) {
      message.error(labels.imageSaveChooseFolderFailed.replace("{error}", error?.message || error));
    }
  };

  return (
    <Card title={<span><SaveOutlined style={{ marginRight: 8 }} />{labels.imageSaveSettings}</span>} bordered={false}>
      <Row gutter={[16, 12]}>
        <Col xs={24} sm={10}>
          <Form.Item label={labels.imageSaveNamePrefix} name="imageSaveNamePrefix" style={{ marginBottom: 6 }}>
            <Input placeholder="Ysn_" style={{ height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.imageSaveNamePrefixDesc}
          </Text>
        </Col>
        <Col xs={24} sm={14}>
          <Form.Item label={labels.imageSaveNameFormat} name="imageSaveNameFormat" style={{ marginBottom: 6 }}>
            <Input placeholder="yyyyMMdd_HHmmss" style={{ height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.imageSaveNameFormatDesc}
          </Text>
        </Col>
      </Row>

      <Row gutter={[16, 12]} style={{ marginTop: 12 }}>
        <Col xs={24} md={16}>
          <Form.Item label={labels.imageSaveDefaultDir} name="imageSaveDefaultDir" style={{ marginBottom: 6 }}>
            <Input placeholder={labels.imageSaveDefaultDirPlaceholder} style={{ height: 32 }} />
          </Form.Item>
          <Text type="secondary" style={{ fontSize: 11, display: "block", lineHeight: 1.45 }}>
            {labels.imageSaveDefaultDirDesc}
          </Text>
        </Col>
        <Col xs={24} md={8} style={{ display: "flex", alignItems: "center", justifyContent: "flex-end" }}>
          <Button icon={<FolderOpenOutlined />} onClick={chooseSaveDirectory}>
            {labels.imageSaveChooseFolder}
          </Button>
        </Col>
      </Row>

      <Row gutter={[16, 12]} style={{ marginTop: 12 }}>
        <Col xs={24}>
          <Space align="start">
            <Form.Item name="imageSaveRememberLastDir" valuePropName="checked" style={{ marginBottom: 0 }}>
              <Switch />
            </Form.Item>
            <div>
              <Text strong style={{ fontSize: 12 }}>{labels.imageSaveRememberLastDir}</Text>
              <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: 4, lineHeight: 1.45 }}>
                {labels.imageSaveRememberLastDirDesc}
              </Text>
            </div>
          </Space>
        </Col>
      </Row>
    </Card>
  );
}
