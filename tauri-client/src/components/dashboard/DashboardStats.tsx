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
      <Col span={6}><Card bordered={false}><Statistic title={labels.hotkey} value={hotkey} /></Card></Col>
      <Col span={6}><Card bordered={false}><Statistic title={labels.ocrMode} value={ocrModeLabel} /></Card></Col>
      <Col span={6}><Card bordered={false}><Statistic title={labels.targetLang} value={targetLang} /></Card></Col>
      <Col span={6}><Card bordered={false}><Statistic title={serverTitle} value={serverValue} suffix={<Tag color={serverStatusColor}>{serverStatusText}</Tag>} /></Card></Col>
    </Row>
  );
}
