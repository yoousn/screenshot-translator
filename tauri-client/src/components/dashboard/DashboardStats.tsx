import React from "react";
import { Card, Col, Row, Statistic, Tag } from "antd";

interface DashboardStatsProps {
  hotkey: string;
  ocrModeLabel: string;
  targetLang: string;
  serverTitle: string;
  serverValue: string;
  serverStatusText: string;
  serverStatusColor: string;
  labels: {
    hotkey: string;
    ocrMode: string;
    targetLang: string;
  };
}

const cardStyle: React.CSSProperties = {
  height: "100%",
  borderRadius: 14,
};

const valueStyle: React.CSSProperties = {
  fontSize: 18,
  lineHeight: 1.42,
  fontWeight: 650,
  whiteSpace: "normal",
  wordBreak: "break-word",
};

export default function DashboardStats({
  hotkey,
  ocrModeLabel,
  targetLang,
  serverTitle,
  serverValue,
  serverStatusText,
  serverStatusColor,
  labels,
}: DashboardStatsProps) {
  return (
    <Row gutter={[16, 16]}>
      <Col span={6}><Card bordered={false} style={cardStyle}><Statistic title={labels.hotkey} value={hotkey} valueStyle={valueStyle} /></Card></Col>
      <Col span={6}><Card bordered={false} style={cardStyle}><Statistic title={labels.ocrMode} value={ocrModeLabel} valueStyle={valueStyle} /></Card></Col>
      <Col span={6}><Card bordered={false} style={cardStyle}><Statistic title={labels.targetLang} value={targetLang} valueStyle={valueStyle} /></Card></Col>
      <Col span={6}><Card bordered={false} style={cardStyle}><Statistic title={serverTitle} value={serverValue} valueStyle={valueStyle} suffix={<Tag color={serverStatusColor}>{serverStatusText}</Tag>} /></Card></Col>
    </Row>
  );
}
