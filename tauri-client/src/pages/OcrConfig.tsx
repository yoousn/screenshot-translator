import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { Alert, Button, Card, Col, Descriptions, Input, InputNumber, message, Progress, Row, Space, Tag, Typography } from "antd";
import { CloudDownloadOutlined, FileSearchOutlined, FolderOpenOutlined, GithubOutlined, ReloadOutlined, SaveOutlined, SafetyCertificateOutlined, VideoCameraOutlined } from "@ant-design/icons";
import useOcrConfigController from "../hooks/useOcrConfigController";
import { REPO_URL, T, formatBytes, summarizeRelease } from "../utils/ocrConfigHelpers";

const { Title, Paragraph, Text } = Typography;

type FfmpegReleaseInfo = { tag: string; pageUrl?: string | null; assetName: string; downloadUrl: string; size?: number | null; installDir: string; };
type FfmpegProgress = { phase: string; downloaded: number; total?: number | null; percent: number; };
type RecordingInfo = { ffmpegFound: boolean; ffmpegPath?: string; isRecording: boolean; audioDevices: string[]; };

const openContainingDir = async (path: string) => {
  if (!path) return;
  await openPath(path.replace(/[\\/][^\\/]+$/, ""));
};

export default function OcrConfig() {
  const { config, setConfig, latest, latestAsset, status, checking, checkingStatus, downloading, saving, downloadSize, downloadProgress, movingDir, hasUpdate, statusTag, saveConfig, checkOcrStatus, checkLatest, downloadLatest, chooseOcrRuntimeDir, moveOcrDir, openOcrDir } = useOcrConfigController();
  const [ffmpegPath, setFfmpegPath] = useState("");
  const [defaultVideoDir, setDefaultVideoDir] = useState("");
  const [ffmpegRelease, setFfmpegRelease] = useState<FfmpegReleaseInfo | null>(null);
  const [checkingFfmpeg, setCheckingFfmpeg] = useState(false);
  const [downloadingFfmpeg, setDownloadingFfmpeg] = useState(false);
  const [ffmpegProgress, setFfmpegProgress] = useState<FfmpegProgress | null>(null);
  const [recordingInfo, setRecordingInfo] = useState<RecordingInfo | null>(null);
  const [checkingRecordingInfo, setCheckingRecordingInfo] = useState(false);

  useEffect(() => {
    const loadInitial = async () => {
      try {
        const stored = JSON.parse(await invoke<string>("get_config"));
        setFfmpegPath(stored.recordingFfmpegPath || "");
      } catch {}
      try { setDefaultVideoDir(await invoke<string>("get_default_recording_output_dir")); } catch {}
    };
    loadInitial();
    const unlisten = listen<FfmpegProgress>("ffmpeg-download-progress", (event) => setFfmpegProgress(event.payload));
    checkRecordingInfo();
    return () => { unlisten.then((dispose) => dispose()).catch(() => undefined); };
  }, []);

  const checkRecordingInfo = async () => {
    try {
      setCheckingRecordingInfo(true);
      const next = await invoke<RecordingInfo>("get_recording_info");
      setRecordingInfo(next);
      if (next.ffmpegPath) setFfmpegPath(next.ffmpegPath);
    } finally {
      setCheckingRecordingInfo(false);
    }
  };

  const saveFfmpegPath = async (path: string) => {
    const stored = JSON.parse(await invoke<string>("get_config"));
    stored.recordingFfmpegPath = path.trim();
    await invoke("save_config", { configStr: JSON.stringify(stored, null, 2) });
    setFfmpegPath(path.trim());
    message.success("FFmpeg path saved");
  };

  const chooseFfmpegPath = async () => {
    const selected = await invoke<string | null>("choose_ffmpeg_executable", { currentPath: ffmpegPath || null });
    if (!selected) return;
    await saveFfmpegPath(selected);
    await checkRecordingInfo();
  };

  const checkFfmpegRelease = async () => {
    try {
      setCheckingFfmpeg(true);
      setFfmpegRelease(await invoke<FfmpegReleaseInfo>("get_ffmpeg_release_info"));
      message.success("FFmpeg release checked");
    } catch (error: any) {
      message.error("FFmpeg release check failed: " + (error?.message || error));
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
      await checkRecordingInfo();
    } catch (error: any) {
      message.error("FFmpeg download failed: " + (error?.message || error));
    } finally {
      setDownloadingFfmpeg(false);
    }
  };

  const manifest = status?.runtimeManifest;
  const engine = (manifest?.engine || (status?.path?.toLowerCase().includes("paddle") ? "paddleocr-json" : "rapidocr-onnx")).toString();
  const isRapid = engine.toLowerCase().includes("rapid") || engine.toLowerCase().includes("onnx");

  return (
    <Space direction="vertical" size={16} style={{ width: "100%" }}>
      <Card bordered={false} style={{ borderRadius: 16 }}>
        <Title level={4} style={{ margin: 0 }}>识字模型 / 视频录制</Title>
        <Paragraph type="secondary" style={{ margin: "6px 0 0" }}>默认推荐 RapidOCR ONNX；PaddleOCR-json 保留为兼容模式。录制视频默认保存到系统 Videos\YSN。</Paragraph>
      </Card>
      <Alert type="info" showIcon message="RapidOCR ONNX is the main OCR runtime" description="Choose a RapidOCR ONNX runtime folder that contains ocr-runtime.json, or keep PaddleOCR-json as compatibility mode. The runtime output is normalized to the existing OcrBlock format." />
      <Row gutter={[16, 16]}>
        <Col xs={24} xl={12}>
          <Card title="OCR Runtime" bordered={false} style={{ borderRadius: 16, height: "100%" }}>
            <Space direction="vertical" size={12} style={{ width: "100%" }}>
              <div><Text strong>Runtime path</Text><Input style={{ marginTop: 8 }} placeholder="Leave empty to auto-detect app ocr runtime, or choose a RapidOCR ONNX / PaddleOCR-json folder." value={config.localOcrExecutablePath || ""} onChange={(event) => setConfig({ ...config, localOcrExecutablePath: event.target.value })} /></div>
              <div><Text strong>Local OCR timeout (ms)</Text><InputNumber style={{ marginTop: 8, width: "100%" }} min={3000} max={120000} step={1000} value={config.localOcrTimeoutMs || 15000} onChange={(value) => setConfig({ ...config, localOcrTimeoutMs: Number(value || 15000) })} /></div>
              <Space wrap><Button type="primary" icon={<SaveOutlined />} loading={saving} onClick={() => saveConfig()}>Save OCR config</Button><Button icon={<FolderOpenOutlined />} onClick={chooseOcrRuntimeDir}>Choose runtime folder</Button><Button icon={<SafetyCertificateOutlined />} loading={checkingStatus} onClick={() => checkOcrStatus()}>Check runtime</Button><Button icon={<FolderOpenOutlined />} disabled={!config.localOcrExecutablePath && !status?.path} onClick={openOcrDir}>Open runtime</Button></Space>
              <Descriptions size="small" column={1} bordered>
                <Descriptions.Item label="Status">{statusTag}</Descriptions.Item>
                <Descriptions.Item label="Recommended mode">{isRapid ? <Tag color="blue">RapidOCR ONNX</Tag> : <Tag color="orange">PaddleOCR-json compatibility</Tag>}</Descriptions.Item>
                <Descriptions.Item label="Resolved path">{status?.path || "Auto detect"}</Descriptions.Item>
                <Descriptions.Item label="Runtime name">{manifest?.name || (isRapid ? "RapidOCR ONNX" : "PaddleOCR-json")}</Descriptions.Item>
                <Descriptions.Item label="Engine">{engine}</Descriptions.Item>
                <Descriptions.Item label="Protocol">{manifest?.protocol || (isRapid ? "cli-json-file" : "paddleocr-json-stdin")}</Descriptions.Item>
                <Descriptions.Item label="Version">{manifest?.version || "-"}</Descriptions.Item>
              </Descriptions>
            </Space>
          </Card>
        </Col>
        <Col xs={24} xl={12}>
          <Card title="PaddleOCR-json Compatibility" bordered={false} style={{ borderRadius: 16, height: "100%" }}>
            <Space direction="vertical" size={12} style={{ width: "100%" }}>
              <Alert type="warning" showIcon message="Compatibility mode" description="Use this only when you need the old PaddleOCR-json runtime. RapidOCR ONNX is the preferred default for screenshots." />
              <Space wrap><Button icon={<ReloadOutlined />} loading={checking} onClick={checkLatest}>Check PaddleOCR release</Button><Button type="primary" ghost icon={<CloudDownloadOutlined />} loading={downloading} disabled={!latestAsset} onClick={downloadLatest}>{hasUpdate ? "Update PaddleOCR" : "Download PaddleOCR"}</Button><Button icon={<GithubOutlined />} onClick={() => openUrl(REPO_URL)}>Open repository</Button><Button icon={<FolderOpenOutlined />} loading={movingDir} onClick={moveOcrDir}>Move runtime folder</Button></Space>
              <Descriptions size="small" column={1} bordered>
                <Descriptions.Item label="Installed version">{config.paddleOcrReleaseTag || T.notDownloaded}</Descriptions.Item>
                <Descriptions.Item label="Latest version">{latest?.tag_name || T.notChecked} {hasUpdate && <Tag color="orange">Update available</Tag>}</Descriptions.Item>
                <Descriptions.Item label="Asset">{latestAsset?.name || config.paddleOcrReleaseAssetName || "-"}</Descriptions.Item>
                <Descriptions.Item label="Install directory">{config.paddleOcrInstallDir || "-"}</Descriptions.Item>
                <Descriptions.Item label="Download size">{formatBytes(downloadSize)}</Descriptions.Item>
              </Descriptions>
              {latest && <Card size="small" title={latest.name || latest.tag_name} extra={<Button type="link" size="small" icon={<FileSearchOutlined />} onClick={() => openUrl(latest.html_url)}>Release notes</Button>}><pre style={{ whiteSpace: "pre-wrap", margin: 0, maxHeight: 160, overflow: "auto", fontSize: 12 }}>{summarizeRelease(latest.body)}</pre></Card>}
              {downloadProgress && <Progress percent={downloadProgress.percent} status={downloadProgress.percent >= 100 ? "success" : "active"} format={() => `${downloadProgress.phase} ${downloadProgress.percent}%`} />}
            </Space>
          </Card>
        </Col>
      </Row>
      <Card title={<span><VideoCameraOutlined style={{ marginRight: 8 }} />Video Recording</span>} bordered={false} style={{ borderRadius: 16 }}>
        <Space direction="vertical" size={12} style={{ width: "100%" }}>
          <Alert type="info" showIcon message="Default video save folder" description={defaultVideoDir || "Videos\\YSN"} />
          <Input value={ffmpegPath} placeholder="Leave empty for auto-detect, or choose ffmpeg.exe" onChange={(event) => setFfmpegPath(event.target.value)} />
          <Space wrap><Button icon={<SaveOutlined />} onClick={() => saveFfmpegPath(ffmpegPath)}>Save FFmpeg path</Button><Button icon={<FolderOpenOutlined />} onClick={chooseFfmpegPath}>Choose ffmpeg.exe</Button><Button icon={<ReloadOutlined />} loading={checkingFfmpeg} onClick={checkFfmpegRelease}>Check official release</Button><Button icon={<SafetyCertificateOutlined />} loading={checkingRecordingInfo} onClick={checkRecordingInfo}>Check availability</Button><Button type="primary" icon={<CloudDownloadOutlined />} loading={downloadingFfmpeg} onClick={downloadFfmpegRelease}>Download / update FFmpeg</Button><Button icon={<GithubOutlined />} onClick={() => openUrl("https://github.com/BtbN/FFmpeg-Builds/releases/latest")}>Official GitHub</Button><Button disabled={!ffmpegPath} onClick={() => openContainingDir(ffmpegPath)}>Open FFmpeg directory</Button><Button disabled={!defaultVideoDir} onClick={() => defaultVideoDir && openPath(defaultVideoDir)}>Open Videos\YSN</Button></Space>
          <Descriptions size="small" column={1} bordered>
            <Descriptions.Item label="Configured path">{ffmpegPath || "Auto detect"}</Descriptions.Item>
            <Descriptions.Item label="Available">{recordingInfo?.ffmpegFound ? <Tag color="green">Available</Tag> : <Tag color="red">Unavailable</Tag>} {recordingInfo?.isRecording && <Tag color="blue">Recording</Tag>}</Descriptions.Item>
            <Descriptions.Item label="Resolved path">{recordingInfo?.ffmpegPath || "Not checked"}</Descriptions.Item>
            <Descriptions.Item label="Audio devices">{recordingInfo?.audioDevices?.length ? `${recordingInfo.audioDevices.length} detected` : "Not checked"}</Descriptions.Item>
            <Descriptions.Item label="Official release">{ffmpegRelease ? `${ffmpegRelease.tag} / ${ffmpegRelease.assetName}` : "Not checked"}</Descriptions.Item>
            <Descriptions.Item label="Default install directory">{ffmpegRelease?.installDir || "app ffmpeg directory"}</Descriptions.Item>
          </Descriptions>
          {ffmpegProgress && <Progress percent={ffmpegProgress.percent} status={ffmpegProgress.percent >= 100 ? "success" : "active"} format={() => `${ffmpegProgress.phase} ${ffmpegProgress.percent}%`} />}
        </Space>
      </Card>
    </Space>
  );
}
