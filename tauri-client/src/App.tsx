import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ConfigProvider, App as AntdApp } from "antd";
import {
  CloudDownloadOutlined,
  DashboardOutlined,
  FileTextOutlined,
  HistoryOutlined,
  InfoCircleOutlined,
  SettingOutlined,
  ThunderboltOutlined,
} from "@ant-design/icons";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import History from "./pages/History";
import About from "./pages/About";
import OcrConfig from "./pages/OcrConfig";
import ModelManagement from "./pages/ModelManagement";
import FeatureSwitches from "./pages/FeatureSwitches";
import AppLayout from "./components/app/AppLayout";
import useServerStatus from "./hooks/useServerStatus";
import useStartupDependencyStatus from "./hooks/useStartupDependencyStatus";
import { I18nProvider, useI18n } from "./i18n";

function AppContent() {
  const [activeKey, setActiveKey] = useState<string>("dashboard");
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const { message, notification } = AntdApp.useApp();
  const { language, setLanguage, text } = useI18n();
  const { serverUrl, isOnline, isChecking, responseTime, translationMetadata, checkStatus, fetchServerUrl } = useServerStatus();
  const dependencyStatus = useStartupDependencyStatus();

  useEffect(() => {
    checkShortcutStatus();
  }, []);

  const checkShortcutStatus = async () => {
    try {
      await invoke("get_shortcut_status");
      setShortcutError(null);
    } catch (error: any) {
      const errorMessage = error?.message || error?.toString?.() || String(error);
      setShortcutError(errorMessage);

      let hotkey = "Alt+A";
      try {
        const configStr = await invoke<string>("get_config");
        const parsedConfig = JSON.parse(configStr || "{}");
        if (parsedConfig.hotkey) hotkey = parsedConfig.hotkey;
      } catch (_) {}

      notification.error({
        message: text.app.shortcutErrorTitle,
        description: text.app.shortcutErrorDesc.replace("{hotkey}", hotkey),
        duration: 0,
        placement: "topRight",
      });
    }
  };

  const handleStartScreenshot = async () => {
    try {
      await invoke("start_screenshot");
    } catch (error: any) {
      message.error({ content: `${text.app.screenshotFailed}${error?.message || error}`, key: "screenshot" });
    }
  };

  const menuItems = [
    { key: "dashboard", icon: <DashboardOutlined />, label: text.nav.dashboard },
    { key: "settings", icon: <SettingOutlined />, label: text.nav.settings },
    { key: "model-management", icon: <CloudDownloadOutlined />, label: text.nav.modelManagement },
    { key: "ocr-config", icon: <FileTextOutlined />, label: text.nav.ocrConfig },
    { key: "feature-switches", icon: <ThunderboltOutlined />, label: "功能开关" },
    { key: "history", icon: <HistoryOutlined />, label: text.nav.history },
    { key: "about", icon: <InfoCircleOutlined />, label: text.nav.about },
  ];

  const renderContent = () => {
    switch (activeKey) {
      case "dashboard":
        return <Dashboard onStartScreenshot={handleStartScreenshot} shortcutError={shortcutError} serverStatus={isOnline} responseTime={responseTime} />;
      case "settings":
        return <Settings onConfigSaved={fetchServerUrl} />;
      case "model-management":
        return <ModelManagement />;
      case "ocr-config":
        return <OcrConfig />;
      case "feature-switches":
        return <FeatureSwitches />;
      case "history":
        return <History />;
      case "about":
        return <About />;
      default:
        return <Dashboard onStartScreenshot={handleStartScreenshot} shortcutError={shortcutError} serverStatus={isOnline} responseTime={responseTime} />;
    }
  };

  return (
    <AppLayout
      activeKey={activeKey}
      menuItems={menuItems}
      serverUrl={serverUrl}
      isOnline={isOnline}
      isChecking={isChecking}
      translationMetadata={translationMetadata}
      dependencySnapshot={dependencyStatus.snapshot}
      dependencyChecking={dependencyStatus.checking}
      dependencyError={dependencyStatus.error}
      language={language}
      labels={{
        screenshotNow: text.app.screenshotNow,
        refresh: text.app.refresh,
        language: text.app.language,
        service: text.status.service,
        online: text.status.online,
        offline: text.status.offline,
        checking: text.status.checking,
        channel: text.status.channel,
        glossary: text.status.glossary,
        qualityRisk: text.status.qualityRisk,
      }}
      onLanguageChange={setLanguage}
      onMenuSelect={setActiveKey}
      onStartScreenshot={handleStartScreenshot}
      onRefreshStatus={() => checkStatus(serverUrl)}
      onRefreshDependencies={dependencyStatus.refresh}
      onOpenTranslationSettings={() => setActiveKey("settings")}
      onOpenModelManagement={() => setActiveKey("model-management")}
      onOpenDependencies={() => setActiveKey("ocr-config")}
    >
      {renderContent()}
    </AppLayout>
  );
}

function LocalizedApp() {
  const { antdLocale } = useI18n();

  return (
    <ConfigProvider
      locale={antdLocale}
      theme={{
        token: {
          colorPrimary: "#1677ff",
          borderRadius: 12,
          fontFamily: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif",
        },
        components: {
          Card: {
            paddingLG: 20,
          },
        },
      }}
    >
      <AntdApp>
        <AppContent />
      </AntdApp>
    </ConfigProvider>
  );
}

export default function App() {
  return (
    <I18nProvider>
      <LocalizedApp />
    </I18nProvider>
  );
}
