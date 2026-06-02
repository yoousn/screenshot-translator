import React from "react";
import { Button, Card, Space, Tag, Typography, message } from "antd";
import { CopyOutlined, GlobalOutlined, SafetyCertificateOutlined, VideoCameraOutlined } from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "../../i18n";
import { DEFAULT_TRANSLATION_SERVICE_URL } from "../../utils/translationService";
import translationGlossary from "../../utils/translationGlossary.json";
import { getTranslationMemoryStorageStats } from "../../utils/translationMemory";

const { Title, Paragraph, Text } = Typography;

export default function ConfigPageHeader() {
  const { text } = useI18n();
  const labels = text.config;

  const buildTranslationDiagnostics = async () => {
    const configStr = await invoke<string>("get_config");
    const config = JSON.parse(configStr || "{}");
    const serviceUrl = (
      config.preferLanServer && config.lanServerUrl
        ? config.lanServerUrl
        : (config.serverUrl || DEFAULT_TRANSLATION_SERVICE_URL)
    ).replace(/\/$/, "");

    let health: any = null;
    let healthError: string | null = null;
    try {
      const response = await fetch(`${serviceUrl}/api/health`, { method: "GET" });
      health = await response.json();
    } catch (error: any) {
      healthError = error?.message || String(error);
    }

    const serverTranslation = health?.translation || null;
    const qualityWarnings = [];
    if (serverTranslation?.quality_flags?.google_free_low_quality_risk) {
      qualityWarnings.push("google-free-low-quality-risk");
    }
    if (serverTranslation && serverTranslation.glossary_version !== translationGlossary.version) {
      qualityWarnings.push("glossary-version-mismatch");
    }
    if (serverTranslation && !serverTranslation.glossary_loaded) {
      qualityWarnings.push("server-glossary-not-loaded");
    }

    return {
      serviceUrl,
      configuredChannel: config.channel || "google",
      targetLang: config.targetLang || "zh",
      localGlossaryVersion: translationGlossary.version,
      localTranslationCache: getTranslationMemoryStorageStats(),
      serverHealth: health,
      serverTranslation,
      healthError,
      qualityWarnings,
    };
  };

  const copyDiagnostics = async () => {
    try {
      const report = await invoke<any>("get_diagnostics_report");
      report.translation = await buildTranslationDiagnostics();
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
