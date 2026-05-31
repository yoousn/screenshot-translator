import React from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Input,
  InputNumber,
  Progress,
  Row,
  Space,
  Tag,
  Typography,
} from "antd";
import {
  CloudDownloadOutlined,
  FileSearchOutlined,
  FolderOpenOutlined,
  GithubOutlined,
  ReloadOutlined,
  SaveOutlined,
  SafetyCertificateOutlined,
} from "@ant-design/icons";
import useOcrConfigController from "../hooks/useOcrConfigController";
import { REPO_URL, T, formatBytes, summarizeRelease } from "../utils/ocrConfigHelpers";

const { Title, Paragraph, Text } = Typography;
export default function OcrConfig() {
  const {
    config,
    setConfig,
    latest,
    latestAsset,
    status,
    checking,
    checkingStatus,
    downloading,
    saving,
    downloadSize,
    downloadProgress,
    movingDir,
    hasUpdate,
    statusTag,
    saveConfig,
    checkOcrStatus,
    checkLatest,
    downloadLatest,
    moveOcrDir,
    openOcrDir,
  } = useOcrConfigController();


  return (
    <Space direction="vertical" size={16} style={{ width: "100%" }}>
      <Card bordered={false} style={{ borderRadius: 16 }}>
        <Title level={4} style={{ margin: 0 }}>{T.pageTitle}</Title>
        <Paragraph type="secondary" style={{ margin: "6px 0 0" }}>{T.pageDesc}</Paragraph>
      </Card>

      <Alert type="info" showIcon message={T.repoTitle} description={T.repoDesc} />

      <Row gutter={[16, 16]}>
        <Col span={12}>
          <Card title={T.localTitle} bordered={false} style={{ borderRadius: 16, height: "100%" }}>
            <Space direction="vertical" size={12} style={{ width: "100%" }}>
              <div>
                <Text strong>{T.exePath}</Text>
                <Input style={{ marginTop: 8 }} placeholder={T.exePlaceholder} value={config.localOcrExecutablePath || ""} onChange={(event) => setConfig({ ...config, localOcrExecutablePath: event.target.value })} />
              </div>
              <div>
                <Text strong>{T.timeout}</Text>
                <InputNumber min={500} max={30000} style={{ marginTop: 8, width: "100%" }} value={config.localOcrTimeoutMs || 15000} onChange={(value) => setConfig({ ...config, localOcrTimeoutMs: Number(value || 15000) })} />
              </div>
              <Space wrap>
                <Button type="primary" icon={<SaveOutlined />} loading={saving} onClick={() => saveConfig()}>{T.save}</Button>
                <Button icon={<SafetyCertificateOutlined />} loading={checkingStatus} onClick={() => checkOcrStatus()}>{T.checkStatus}</Button>
                <Button icon={<FolderOpenOutlined />} onClick={openOcrDir} disabled={!config.localOcrExecutablePath && !status?.path && !config.paddleOcrInstallDir}>{T.openDir}</Button>
                <Button loading={movingDir} onClick={moveOcrDir}>{T.moveInstallDir}</Button>
              </Space>
              <Descriptions size="small" column={1} bordered>
                <Descriptions.Item label={T.mode}><Tag color="green">{T.localOnly}</Tag></Descriptions.Item>
                <Descriptions.Item label={T.status}>{statusTag}</Descriptions.Item>
                <Descriptions.Item label={T.exePath}>{status?.path || config.localOcrExecutablePath || "-"}</Descriptions.Item>
              </Descriptions>
            </Space>
          </Card>
        </Col>

        <Col span={12}>
          <Card title={T.updateTitle} bordered={false} style={{ borderRadius: 16, height: "100%" }}>
            <Space direction="vertical" size={12} style={{ width: "100%" }}>
              <Space wrap>
                <Button icon={<ReloadOutlined />} loading={checking} onClick={checkLatest}>{T.check}</Button>
                <Button type="primary" icon={<CloudDownloadOutlined />} disabled={!latestAsset} loading={downloading} onClick={downloadLatest}>{config.paddleOcrInstallDir ? T.downloadUpdate : T.downloadFirst}</Button>
                <Button icon={<GithubOutlined />} onClick={() => openUrl(REPO_URL)}>{T.officialRepo}</Button>
              </Space>

              <Descriptions size="small" column={1} bordered>
                <Descriptions.Item label={T.downloadedVersion}>{config.paddleOcrReleaseTag || T.notDownloaded}</Descriptions.Item>
                <Descriptions.Item label={T.latestVersion}>{latest?.tag_name || T.notChecked} {hasUpdate && <Tag color="orange">{T.hasUpdate}</Tag>}</Descriptions.Item>
                <Descriptions.Item label={T.assetName}>{latestAsset?.name || config.paddleOcrReleaseAssetName || "-"}</Descriptions.Item>
                <Descriptions.Item label={T.lastChecked}>{config.paddleOcrReleaseCheckedAt || T.notChecked}</Descriptions.Item>
                <Descriptions.Item label={T.installDir}>{config.paddleOcrInstallDir || "-"}</Descriptions.Item>
                <Descriptions.Item label={T.downloadSize}>{formatBytes(downloadSize)}</Descriptions.Item>
              </Descriptions>

              {latest && (
                <Card size="small" title={latest.name || latest.tag_name} extra={<Button type="link" size="small" icon={<FileSearchOutlined />} onClick={() => openUrl(latest.html_url)}>{T.fullLog}</Button>}>
                  <pre style={{ whiteSpace: "pre-wrap", margin: 0, maxHeight: 180, overflow: "auto", fontSize: 12 }}>{summarizeRelease(latest.body)}</pre>
                </Card>
              )}

              {downloadProgress && (
                <Progress percent={downloadProgress.percent} status={downloadProgress.percent >= 100 ? "success" : "active"} format={() => `${downloadProgress.phase} ${downloadProgress.percent}%`} />
              )}
              {(config.localOcrExecutablePath || config.paddleOcrInstallDir) && <Button onClick={openOcrDir}>{T.openInstallDir}</Button>}
            </Space>
          </Card>
        </Col>
      </Row>
    </Space>
  );
}
