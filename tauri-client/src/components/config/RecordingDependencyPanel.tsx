import React from "react";
import { Alert, Button, Descriptions, Input, Progress, Space, Tag } from "antd";
import { CloudDownloadOutlined, FolderOpenOutlined, GithubOutlined, ReloadOutlined, SaveOutlined, SafetyCertificateOutlined, VideoCameraOutlined } from "@ant-design/icons";
import ConfigSectionCard from "./ConfigSectionCard";
import type { RecordingDependencyPanelProps } from "./types";
import { useI18n } from "../../i18n";

export default function RecordingDependencyPanel({
  ffmpegPath,
  defaultVideoDir,
  ffmpegRelease,
  ffmpegProgress,
  recordingInfo,
  checkingFfmpeg,
  checkingRecordingInfo,
  downloadingFfmpeg,
  onSetFfmpegPath,
  onSaveFfmpegPath,
  onChooseFfmpegPath,
  onCheckFfmpegRelease,
  onCheckRecordingInfo,
  onDownloadFfmpeg,
  onOpenFfmpegRepo,
  onOpenFfmpegDir,
  onOpenVideoDir,
}: RecordingDependencyPanelProps) {
  const { text } = useI18n();
  const labels = text.config;

  return (
    <ConfigSectionCard
      eyebrow={labels.recordingEyebrow}
      title={<span><VideoCameraOutlined style={{ marginRight: 8 }} />{labels.recordingTitle}</span>}
      description={labels.recordingDesc}
    >
      <Alert type="info" showIcon message={labels.defaultVideoSaveFolder} description={defaultVideoDir || "Videos\\YSN"} />
      <Input value={ffmpegPath} placeholder={labels.ffmpegPathPlaceholder} onChange={(event) => onSetFfmpegPath(event.target.value)} />
      <Space wrap>
        <Button icon={<SaveOutlined />} onClick={onSaveFfmpegPath}>{labels.saveFfmpegPath}</Button>
        <Button icon={<FolderOpenOutlined />} onClick={onChooseFfmpegPath}>{labels.chooseFfmpeg}</Button>
        <Button icon={<ReloadOutlined />} loading={checkingFfmpeg} onClick={onCheckFfmpegRelease}>{labels.checkOfficialRelease}</Button>
        <Button icon={<SafetyCertificateOutlined />} loading={checkingRecordingInfo} onClick={onCheckRecordingInfo}>{labels.checkAvailability}</Button>
        <Button type="primary" icon={<CloudDownloadOutlined />} loading={downloadingFfmpeg} onClick={onDownloadFfmpeg}>{labels.downloadUpdateFfmpeg}</Button>
        <Button icon={<GithubOutlined />} onClick={onOpenFfmpegRepo}>{labels.officialGithub}</Button>
        <Button disabled={!ffmpegPath} onClick={onOpenFfmpegDir}>{labels.openFfmpegDirectory}</Button>
        <Button disabled={!defaultVideoDir} onClick={onOpenVideoDir}>{labels.openVideosYsn}</Button>
      </Space>
      <Descriptions size="small" column={1} bordered>
        <Descriptions.Item label={labels.configuredPath}>{ffmpegPath || labels.autoDetect}</Descriptions.Item>
        <Descriptions.Item label={labels.available}>{recordingInfo?.ffmpegFound ? <Tag color="green">{labels.availableValue}</Tag> : <Tag color="red">{labels.unavailableValue}</Tag>} {recordingInfo?.isRecording && <Tag color="blue">{labels.recording}</Tag>}</Descriptions.Item>
        <Descriptions.Item label={labels.resolvedPath}>{recordingInfo?.ffmpegPath || labels.notChecked}</Descriptions.Item>
        <Descriptions.Item label={labels.audioDevices}>{recordingInfo?.audioDevices?.length ? labels.audioDevicesDetected.replace("{count}", String(recordingInfo.audioDevices.length)) : labels.notChecked}</Descriptions.Item>
        <Descriptions.Item label={labels.officialRelease}>{ffmpegRelease ? `${ffmpegRelease.tag} / ${ffmpegRelease.assetName}` : labels.notChecked}</Descriptions.Item>
        <Descriptions.Item label={labels.defaultInstallDirectory}>{ffmpegRelease?.installDir || labels.appFfmpegDirectory}</Descriptions.Item>
      </Descriptions>
      {ffmpegProgress && <Progress percent={ffmpegProgress.percent} status={ffmpegProgress.percent >= 100 ? "success" : "active"} format={() => `${ffmpegProgress.phase} ${ffmpegProgress.percent}%`} />}
    </ConfigSectionCard>
  );
}
