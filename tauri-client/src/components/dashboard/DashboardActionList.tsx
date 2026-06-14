import React from "react";
import { Button, Card, Space, Tag, Typography } from "antd";

const { Text } = Typography;

export interface DashboardActionItem {
  title: string;
  description: string;
  icon: React.ReactNode;
  hotkey: string;
  buttonText: string;
  danger?: boolean;
  onClick: () => void;
}

interface DashboardActionListProps {
  title: string;
  delayedTitle: string;
  delayedActive: boolean;
  delayedCancelText: string;
  defaultButtonText: string;
  items: DashboardActionItem[];
}

export default function DashboardActionList({
  title,
  delayedTitle,
  delayedActive,
  delayedCancelText,
  defaultButtonText,
  items,
}: DashboardActionListProps) {
  return (
    <Card title={title} variant="borderless" style={{ borderRadius: 16 }}>
      <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
        {items.map((item, index) => {
          const isDelayed = item.title === delayedTitle;
          return (
            <div
              key={item.title}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 16,
                padding: "12px 0",
                borderTop: index === 0 ? "none" : "1px solid #f0f0f0",
              }}
            >
              <Space align="start" style={{ minWidth: 0, flex: "1 1 auto" }}>
                {item.icon}
                <div style={{ minWidth: 0 }}>
                  <Text strong>{item.title}</Text>
                  <Text type="secondary" style={{ display: "block", marginTop: 2 }}>{item.description}</Text>
                </div>
              </Space>
              <Space>
                  <Tag color={item.danger ? "error" : "blue"}>{item.hotkey}</Tag>
                  <Button type="primary" danger={isDelayed && delayedActive} onClick={item.onClick}>
                    {isDelayed && delayedActive ? delayedCancelText : item.buttonText || defaultButtonText}
                  </Button>
              </Space>
            </div>
          );
        })}
      </div>
    </Card>
  );
}
