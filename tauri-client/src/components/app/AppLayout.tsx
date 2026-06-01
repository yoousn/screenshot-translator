import React from "react";
import { Button, Layout, Menu, Space, Tag, Tooltip, Typography } from "antd";
import { CameraOutlined, SyncOutlined, WifiOutlined } from "@ant-design/icons";

const { Header, Sider, Content } = Layout;
const { Text } = Typography;

interface AppLayoutProps {
  activeKey: string;
  menuItems: any[];
  serverUrl: string;
  isOnline: "checking" | "online" | "offline";
  isChecking: boolean;
  children: React.ReactNode;
  onMenuSelect: (key: string) => void;
  onStartScreenshot: () => void;
  onRefreshStatus: () => void;
}

export default function AppLayout({
  activeKey,
  menuItems,
  serverUrl,
  isOnline,
  isChecking,
  children,
  onMenuSelect,
  onStartScreenshot,
  onRefreshStatus,
}: AppLayoutProps) {
  return (
    <Layout style={{ height: "100vh", width: "100vw", overflow: "hidden" }}>
      <Sider
        width={200}
        theme="light"
        style={{
          borderRight: "1px solid #f0f0f0",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
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

          <Menu
            mode="inline"
            selectedKeys={[activeKey]}
            onClick={({ key }) => onMenuSelect(key)}
            items={menuItems}
            style={{ borderRight: 0, paddingTop: 12, flex: 1 }}
          />

          <div style={{ padding: 16, borderTop: "1px solid #f0f0f0" }}>
            <Button
              type="primary"
              icon={<CameraOutlined />}
              block
              onClick={onStartScreenshot}
              style={{ height: 36, display: "flex", alignItems: "center", justifyContent: "center" }}
            >
              立即截图
            </Button>
          </div>
        </div>
      </Sider>

      <Layout>
        <Header
          style={{
            height: 56,
            background: "#ffffff",
            padding: "0 24px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            borderBottom: "1px solid #f0f0f0",
            lineHeight: "56px",
          }}
        >
          <Space size="middle">
            <span style={{ fontSize: 12, color: "#8c8c8c" }}>本地截图翻译</span>
          </Space>

          <Space size="middle" style={{ marginLeft: "auto" }}>
            <Tooltip title={`文本翻译服务: ${serverUrl}`}>
              <Space size="small">
                {isOnline === "online" && <Tag color="success" icon={<WifiOutlined />} style={{ margin: 0 }}>服务在线 (Online)</Tag>}
                {isOnline === "offline" && <Tag color="error" icon={<WifiOutlined />} style={{ margin: 0 }}>服务离线 (Offline)</Tag>}
                {isOnline === "checking" && <Tag color="warning" icon={<SyncOutlined spin />} style={{ margin: 0 }}>检测中...</Tag>}
              </Space>
            </Tooltip>

            <Button
              type="text"
              icon={<SyncOutlined spin={isChecking} />}
              onClick={onRefreshStatus}
              disabled={isChecking}
              style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 32 }}
            />
          </Space>
        </Header>

        <Content style={{ padding: 24, background: "#f5f7fb", overflowY: "auto" }}>{children}</Content>
      </Layout>
    </Layout>
  );
}
