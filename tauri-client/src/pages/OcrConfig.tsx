import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
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
  VideoCameraOutlined,
} from "@ant-design/icons";
import useOcrConfigController from "../hooks/useOcrConfigController";
import { REPO_URL, T, formatBytes, summarizeRelease } from "../utils/ocrConfigHelpers";

const { Title, Paragraph, Text } = Typography;

type FfmpegReleaseInfo = {
  tag: string;
  pageUrl?: string | null;
  assetName: string;
  downloadUrl: string;
  size?: number | null;
  installDir: string;
};

type FfmpegProgress = {
  phase: string;
  downloaded: number;
  total?: number | null;
  percent: number;
};

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
    chooseOcrRuntimeDir,
    moveOcrDir,
    openOcrDir,
  } = useOcrConfigController();

  const [ffmpegPath, setFfmpegPath] = useState("");
  const [ffmpegRelease, setFfmpegRelease] = useState<FfmpegReleaseInfo | null>(null);
  const [checkingFfmpeg, setCheckingFfmpeg] = useState(false);
  const [downloadingFfmpeg, setDownloadingFfmpeg] = useState(false);
  const [ffmpegProgress, setFfmpegProgress] = useState<FfmpegProgress | null>(null);

  useEffect(() => {
    const loadFfmpegPath = async () => {
      try {
        const stored = JSON.parse(await invoke<string>("get_config"));
        setFfmpegPath(stored.recordingFfmpegPath || "");
      } catch {}
    };
    loadFfmpegPath();
    const unlisten = listen<FfmpegProgress>("ffmpeg-download-progress", (event) => setFfmpegProgress(event.payload));
    return () => {
      unlisten.then((dispose) => dispose()).catch(() => undefined);
    };
  }, []);

  const saveFfmpegPath = async (path: string) => {
    const stored = JSON.parse(await invoke<string>("get_config"));
    stored.recordingFfmpegPath = path.trim();
    await invoke("save_config", { configStr: JSON.stringify(stored, null, 2) });
    setFfmpegPath(path.trim());
  };

  const checkFfmpegRelease = async () => {
    try {
      setCheckingFfmpeg(true);
      setFfmpegRelease(await invoke<FfmpegReleaseInfo>("get_ffmpeg_release_info"));
    } finally {
      setCheckingFfmpeg(false);
    }
  };

  const downloadFfmpegRelease = async () => {
    setDownloadingFfmpeg(true);
    try {
      const release = ffmpegRelease || await invoke<FfmpegReleaseInfo>("get_ffmpeg_release_info");
      setFfmpegRelease(release);
      const result = await invoke<{ path: string; installDir: string; bytes: number }>("download_ffmpeg_release", { url: release.downloadUrl, tag: release.tag });
      await saveFfmpegPath(result.path);
    } finally {
      setDownloadingFfmpeg(false);
    }
  };


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
                <Button icon={<FolderOpenOutlined />} onClick={chooseOcrRuntimeDir}>选择运行包</Button>
                <Button icon={<FolderOpenOutlined />} onClick={openOcrDir} disabled={!config.localOcrExecutablePath && !status?.path && !config.paddleOcrInstallDir}>{T.openDir}</Button>
                <Button loading={movingDir} onClick={moveOcrDir}>{T.moveInstallDir}</Button>
              </Space>
              <Descriptions size="small" column={1} bordered>
                <Descriptions.Item label={T.mode}><Tag color="green">{T.localOnly}</Tag></Descriptions.Item>
                <Descriptions.Item label={T.status}>{statusTag}</Descriptions.Item>
                <Descriptions.Item label="运行包">{status?.runtimeManifest?.name || "PaddleOCR-json"}</Descriptions.Item>
                <Descriptions.Item label="引擎">{status?.runtimeManifest?.engine || "paddleocr-json"}</Descriptions.Item>
                <Descriptions.Item label="协议">{status?.runtimeManifest?.protocol || "paddleocr-json-stdin"}</Descriptions.Item>
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

      <Card title={<span><VideoCameraOutlined style={{ marginRight: 8 }} />FFmpeg 录屏运行包</span>} bordered={false} style={{ borderRadius: 16 }}>
        <Space direction="vertical" size={12} style={{ width: "100%" }}>
          <Alert type="info" showIcon message="默认查找软件同级 ffmpeg\ffmpeg.exe，也可以从官方 GitHub 下载/更新。" />
          <Input value={ffmpegPath} placeholder="可留空自动查找软件同级 ffmpeg\ffmpeg.exe" onChange={(event) => setFfmpegPath(event.target.value)} />
          <Space wrap>
            <Button icon={<SaveOutlined />} onClick={() => saveFfmpegPath(ffmpegPath)}>保存路径</Button>
            <Button icon={<ReloadOutlined />} loading={checkingFfmpeg} onClick={checkFfmpegRelease}>检查官方版本</Button>
            <Button type="primary" icon={<CloudDownloadOutlined />} loading={downloadingFfmpeg} onClick={downloadFfmpegRelease}>下载/更新 ffmpeg</Button>
            <Button icon={<GithubOutlined />} onClick={() => openUrl("https://github.com/BtbN/FFmpeg-Builds/releases/latest")}>官方 GitHub</Button>
            <Button disabled={!ffmpegPath} onClick={() => ffmpegPath && openPath(ffmpegPath.replace(/[\\/][^\\/]+$/, ""))}>打开目录</Button>
          </Space>
          <Descriptions size="small" column={1} bordered>
            <Descriptions.Item label="当前路径">{ffmpegPath || "自动查找"}</Descriptions.Item>
            <Descriptions.Item label="官方版本">{ffmpegRelease ? `${ffmpegRelease.tag} / ${ffmpegRelease.assetName}` : "未检查"}</Descriptions.Item>
            <Descriptions.Item label="默认安装目录">{ffmpegRelease?.installDir || "软件同级 ffmpeg 目录"}</Descriptions.Item>
          </Descriptions>
          {ffmpegProgress && <Progress percent={ffmpegProgress.percent} status={ffmpegProgress.percent >= 100 ? "success" : "active"} format={() => `${ffmpegProgress.phase} ${ffmpegProgress.percent}%`} />}
        </Space>
      </Card>
    </Space>
  );
}
