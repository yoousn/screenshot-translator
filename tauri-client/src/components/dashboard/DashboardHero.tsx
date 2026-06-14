import React from "react";
import { Button, Card, Flex, Typography } from "antd";
import { CameraOutlined } from "@ant-design/icons";

const { Title, Paragraph } = Typography;

interface DashboardHeroProps {
  title: string;
  description: string;
  buttonText: string;
  onStartScreenshot: () => void;
}

export default function DashboardHero({ title, description, buttonText, onStartScreenshot }: DashboardHeroProps) {
  return (
    <Card variant="borderless" style={{ borderRadius: 16 }}>
      <Flex justify="space-between" align="center" wrap="wrap" gap={16}>
        <div>
          <Title level={4} style={{ margin: 0 }}>{title}</Title>
          <Paragraph type="secondary" style={{ margin: "6px 0 0", maxWidth: 720 }}>{description}</Paragraph>
        </div>
        <Button type="primary" icon={<CameraOutlined />} onClick={onStartScreenshot}>{buttonText}</Button>
      </Flex>
    </Card>
  );
}
