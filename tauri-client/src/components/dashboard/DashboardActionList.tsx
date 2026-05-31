import React from "react";
import { Button, Card, List, Space, Tag, Typography } from "antd";

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
    <Card title={title} bordered={false} style={{ borderRadius: 16 }}>
      <List
        itemLayout="horizontal"
        dataSource={items}
        renderItem={(item) => {
          const isDelayed = item.title === delayedTitle;
          return (
            <List.Item
              actions={[
                <Space key="actions">
                  <Tag color={item.danger ? "error" : "blue"}>{item.hotkey}</Tag>
                  <Button type="primary" danger={isDelayed && delayedActive} onClick={item.onClick}>
                    {isDelayed && delayedActive ? delayedCancelText : item.buttonText || defaultButtonText}
                  </Button>
                </Space>,
              ]}
            >
              <List.Item.Meta
                avatar={item.icon}
                title={<Text strong>{item.title}</Text>}
                description={<Text type="secondary">{item.description}</Text>}
              />
            </List.Item>
          );
        }}
      />
    </Card>
  );
}
