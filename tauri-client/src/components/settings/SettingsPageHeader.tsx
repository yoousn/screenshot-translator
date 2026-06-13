import { Typography } from "antd";
import type { CSSProperties } from "react";
import { useI18n } from "../../i18n";

const { Title, Paragraph, Text } = Typography;

type SettingsPageHeaderProps = {
  saving: boolean;
};

const headerStyle: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "flex-start",
  gap: 16,
  borderBottom: "1px solid #e5e7eb",
  padding: "4px 0 18px",
  marginBottom: 24,
};
const leftStyle: CSSProperties = { minWidth: 0 };
const titleStyle: CSSProperties = { margin: 0 };
const descStyle: CSSProperties = { fontSize: 12, margin: "4px 0 0 0", lineHeight: 1.5 };
const statusStyle: CSSProperties = { flex: "0 0 auto", fontSize: 12, whiteSpace: "nowrap", paddingTop: 6 };

export default function SettingsPageHeader({ saving }: SettingsPageHeaderProps) {
  const { text } = useI18n();
  const labels = text.settings;

  return (
    <div style={headerStyle}>
      <div style={leftStyle}>
        <Title level={4} style={titleStyle}>{labels.pageTitle}</Title>
        <Paragraph type="secondary" style={descStyle}>
          {labels.pageDesc}
        </Paragraph>
      </div>
      <Text type={saving ? "warning" : "secondary"} style={statusStyle}>
        {saving ? "保存中…" : "修改后自动保存"}
      </Text>
    </div>
  );
}
