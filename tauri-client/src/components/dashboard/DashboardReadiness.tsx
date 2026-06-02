import React from "react";
import { Alert, Button, Card, Space, Tag, Typography } from "antd";
import { ApiOutlined, CheckCircleOutlined, ExclamationCircleOutlined, ThunderboltOutlined } from "@ant-design/icons";

const { Text } = Typography;

type DashboardReadinessLabels = {
  commercialReady: string;
  commercialReadyDesc: string;
  installModels: string;
  configureService: string;
  fixHotkey: string;
  allCoreReady: string;
  actionNeeded: string;
};

interface DashboardReadinessProps {
  labels: DashboardReadinessLabels;
  serverStatus: "checking" | "online" | "offline";
  shortcutError?: string | null;
  onOpenModels: () => void;
  onOpenSettings: () => void;
}

export default function DashboardReadiness({ labels, serverStatus, shortcutError, onOpenModels, onOpenSettings }: DashboardReadinessProps) {
  const hasServerIssue = serverStatus === "offline";
  const hasShortcutIssue = Boolean(shortcutError);
  const allReady = !hasServerIssue && !hasShortcutIssue;

  return (
    <Card bordered={false} style={{ borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.08)" }}>
      <Space direction="vertical" size={14} style={{ width: "100%" }}>
        <Space align="start" style={{ justifyContent: "space-between", width: "100%" }}>
          <Space align="start">
            <div style={{ width: 40, height: 40, borderRadius: 14, background: "linear-gradient(135deg, #dbeafe 0%, #cffafe 100%)", display: "flex", alignItems: "center", justifyContent: "center", color: "#2563eb", fontSize: 18 }}>
              <ThunderboltOutlined />
            </div>
            <div>
              <Text strong style={{ display: "block", color: "#0f172a", fontSize: 15 }}>{labels.commercialReady}</Text>
              <Text type="secondary" style={{ fontSize: 12 }}>{labels.commercialReadyDesc}</Text>
            </div>
          </Space>
          <Tag color={allReady ? "success" : "warning"} icon={allReady ? <CheckCircleOutlined /> : <ExclamationCircleOutlined />} style={{ borderRadius: 999, margin: 0 }}>
            {allReady ? labels.allCoreReady : labels.actionNeeded}
          </Tag>
        </Space>

        {(hasServerIssue || hasShortcutIssue) && (
          <Alert
            type="warning"
            showIcon
            message={hasShortcutIssue ? labels.fixHotkey : labels.configureService}
            description={shortcutError || undefined}
            action={<Button size="small" onClick={onOpenSettings}>{labels.configureService}</Button>}
          />
        )}

        <Space wrap>
          <Button icon={<ApiOutlined />} onClick={onOpenModels}>{labels.installModels}</Button>
          <Button onClick={onOpenSettings}>{labels.configureService}</Button>
          {hasShortcutIssue && <Button danger onClick={onOpenSettings}>{labels.fixHotkey}</Button>}
        </Space>
      </Space>
    </Card>
  );
}
