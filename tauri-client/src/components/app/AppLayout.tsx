import React from "react";
import { Button, Layout, Menu, Select, Space, Tooltip } from "antd";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CameraOutlined, GlobalOutlined, SyncOutlined } from "@ant-design/icons";
import { LANGUAGE_OPTIONS, type AppLanguage } from "../../i18n";
import type { TranslationServiceMetadata } from "../../hooks/useServerStatus";
import type { StartupDependencySnapshot } from "../../hooks/useStartupDependencyStatus";
import DependencyStatusBar from "./DependencyStatusBar";
import MainWindowControls from "./MainWindowControls";

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
  dependencySnapshot: StartupDependencySnapshot | null;
  dependencyChecking: boolean;
  dependencyError?: string | null;
  language: AppLanguage;
  labels: AppLayoutLabels;
  children: React.ReactNode;
  onLanguageChange: (language: AppLanguage) => void;
  onMenuSelect: (key: string) => void;
  onStartScreenshot: () => void;
  onRefreshStatus: () => void;
  onRefreshDependencies: () => void;
  onOpenTranslationSettings: () => void;
  onOpenModelManagement: () => void;
  onOpenDependencies: () => void;
}

export default function AppLayout({
  activeKey,
  menuItems,
  serverUrl: _serverUrl,
  isOnline,
  isChecking,
  translationMetadata,
  dependencySnapshot,
  dependencyChecking,
  dependencyError,
  language,
  labels,
  children,
  onLanguageChange,
  onMenuSelect,
  onStartScreenshot,
  onRefreshStatus,
  onRefreshDependencies,
  onOpenTranslationSettings,
  onOpenModelManagement,
  onOpenDependencies,
}: AppLayoutProps) {
  const refreshAll = () => {
    onRefreshStatus();
    onRefreshDependencies();
  };

  const startWindowDrag = (event: React.MouseEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    const target = event.target as HTMLElement;
    if (target.closest("[data-no-drag='true']")) return;
    try {
      getCurrentWindow().startDragging().catch(() => {});
    } catch {
      // Browser preview does not provide Tauri window metadata.
    }
  };

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
          <div className="main-window-drag-region" onMouseDown={startWindowDrag} style={{ minHeight: 64, display: "flex", alignItems: "center", justifyContent: "flex-start", padding: "14px 20px", borderBottom: "1px solid #eef2f7", userSelect: "none" }}>
            <div className="app-brand-wordmark">YsnTrans</div>
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
        <Header className="main-window-drag-region" onMouseDown={startWindowDrag} style={{ height: 56, background: "rgba(255,255,255,0.92)", padding: "0 10px 0 24px", display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: "1px solid #e5e7eb", lineHeight: "normal", backdropFilter: "blur(12px)", WebkitBackdropFilter: "blur(12px)" }}>
          <Space size="middle" style={{ marginLeft: "auto" }} data-no-drag="true">
            <DependencyStatusBar
              translationStatus={isOnline}
              translationChecking={isChecking}
              translationMetadata={translationMetadata}
              dependencySnapshot={dependencySnapshot}
              dependencyChecking={dependencyChecking}
              dependencyError={dependencyError}
              onOpenTranslationSettings={onOpenTranslationSettings}
              onOpenModelManagement={onOpenModelManagement}
              onOpenDependencies={onOpenDependencies}
            />

            <Tooltip title={labels.refresh}>
              <Button type="text" icon={<SyncOutlined spin={isChecking || dependencyChecking} />} onClick={refreshAll} disabled={isChecking || dependencyChecking} style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 32, width: 32, borderRadius: 999 }} />
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
            <MainWindowControls />
          </Space>
        </Header>

        <Content
          style={{
            padding: 24,
            background: "linear-gradient(180deg, #f8fafc 0%, #eef2ff 100%)",
            overflow: "auto",
            minWidth: 0,
          }}
        >
          <div style={{ minWidth: 720, minHeight: "100%" }}>{children}</div>
        </Content>
      </Layout>
    </Layout>
  );
}
