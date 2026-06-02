import { Button, Typography } from "antd";
import { SaveOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";

const { Title, Paragraph } = Typography;

type SettingsPageHeaderProps = {
  saving: boolean;
};

export default function SettingsPageHeader({ saving }: SettingsPageHeaderProps) {
  const { text } = useI18n();
  const labels = text.settings;

  return (
    <div style={{ position: "sticky", top: 0, zIndex: 20, display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid #e8e8e8", padding: "10px 0 16px", marginBottom: 24, background: "rgba(255,255,255,0.96)", backdropFilter: "blur(10px)" }}>
      <div>
        <Title level={4} style={{ margin: 0 }}>{labels.pageTitle}</Title>
        <Paragraph type="secondary" style={{ fontSize: 12, margin: "4px 0 0 0" }}>
          {labels.pageDesc}
        </Paragraph>
      </div>
      <Button type="primary" icon={<SaveOutlined />} htmlType="submit" loading={saving} style={{ height: 36 }}>
        {labels.saveSettings}
      </Button>
    </div>
  );
}
