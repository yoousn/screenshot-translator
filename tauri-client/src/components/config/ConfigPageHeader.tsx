import React from "react";
import { Button, Card, Space, Tag, Typography, message } from "antd";
import { CopyOutlined, GlobalOutlined, SafetyCertificateOutlined, VideoCameraOutlined } from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../../i18n";

const { Title, Paragraph, Text } = Typography;

export default function ConfigPageHeader() {
  const { text } = useI18n();
  const labels = text.config;

  const copyDiagnostics = async () => {
    try {
      const report = await invoke("get_diagnostics_report");
      await navigator.clipboard.writeText(JSON.stringify(report, null, 2));
      message.success(labels.diagnosticsCopied);
    } catch (error: any) {
      message.error(labels.diagnosticsCopyFailed + (error?.message || error));
    }
  };

  return (
    <Card bordered={false} style={{ borderRadius: 20, background: "linear-gradient(135deg, #eef6ff 0%, #f8fbff 52%, #fff7ed 100%)" }}>
      <Space direction="vertical" size={10} style={{ width: "100%" }}>
        <Space wrap size={8}>
          <Tag color="blue" icon={<SafetyCertificateOutlined />}>{labels.commercialOcrMainline}</Tag>
          <Tag color="cyan" icon={<GlobalOutlined />}>{labels.automaticSourceLanguage}</Tag>
          <Tag color="orange" icon={<VideoCameraOutlined />}>{labels.recordingSaveFolder}</Tag>
        </Space>
        <div>
          <Title level={3} style={{ margin: 0 }}>{labels.pageTitle}</Title>
          <Paragraph type="secondary" style={{ margin: "8px 0 0", maxWidth: 860 }}>{labels.pageDesc}</Paragraph>
        </div>
        <Space align="center" style={{ justifyContent: "space-between", width: "100%" }} wrap>
          <Text type="secondary" style={{ fontSize: 12 }}>{labels.pageHint}</Text>
          <Button size="small" icon={<CopyOutlined />} onClick={copyDiagnostics}>{labels.copyDiagnostics}</Button>
        </Space>
      </Space>
    </Card>
  );
}
