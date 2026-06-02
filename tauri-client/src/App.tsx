import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ConfigProvider, App as AntdApp } from "antd";
import {
  DashboardOutlined,
  FileTextOutlined,
  HistoryOutlined,
  InfoCircleOutlined,
  SettingOutlined,
} from "@ant-design/icons";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import History from "./pages/History";
import About from "./pages/About";
import OcrConfig from "./pages/OcrConfig";
import AppLayout from "./components/app/AppLayout";
import useServerStatus from "./hooks/useServerStatus";
import { I18nProvider, useI18n } from "./i18n";

function AppContent() {
  const [activeKey, setActiveKey] = useState<string>("dashboard");
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const { message, notification } = AntdApp.useApp();
  const { language, setLanguage, text } = useI18n();
  const { serverUrl, isOnline, isChecking, responseTime, translationMetadata, checkStatus, fetchServerUrl } = useServerStatus();

  useEffect(() => {
    checkShortcutStatus();
    invoke("prewarm_local_ocr_models").catch((error) => console.warn("Local OCR model prewarm failed", error));
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
      message.loading({ content: text.app.startingScreenshot, key: "screenshot" });
      await invoke("start_screenshot");
      message.success({ content: text.app.screenshotStarted, key: "screenshot" });
    } catch (error: any) {
      message.error({ content: `${text.app.screenshotFailed}${error?.message || error}`, key: "screenshot" });
    }
  };

  const menuItems = [
    { key: "dashboard", icon: <DashboardOutlined />, label: text.nav.dashboard },
    { key: "settings", icon: <SettingOutlined />, label: text.nav.settings },
    { key: "ocr-config", icon: <FileTextOutlined />, label: text.nav.ocrConfig },
    { key: "history", icon: <HistoryOutlined />, label: text.nav.history },
    { key: "about", icon: <InfoCircleOutlined />, label: text.nav.about },
  ];

  const renderContent = () => {
    switch (activeKey) {
      case "dashboard":
        return <Dashboard onStartScreenshot={handleStartScreenshot} shortcutError={shortcutError} serverStatus={isOnline} responseTime={responseTime} onNavigate={setActiveKey} />;
      case "settings":
        return <Settings onConfigSaved={fetchServerUrl} />;
      case "ocr-config":
        return <OcrConfig />;
      case "history":
        return <History />;
      case "about":
        return <About />;
      default:
        return <Dashboard onStartScreenshot={handleStartScreenshot} shortcutError={shortcutError} serverStatus={isOnline} responseTime={responseTime} onNavigate={setActiveKey} />;
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
      language={language}
      labels={{
        appName: text.app.name,
        tagline: text.app.tagline,
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
