import React from "react";
import { Button, Layout, Menu, Select, Space, Tag, Tooltip } from "antd";
import { CameraOutlined, GlobalOutlined, SyncOutlined, WifiOutlined } from "@ant-design/icons";
import { LANGUAGE_OPTIONS, type AppLanguage } from "../../i18n";
import type { TranslationServiceMetadata } from "../../hooks/useServerStatus";

const { Header, Sider, Content } = Layout;

type AppLayoutLabels = {
  screenshotNow: string;
  refresh: string;
  language: string;
  service: string;
  online: string;
  offline: string;
  checking: string;
  channel: string;
  glossary: string;
  qualityRisk: string;
};

interface AppLayoutProps {
  activeKey: string;
  menuItems: any[];
  serverUrl: string;
  isOnline: "checking" | "online" | "offline";
  isChecking: boolean;
  translationMetadata: TranslationServiceMetadata | null;
  language: AppLanguage;
  labels: AppLayoutLabels;
  children: React.ReactNode;
  onLanguageChange: (language: AppLanguage) => void;
  onMenuSelect: (key: string) => void;
  onStartScreenshot: () => void;
  onRefreshStatus: () => void;
}

const statusColor = {
  online: "success",
  offline: "error",
  checking: "warning",
} as const;

export default function AppLayout({
  activeKey,
  menuItems,
  serverUrl,
  isOnline,
  isChecking,
  translationMetadata,
  language,
  labels,
  children,
  onLanguageChange,
  onMenuSelect,
  onStartScreenshot,
  onRefreshStatus,
}: AppLayoutProps) {
  const statusText = isOnline === "online" ? labels.online : isOnline === "offline" ? labels.offline : labels.checking;
  const hasQualityRisk = Boolean(translationMetadata?.quality_flags?.google_free_low_quality_risk);
  const serviceTooltip = [
    `${labels.service}: ${serverUrl || "-"}`,
    translationMetadata?.active_channel ? `${labels.channel}: ${translationMetadata.active_channel}` : "",
    translationMetadata?.glossary_version ? `${labels.glossary}: ${translationMetadata.glossary_version}${translationMetadata.glossary_loaded === false ? " (not loaded)" : ""}` : "",
    hasQualityRisk ? labels.qualityRisk : "",
  ].filter(Boolean).join("\n");

  return (
    <Layout style={{ height: "100vh", width: "100vw", overflow: "hidden", background: "#f5f7fb" }}>
      <Sider
        width={224}
        theme="light"
        style={{
          borderRight: "1px solid #e5e7eb",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          background: "#ffffff",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
          <div style={{ minHeight: 72, display: "flex", alignItems: "center", justifyContent: "flex-start", padding: "14px 20px", borderBottom: "1px solid #eef2f7", userSelect: "none" }}>
            <div className="app-brand-wordmark">Ysn Trans</div>
          </div>

          <Menu mode="inline" selectedKeys={[activeKey]} onClick={({ key }) => onMenuSelect(key)} items={menuItems} style={{ borderRight: 0, paddingTop: 12, flex: 1 }} />

          <div style={{ padding: 16, borderTop: "1px solid #eef2f7" }}>
            <Button type="primary" icon={<CameraOutlined />} block onClick={onStartScreenshot} style={{ height: 40, display: "flex", alignItems: "center", justifyContent: "center", borderRadius: 12, fontWeight: 700 }}>
              {labels.screenshotNow}
            </Button>
          </div>
        </div>
      </Sider>

      <Layout>
        <Header style={{ height: 64, background: "rgba(255,255,255,0.92)", padding: "0 24px", display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: "1px solid #e5e7eb", lineHeight: "normal", backdropFilter: "blur(12px)", WebkitBackdropFilter: "blur(12px)" }}>
          <Space size="middle" style={{ marginLeft: "auto" }}>
            <Tooltip title={<span style={{ whiteSpace: "pre-line" }}>{serviceTooltip}</span>}>
              <Tag color={statusColor[isOnline]} icon={isOnline === "checking" ? <SyncOutlined spin /> : <WifiOutlined />} style={{ margin: 0, borderRadius: 999, padding: "2px 10px" }}>
                {statusText}
              </Tag>
            </Tooltip>

            <Tooltip title={labels.refresh}>
              <Button type="text" icon={<SyncOutlined spin={isChecking} />} onClick={onRefreshStatus} disabled={isChecking} style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 32, width: 32, borderRadius: 999 }} />
            </Tooltip>

            <Select
              size="small"
              value={language}
              options={LANGUAGE_OPTIONS}
              onChange={onLanguageChange}
              suffixIcon={<GlobalOutlined />}
              aria-label={labels.language}
              style={{ width: 126 }}
            />
          </Space>
        </Header>

        <Content style={{ padding: 24, background: "linear-gradient(180deg, #f8fafc 0%, #eef2ff 100%)", overflowY: "auto" }}>{children}</Content>
      </Layout>
    </Layout>
  );
}
