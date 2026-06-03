import type React from "react";
import { Card, Col, Progress, Row, Space, Tag, Typography } from "antd";
import { CheckCircleOutlined, ExclamationCircleOutlined, GlobalOutlined, VideoCameraOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";
import ConfigRecoveryChecklist from "./ConfigRecoveryChecklist";
import type { RecordingInfo } from "./types";
import type { RapidOcrStatus } from "../../ocr-models";

const { Text } = Typography;

type ConfigReadinessOverviewProps = {
  runtimeStatus: RapidOcrStatus | null;
  recordingInfo: RecordingInfo | null;
  targetLang: string;
};

type ReadinessItem = {
  title: string;
  desc: string;
  ready: boolean;
  accent: string;
  icon: React.ReactNode;
  readyLabel: string;
  actionLabel: string;
};

function ReadinessTile({ item }: { item: ReadinessItem }) {
  return (
    <Card size="small" bordered={false} style={{ height: "100%", borderRadius: 16, background: "rgba(255,255,255,0.78)", boxShadow: "0 10px 30px rgba(15,23,42,0.06)" }}>
      <Space direction="vertical" size={8} style={{ width: "100%" }}>
        <Space align="center" style={{ width: "100%", justifyContent: "space-between" }}>
          <Space>
            <span style={{ color: item.accent, fontSize: 18 }}>{item.icon}</span>
            <Text strong>{item.title}</Text>
          </Space>
          <Tag color={item.ready ? "green" : "orange"}>{item.ready ? item.readyLabel : item.actionLabel}</Tag>
        </Space>
        <Text type="secondary" style={{ fontSize: 12 }}>{item.desc}</Text>
      </Space>
    </Card>
  );
}

export default function ConfigReadinessOverview({ runtimeStatus, recordingInfo, targetLang }: ConfigReadinessOverviewProps) {
  const { text } = useI18n();
  const labels = text.config;
  const items: ReadinessItem[] = [
    {
      title: labels.overviewOcrRuntime,
      desc: runtimeStatus?.runtimeInferenceReady
        ? runtimeStatus.workerEnabled
          ? "RapidOCR runner、模型目录与常驻服务配置已就绪。"
          : "RapidOCR runner 与模型 probe 已通过。"
        : "RapidOCR runner 尚未完成自测。",
      ready: Boolean(runtimeStatus?.runtimeInferenceReady),
      accent: "#2563eb",
      icon: <CheckCircleOutlined />,
      readyLabel: labels.readinessReady,
      actionLabel: labels.overviewOcrRuntimeAction || labels.readinessAction,
    },
    {
      title: labels.overviewModelPacks,
      desc: runtimeStatus?.modelPacksReady
        ? `当前模型：Rapid OCR ${(runtimeStatus.rapidOcrModelVersion || "v5").toUpperCase()}`
        : "请运行 RapidOCR 自测或修复 runner。",
      ready: Boolean(runtimeStatus?.modelPacksReady),
      accent: "#7c3aed",
      icon: <CheckCircleOutlined />,
      readyLabel: labels.readinessReady,
      actionLabel: labels.overviewModelPacksAction || labels.readinessAction,
    },
    {
      title: labels.overviewTranslation,
      desc: `${labels.overviewTargetLanguage}: ${targetLang || "zh"}`,
      ready: Boolean(targetLang),
      accent: "#0891b2",
      icon: <GlobalOutlined />,
      readyLabel: labels.readinessReady,
      actionLabel: labels.overviewTranslationAction || labels.readinessAction,
    },
    {
      title: labels.overviewRecording,
      desc: recordingInfo?.ffmpegFound ? labels.overviewRecordingReady : labels.overviewRecordingPending,
      ready: Boolean(recordingInfo?.ffmpegFound),
      accent: "#ea580c",
      icon: recordingInfo?.ffmpegFound ? <VideoCameraOutlined /> : <ExclamationCircleOutlined />,
      readyLabel: labels.readinessReady,
      actionLabel: labels.overviewRecordingAction || labels.readinessAction,
    },
  ];
  const readyCount = items.filter((item) => item.ready).length;
  const percent = Math.round((readyCount / items.length) * 100);

  return (
    <Card bordered={false} style={{ borderRadius: 20, background: "linear-gradient(135deg, rgba(37,99,235,0.10), rgba(20,184,166,0.08), rgba(249,115,22,0.10))" }}>
      <Space direction="vertical" size={14} style={{ width: "100%" }}>
        <Space wrap align="center" style={{ width: "100%", justifyContent: "space-between" }}>
          <div>
            <Text style={{ display: "block", color: "#2563eb", fontSize: 11, fontWeight: 800, letterSpacing: 0.4, textTransform: "uppercase" }}>{labels.overviewEyebrow}</Text>
            <Text strong style={{ display: "block", fontSize: 18, color: "#0f172a" }}>{labels.overviewTitle}</Text>
            <Text type="secondary" style={{ fontSize: 12 }}>{labels.overviewDesc}</Text>
          </div>
          <div style={{ minWidth: 180 }}>
            <Progress percent={percent} size="small" status={percent === 100 ? "success" : "active"} />
            <Text type="secondary" style={{ fontSize: 12 }}>{readyCount}/{items.length} {labels.overviewReadyCount}</Text>
          </div>
        </Space>
        <Row gutter={[12, 12]}>
          {items.map((item) => (
            <Col xs={24} md={12} xl={6} key={item.title}>
              <ReadinessTile item={item} />
            </Col>
          ))}
        </Row>
        <ConfigRecoveryChecklist runtimeStatus={runtimeStatus} recordingInfo={recordingInfo} />
      </Space>
    </Card>
  );
}
