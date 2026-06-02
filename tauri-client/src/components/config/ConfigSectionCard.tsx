import React from "react";
import { Card, Space, Typography } from "antd";

const { Paragraph, Text } = Typography;

interface ConfigSectionCardProps {
  title: React.ReactNode;
  eyebrow?: string;
  description?: React.ReactNode;
  extra?: React.ReactNode;
  children: React.ReactNode;
}

export default function ConfigSectionCard({ title, eyebrow, description, extra, children }: ConfigSectionCardProps) {
  return (
    <Card bordered={false} style={{ borderRadius: 18, height: "100%", boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }} extra={extra}>
      <Space direction="vertical" size={14} style={{ width: "100%" }}>
        <div>
          {eyebrow && <Text style={{ display: "block", marginBottom: 4, color: "#2563eb", fontSize: 11, fontWeight: 800, letterSpacing: 0.4, textTransform: "uppercase" }}>{eyebrow}</Text>}
          <Text strong style={{ display: "block", color: "#0f172a", fontSize: 16 }}>{title}</Text>
          {description && <Paragraph type="secondary" style={{ margin: "6px 0 0", fontSize: 12 }}>{description}</Paragraph>}
        </div>
        {children}
      </Space>
    </Card>
  );
}
