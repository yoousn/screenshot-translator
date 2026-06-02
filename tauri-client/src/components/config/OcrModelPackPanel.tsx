import React from "react";
import { Alert, Button, Space, Tag, Typography } from "antd";
import { ApiOutlined, CloudDownloadOutlined, ExperimentOutlined, ReloadOutlined } from "@ant-design/icons";
import ConfigSectionCard from "./ConfigSectionCard";
import ModelPackOperationStatus from "./ModelPackOperationStatus";
import ModelPackStatusList from "./ModelPackStatusList";
import { getDefaultOcrModelManifest } from "../../ocr-models";
import type { OcrModelPackOperation, YsnOcrRuntimeStatus, YsnOcrSelfTestResult } from "../../ocr-models";
import { useI18n } from "../../i18n";

const { Text } = Typography;
const BASE_PACK_ID = "auto-multilingual-balanced";

interface OcrModelPackPanelProps {
  runtimeStatus: YsnOcrRuntimeStatus | null;
  loadingRuntimeStatus: boolean;
  selfTesting: boolean;
  runningPackAction?: string | null;
  lastSelfTest?: YsnOcrSelfTestResult | null;
  lastOperation?: OcrModelPackOperation | null;
  onRefreshRuntimeStatus: () => void;
  onRunSelfTest: () => void;
  onInstallPack: (packId: string) => void;
  onUpdatePack: (packId: string) => void;
}

export default function OcrModelPackPanel({
  runtimeStatus,
  loadingRuntimeStatus,
  selfTesting,
  runningPackAction,
  lastSelfTest,
  lastOperation,
  onRefreshRuntimeStatus,
  onRunSelfTest,
  onInstallPack,
  onUpdatePack,
}: OcrModelPackPanelProps) {
  const { text } = useI18n();
  const labels = text.config;
  const modelManifest = runtimeStatus?.manifest || getDefaultOcrModelManifest();
  const basePack = modelManifest.packs.find((pack) => pack.id === BASE_PACK_ID) || modelManifest.packs.find((pack) => pack.required);
  const missingActiveModels = lastSelfTest?.missingActiveModels || [];
  const isReady = Boolean(runtimeStatus?.modelPacksReady && runtimeStatus?.activeModelsReady);
  const needsInstall = !runtimeStatus?.modelPacksReady;
  const needsRepair = Boolean(runtimeStatus?.modelPacksReady && !runtimeStatus?.activeModelsReady);
  const languages = basePack?.languages.length || 0;

  const statusAlert = isReady
    ? {
        type: "success" as const,
        message: "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6a21\u578b\u5df2\u5c31\u7eea",
        description: "\u73b0\u5728\u53ef\u4ee5\u76f4\u63a5\u7528 Ctrl+D \u8bc6\u522b\u6587\u5b57\uff0c\u7528\u7ffb\u8bd1\u6309\u94ae\u8fdb\u884c\u622a\u56fe\u7ffb\u8bd1\u3002",
      }
    : needsInstall
      ? {
          type: "warning" as const,
          message: "\u9700\u8981\u5b89\u88c5\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6a21\u578b",
          description: "\u8fd9\u91cc\u53ea\u6709\u4e00\u5957\u4ea7\u54c1\u80fd\u529b\uff1a\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u3002\u5b89\u88c5\u540e\u5e94\u7528\u4f1a\u81ea\u52a8\u7528\u5b83\u6765\u8bc6\u522b\u6587\u5b57\u3002",
        }
      : needsRepair
        ? {
            type: "warning" as const,
            message: "\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6a21\u578b\u9700\u8981\u4fee\u590d",
            description: "\u68c0\u6d4b\u5230\u6a21\u578b\u6587\u4ef6\u7f3a\u5931\u6216\u6821\u9a8c\u4e0d\u901a\u8fc7\uff0c\u70b9\u51fb\u4fee\u590d\u540e\u518d\u6d4b\u8bd5\u622a\u56fe\u7ffb\u8bd1\u3002",
          }
        : {
            type: "info" as const,
            message: "\u8bf7\u8fd0\u884c\u4e00\u6b21\u6a21\u578b\u81ea\u68c0",
            description: "\u81ea\u68c0\u53ea\u68c0\u67e5\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u6a21\u578b\u662f\u5426\u80fd\u88ab\u5e94\u7528\u6b63\u5e38\u4f7f\u7528\u3002",
          };

  const primaryActions = needsInstall ? (
    <Button type="primary" icon={<CloudDownloadOutlined />} loading={runningPackAction === BASE_PACK_ID} onClick={() => onInstallPack(BASE_PACK_ID)}>{labels.modelPackInstall}</Button>
  ) : needsRepair ? (
    <Button type="primary" danger icon={<CloudDownloadOutlined />} loading={runningPackAction === BASE_PACK_ID} onClick={() => onInstallPack(BASE_PACK_ID)}>{labels.modelPackRepair}</Button>
  ) : !lastSelfTest?.ok ? (
    <Button type="primary" icon={<ExperimentOutlined />} loading={selfTesting} onClick={onRunSelfTest}>{labels.selfTest}</Button>
  ) : (
    <Button icon={<ReloadOutlined />} loading={loadingRuntimeStatus} onClick={onRefreshRuntimeStatus}>{labels.refresh}</Button>
  );

  return (
    <ConfigSectionCard
      eyebrow="\u672c\u5730\u6a21\u578b"
      title={<span><ApiOutlined style={{ marginRight: 8 }} />\u672c\u5730\u622a\u56fe\u7ffb\u8bd1</span>}
      description="\u4e00\u4e2a\u7edf\u4e00\u7684\u672c\u5730\u622a\u56fe\u7ffb\u8bd1\u80fd\u529b\uff1a\u81ea\u52a8\u8bc6\u522b\u6587\u5b57\uff0c\u518d\u8fdb\u884c\u7ffb\u8bd1\u548c\u91cd\u7ed8\u3002"
      extra={<Button size="small" icon={<ReloadOutlined />} loading={loadingRuntimeStatus} onClick={onRefreshRuntimeStatus}>{labels.refresh}</Button>}
    >
      <Space direction="vertical" size={14} style={{ width: "100%" }}>
        <Alert type={statusAlert.type} showIcon message={statusAlert.message} description={statusAlert.description} action={primaryActions} />
        <Space wrap>
          <Tag color={isReady ? "green" : "orange"}>{isReady ? "\u53ef\u7528" : "\u5f85\u5b8c\u6210"}</Tag>
          <Tag color="blue">{languages} \u79cd\u8bed\u8a00\u8986\u76d6</Tag>
          <Tag color="purple">\u6a21\u578b\u76ee\u5f55\uff1a{runtimeStatus?.modelDir || "models/ocr"}</Tag>
        </Space>
        {lastOperation && <ModelPackOperationStatus operation={lastOperation} />}
        <ModelPackStatusList
          manifest={modelManifest}
          actionLoadingPackId={runningPackAction}
          selfTesting={selfTesting}
          sourceReadiness={runtimeStatus?.sourceReadiness || null}
          onInstallPack={onInstallPack}
          onUpdatePack={onUpdatePack}
          onSelfTest={onRunSelfTest}
        />
        {lastSelfTest && !lastSelfTest.ok && missingActiveModels.length > 0 && (
          <Alert
            type="warning"
            showIcon
            message="\u6a21\u578b\u6587\u4ef6\u9700\u8981\u4fee\u590d"
            description={<Text type="secondary">{missingActiveModels.slice(0, 6).join(", ")}</Text>}
          />
        )}
      </Space>
    </ConfigSectionCard>
  );
}
