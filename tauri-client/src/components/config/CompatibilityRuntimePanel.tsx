import React from "react";
import { Alert, Button, Card, Descriptions, Progress, Space, Tag } from "antd";
import { CloudDownloadOutlined, FileSearchOutlined, FolderOpenOutlined, GithubOutlined, ReloadOutlined } from "@ant-design/icons";
import ConfigSectionCard from "./ConfigSectionCard";
import type { CompatibilityRuntimePanelProps } from "./types";
import { formatBytes, summarizeRelease } from "../../utils/ocrConfigHelpers";
import { useI18n } from "../../i18n";

export default function CompatibilityRuntimePanel({
  config,
  latest,
  latestAsset,
  checking,
  downloading,
  movingDir,
  hasUpdate,
  downloadSize,
  downloadProgress,
  onCheckLatest,
  onDownloadLatest,
  onOpenRepo,
  onOpenReleaseNotes,
  onMoveRuntimeDir,
}: CompatibilityRuntimePanelProps) {
  const { text } = useI18n();
  const labels = text.config;

  return (
    <ConfigSectionCard eyebrow={labels.compatibilityEyebrow} title={labels.compatibilityTitle} description={labels.compatibilityDesc}>
      <Alert type="warning" showIcon message={labels.compatibilityWarning} description={labels.compatibilityWarningDesc} />
      <Space wrap>
        <Button icon={<ReloadOutlined />} loading={checking} onClick={onCheckLatest}>{labels.checkPaddleRelease}</Button>
        <Button type="primary" ghost icon={<CloudDownloadOutlined />} loading={downloading} disabled={!latestAsset} onClick={onDownloadLatest}>{hasUpdate ? labels.updatePaddle : labels.downloadPaddle}</Button>
        <Button icon={<GithubOutlined />} onClick={onOpenRepo}>{labels.openRepository}</Button>
        <Button icon={<FolderOpenOutlined />} loading={movingDir} onClick={onMoveRuntimeDir}>{labels.moveRuntimeFolder}</Button>
      </Space>
      <Descriptions size="small" column={1} bordered>
        <Descriptions.Item label={labels.installedVersion}>{config.paddleOcrReleaseTag || labels.notInstalled}</Descriptions.Item>
        <Descriptions.Item label={labels.latestVersion}>{latest?.tag_name || labels.notChecked} {hasUpdate && <Tag color="orange">{labels.updateAvailable}</Tag>}</Descriptions.Item>
        <Descriptions.Item label={labels.asset}>{latestAsset?.name || config.paddleOcrReleaseAssetName || "-"}</Descriptions.Item>
        <Descriptions.Item label={labels.installDirectory}>{config.paddleOcrInstallDir || "-"}</Descriptions.Item>
        <Descriptions.Item label={labels.downloadSize}>{formatBytes(downloadSize)}</Descriptions.Item>
      </Descriptions>
      {latest && <Card size="small" title={latest.name || latest.tag_name} extra={<Button type="link" size="small" icon={<FileSearchOutlined />} onClick={() => latest.html_url && onOpenReleaseNotes(latest.html_url)} >{labels.releaseNotes}</Button>}><pre style={{ whiteSpace: "pre-wrap", margin: 0, maxHeight: 160, overflow: "auto", fontSize: 12 }}>{summarizeRelease(latest.body)}</pre></Card>}
      {downloadProgress && <Progress percent={downloadProgress.percent} status={downloadProgress.percent >= 100 ? "success" : "active"} format={() => `${downloadProgress.phase} ${downloadProgress.percent}%`} />}
    </ConfigSectionCard>
  );
}
