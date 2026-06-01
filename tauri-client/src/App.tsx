import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ConfigProvider, App as AntdApp } from "antd";
import {
  HistoryOutlined,
  SettingOutlined,
  InfoCircleOutlined,
  DashboardOutlined,
  FileTextOutlined,
} from "@ant-design/icons";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import History from "./pages/History";
import About from "./pages/About";
import OcrConfig from "./pages/OcrConfig";
import AppLayout from "./components/app/AppLayout";
import useServerStatus from "./hooks/useServerStatus";

function AppContent() {
  const [activeKey, setActiveKey] = useState<string>("dashboard");
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const { message, notification } = AntdApp.useApp();
  const { serverUrl, isOnline, isChecking, responseTime, checkStatus, fetchServerUrl } = useServerStatus();

  useEffect(() => {
    checkShortcutStatus();
  }, []);

  const checkShortcutStatus = async () => {
    try {
      await invoke("get_shortcut_status");
      setShortcutError(null);
    } catch (e: any) {
      const errMsg = e.toString();
      setShortcutError(errMsg);
      
      let hotkey = "Alt+A";
      try {
        const configStr = await invoke<string>("get_config");
        const parsedConfig = JSON.parse(configStr);
        if (parsedConfig.hotkey) {
          hotkey = parsedConfig.hotkey;
        }
      } catch (_) {}
      
      notification.error({
        message: `全局快捷键 (${hotkey}) 注册失败`,
        description: `无法成功在系统中注册截图快捷键 ${hotkey}。该热键可能已被其他运行中的软件占用。请尝试关闭相应软件或在设置中修改快捷键。`,
        duration: 0,
        placement: "topRight",
      });
    }
  };

  const handleStartScreenshot = async () => {
    try {
      message.loading({ content: "正在启动截图...", key: "screenshot" });
      await invoke("start_screenshot");
      message.success({ content: "已启动截图窗口", key: "screenshot" });
    } catch (err) {
      message.error({ content: `启动截图失败: ${err}`, key: "screenshot" });
    }
  };

  const menuItems = [
    { key: "dashboard", icon: <DashboardOutlined />, label: "首页" },
    { key: "settings", icon: <SettingOutlined />, label: "系统设置" },
    { key: "ocr-config", icon: <FileTextOutlined />, label: "模型/视频配置" },
    { key: "history", icon: <HistoryOutlined />, label: "历史记录" },
    { key: "about", icon: <InfoCircleOutlined />, label: "关于" },
  ];

  const renderContent = () => {
    switch (activeKey) {
      case "dashboard":
        return <Dashboard onStartScreenshot={handleStartScreenshot} shortcutError={shortcutError} serverStatus={isOnline} responseTime={responseTime} onRefreshStatus={() => checkStatus(serverUrl)} />;
      case "settings":
        return <Settings onConfigSaved={fetchServerUrl} />;
      case "ocr-config":
        return <OcrConfig />;
      case "history":
        return <History />;
      case "about":
        return <About />;
      default:
        return <Dashboard onStartScreenshot={handleStartScreenshot} shortcutError={shortcutError} serverStatus={isOnline} responseTime={responseTime} onRefreshStatus={() => checkStatus(serverUrl)} />;
    }
  };

  return (
    <AppLayout
      activeKey={activeKey}
      menuItems={menuItems}
      serverUrl={serverUrl}
      isOnline={isOnline}
      isChecking={isChecking}
      onMenuSelect={setActiveKey}
      onStartScreenshot={handleStartScreenshot}
      onRefreshStatus={() => checkStatus(serverUrl)}
    >
      {renderContent()}
    </AppLayout>
  );
}

export default function App() {
  return (
    <ConfigProvider
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
