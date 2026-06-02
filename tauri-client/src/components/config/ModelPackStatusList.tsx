import React from "react";
import { Button, List, Space, Tag, Tooltip, Typography } from "antd";
import { CheckCircleOutlined, CloudDownloadOutlined, ExperimentOutlined, ToolOutlined } from "@ant-design/icons";
import { getPackStatusColor, summarizeOcrModelHealth } from "../../ocr-models";
import type { OcrModelManifest, OcrModelSourceReadiness } from "../../ocr-models";
import { useI18n } from "../../i18n";

const { Text } = Typography;

interface ModelPackStatusListProps {
  manifest: OcrModelManifest;
  actionLoadingPackId?: string | null;
  selfTesting?: boolean;
  sourceReadiness?: OcrModelSourceReadiness | null;
  onInstallPack?: (packId: string) => void;
  onUpdatePack?: (packId: string) => void;
  onSelfTest?: () => void;
}

const isBrokenStatus = (status: string) => status.includes("failed") || status === "broken";

export default function ModelPackStatusList({
  manifest,
  actionLoadingPackId,
  selfTesting,
  sourceReadiness,
  onInstallPack,
  onUpdatePack,
  onSelfTest,
}: ModelPackStatusListProps) {
  const { text } = useI18n();
  const labels = text.config;
  const health = summarizeOcrModelHealth(manifest);
  const pendingSourceModelIds = new Set(sourceReadiness?.pendingModelIds || []);

  const hasPendingSources = (packId: string) => {
    const pack = manifest.packs.find((item) => item.id === packId);
    return Boolean(pack?.modelIds.some((modelId) => pendingSourceModelIds.has(modelId)));
  };

  const withSourceGate = (packId: string, button: React.ReactElement<{ disabled?: boolean }>) => {
    if (!hasPendingSources(packId)) return button;
    return <Tooltip title={labels.configureManagedModelSources}>{React.cloneElement(button, { disabled: true })}</Tooltip>;
  };

  const renderAction = (packId: string, status: string) => {
    if (status === "installed") {
      return <Button size="small" icon={<ExperimentOutlined />} loading={selfTesting} onClick={onSelfTest}>{labels.selfTest}</Button>;
    }
    if (status === "update-available") {
      return withSourceGate(packId, <Button size="small" type="primary" icon={<CloudDownloadOutlined />} loading={actionLoadingPackId === packId} onClick={() => onUpdatePack?.(packId)}>{labels.modelPackUpdate}</Button>);
    }
    if (isBrokenStatus(status)) {
      return withSourceGate(packId, <Button size="small" danger icon={<ToolOutlined />} loading={actionLoadingPackId === packId} onClick={() => onInstallPack?.(packId)}>{labels.modelPackRepair}</Button>);
    }
    return withSourceGate(packId, <Button size="small" type="primary" ghost icon={<CloudDownloadOutlined />} loading={actionLoadingPackId === packId} onClick={() => onInstallPack?.(packId)}>{labels.modelPackInstall}</Button>);
  };

  return (
    <Space direction="vertical" size={10} style={{ width: "100%" }}>
      <Space wrap>
        <Tag color={health.ready ? "green" : "orange"} icon={<CheckCircleOutlined />}>{health.installed}/{health.required} {labels.requiredPacksReady}</Tag>
        {health.missing.length > 0 && <Tag color="orange">{labels.missing} {health.missing.length}</Tag>}
        {health.broken.length > 0 && <Tag color="red">{labels.repairNeeded} {health.broken.length}</Tag>}
        {health.updateAvailable.length > 0 && <Tag color="blue">{labels.updates} {health.updateAvailable.length}</Tag>}
      </Space>
      <List
        size="small"
        dataSource={manifest.packs}
        renderItem={(pack) => (
          <List.Item actions={[renderAction(pack.id, pack.status)]}>
            <List.Item.Meta
              title={<Space wrap><Text strong>{pack.name["zh-CN"]}</Text><Tag color={getPackStatusColor(pack.status)}>{pack.status}</Tag>{pack.required && <Tag color="red">{labels.required}</Tag>}{hasPendingSources(pack.id) && <Tag color="orange">{labels.trustedSourcesPending}</Tag>}</Space>}
              description={`${pack.name["en-US"]} · ${pack.profile} · ${pack.languages.join(", ")}`}
            />
          </List.Item>
        )}
      />
    </Space>
  );
}
