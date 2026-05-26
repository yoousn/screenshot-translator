import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { 
  ConfigProvider, 
  Layout, 
  Menu, 
  Button, 
  Tag, 
  Space, 
  Typography, 
  Tooltip,
  App as AntdApp
} from "antd";
import {
  CameraOutlined,
  HistoryOutlined,
  SettingOutlined,
  InfoCircleOutlined,
  WifiOutlined,
  SyncOutlined,
  DashboardOutlined
} from "@ant-design/icons";
import Dashboard from "./pages/Dashboard";
import Settings from "./pages/Settings";
import History from "./pages/History";
import About from "./pages/About";

const { Header, Sider, Content } = Layout;
const { Text } = Typography;

function AppContent() {
  const [activeKey, setActiveKey] = useState<string>("dashboard");
  const [serverUrl, setServerUrl] = useState<string>("https://ocr.yousn.me");
  const [isOnline, setIsOnline] = useState<"checking" | "online" | "offline">("checking");
  const [isChecking, setIsChecking] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const { message, notification } = AntdApp.useApp();

  useEffect(() => {
    fetchServerUrl();
    checkShortcutStatus();
  }, []);

  const checkShortcutStatus = async () => {
    try {
      await invoke("get_shortcut_status");
      setShortcutError(null);
    } catch (e: any) {
      const errMsg = e.toString();
      setShortcutError(errMsg);
      notification.error({
        message: "全局快捷键 (Alt + A) 注册失败",
        description: `无法成功在系统中注册截图快捷键 Alt+A。该热键可能已被微信、QQ 或其他运行中的软件占用。请尝试关闭相应软件后重新运行本程序以激活快捷键。`,
        duration: 0,
        placement: "topRight"
      });
    }
  };

  const fetchServerUrl = async () => {
    try {
      const configStr = await invoke<string>("get_config");
      const parsedConfig = JSON.parse(configStr);
      if (parsedConfig.serverUrl) {
        setServerUrl(parsedConfig.serverUrl);
        checkStatus(parsedConfig.serverUrl);
      } else {
        checkStatus("https://ocr.yousn.me");
      }
    } catch (e) {
      console.error("Failed to load config for App layout:", e);
      checkStatus("https://ocr.yousn.me");
    }
  };

  const checkStatus = async (url: string) => {
    setIsChecking(true);
    setIsOnline("checking");
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 3000);
      const response = await fetch(`${url.replace(/\/$/, "")}/api/health`, {
        method: "GET",
        signal: controller.signal,
      });
      clearTimeout(timeoutId);
      if (response.ok) {
        setIsOnline("online");
      } else {
        setIsOnline("offline");
      }
    } catch (e) {
      setIsOnline("offline");
    } finally {
      setIsChecking(false);
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
    {
      key: "dashboard",
      icon: <DashboardOutlined />,
      label: "控制面板",
    },
    {
      key: "settings",
      icon: <SettingOutlined />,
      label: "系统设置",
    },
    {
      key: "history",
      icon: <HistoryOutlined />,
      label: "历史记录",
    },
    {
      key: "about",
      icon: <InfoCircleOutlined />,
      label: "关于系统",
    },
  ];

  const renderContent = () => {
    switch (activeKey) {
      case "dashboard":
        return <Dashboard onStartScreenshot={handleStartScreenshot} shortcutError={shortcutError} />;
      case "settings":
        return <Settings onConfigSaved={fetchServerUrl} />;
      case "history":
        return <History />;
      case "about":
        return <About />;
      default:
        return <Dashboard onStartScreenshot={handleStartScreenshot} shortcutError={shortcutError} />;
    }
  };

  return (
    <Layout style={{ height: "100vh", width: "100vw", overflow: "hidden" }}>
      {/* Sidebar Sider */}
      <Sider
        width={200}
        theme="light"
        style={{
          borderRight: "1px solid #f0f0f0",
          display: "flex",
          flexDirection: "column",
          justifyContent: "between",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
          {/* Logo Brand Header */}
          <div
            style={{
              height: 56,
              display: "flex",
              alignItems: "center",
              paddingLeft: 20,
              borderBottom: "1px solid #f0f0f0",
              userSelect: "none",
            }}
          >
            <div
              style={{
                height: 28,
                width: 28,
                borderRadius: 8,
                background: "linear-gradient(135deg, #1677ff 0%, #0050b3 100%)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "#ffffff",
                fontWeight: "bold",
                fontSize: 14,
                marginRight: 10,
                boxShadow: "0 2px 8px rgba(22, 119, 255, 0.2)",
              }}
            >
              Y
            </div>
            <Text strong style={{ fontSize: 13, color: "#1f1f1f", letterSpacing: 0.5 }}>
              YSN 截图翻译
            </Text>
          </div>

          {/* Navigation Menu */}
          <Menu
            mode="inline"
            selectedKeys={[activeKey]}
            onClick={({ key }) => setActiveKey(key)}
            items={menuItems}
            style={{ borderRight: 0, paddingTop: 12, flex: 1 }}
          />

          {/* Shortcut Action Trigger Footer */}
          <div style={{ padding: 16, borderTop: "1px solid #f0f0f0" }}>
            <Button
              type="primary"
              icon={<CameraOutlined />}
              block
              onClick={handleStartScreenshot}
              style={{ height: 36, display: "flex", alignItems: "center", justifyContent: "center" }}
            >
              立即截图
            </Button>
          </div>
        </div>
      </Sider>

      {/* Main Panel */}
      <Layout>
        {/* Top Header Status Bar */}
        <Header
          style={{
            height: 56,
            background: "#ffffff",
            padding: "0 24px",
            display: "flex",
            alignItems: "center",
            justifyContent: "between",
            borderBottom: "1px solid #f0f0f0",
            lineHeight: "56px",
          }}
        >
          <Space size="middle">
            <span style={{ fontSize: 12, color: "#8c8c8c" }}>
              算法核心：PIL 图像像素引擎 + PaddleOCR
            </span>
          </Space>

          <Space size="middle" style={{ marginLeft: "auto" }}>
            {/* Online/Offline tag status */}
            <Tooltip title={`服务器地址: ${serverUrl}`}>
              <Space size="small">
                {isOnline === "online" && (
                  <Tag color="success" icon={<WifiOutlined />} style={{ margin: 0 }}>
                    服务在线 (Online)
                  </Tag>
                )}
                {isOnline === "offline" && (
                  <Tag color="error" icon={<WifiOutlined />} style={{ margin: 0 }}>
                    服务离线 (Offline)
                  </Tag>
                )}
                {isOnline === "checking" && (
                  <Tag color="warning" icon={<SyncOutlined spin />} style={{ margin: 0 }}>
                    检测中...
                  </Tag>
                )}
              </Space>
            </Tooltip>

            {/* Check/Refresh connection */}
            <Button
              type="text"
              icon={<SyncOutlined spin={isChecking} />}
              onClick={() => checkStatus(serverUrl)}
              disabled={isChecking}
              style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 32 }}
            />
          </Space>
        </Header>

        {/* Content View */}
        <Content
          style={{
            padding: 24,
            background: "#f5f7fb",
            overflowY: "auto",
          }}
        >
          {renderContent()}
        </Content>
      </Layout>
    </Layout>
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
