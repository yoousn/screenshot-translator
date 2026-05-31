import React, { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { openPath, openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Input,
  InputNumber,
  message,
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

const { Title, Paragraph, Text } = Typography;

const REPO_API = "https://api.github.com/repos/hiroi-sora/PaddleOCR-json/releases/latest";
const REPO_URL = "https://github.com/hiroi-sora/PaddleOCR-json";

const T = {
  pageTitle: "OCR 配置",
  pageDesc: "当前版本强制使用客户端本地 PaddleOCR-json 做 OCR 识别；N100 后端只接收文本进行翻译，不再接收图片做云端 OCR。",
  repoTitle: "PaddleOCR-json 运行包说明",
  repoDesc: "这里下载的是可直接运行的 PaddleOCR-json Windows x64 发布包，不再下载 PaddlePaddle/PaddleOCR 源码包。下载后会自动解压到应用本地 OCR 运行目录，删除压缩包，并把 PaddleOCR-json.exe 设置为默认调用路径。",
  localTitle: "本地 OCR 执行配置",
  exePath: "PaddleOCR-json.exe 物理路径",
  exePlaceholder: "留空则优先使用应用数据目录中的 OCR 运行包，其次使用内置 resources/ocr/PaddleOCR-json.exe",
  timeout: "本地 OCR 超时限制 (ms)",
  save: "保存本地 OCR 配置",
  saved: "OCR 配置已保存",
  openDir: "打开所在目录",
  mode: "OCR 模式",
  localOnly: "强制本地",
  status: "可用状态",
  statusUnknown: "未检查",
  statusOk: "可用",
  statusBad: "不可用",
  checkStatus: "手动检查可用状态",
  checkedStatus: "OCR 状态检查完成",
  updateTitle: "PaddleOCR-json 更新",
  check: "手动检查更新",
  downloadFirst: "下载并安装最新版",
  downloadUpdate: "更新并安装最新版",
  officialRepo: "运行包仓库",
  downloadedVersion: "已安装版本",
  latestVersion: "最新版本",
  assetName: "运行包文件",
  lastChecked: "上次检查",
  installDir: "安装目录",
  downloadSize: "下载大小",
  notDownloaded: "未安装",
  notChecked: "未检查",
  hasUpdate: "有更新",
  fullLog: "完整日志",
  openInstallDir: "打开 OCR 所在目录",
  moveInstallDir: "移动 OCR 目录",
  moving: "正在移动 OCR 目录...",
  moved: "OCR 目录已移动",
  checkFirst: "请先检查更新",
  noWindowsAsset: "最新 Release 未找到 Windows x64 .7z 运行包。",
  officialNoLog: "官方 Release 未提供更新说明。",
  checkedLatest: "检测到 PaddleOCR-json 最新版本：",
  downloaded: "已安装 PaddleOCR-json ",
  saveFailed: "保存失败：",
  checkFailed: "检查更新失败：",
  statusFailed: "检查可用状态失败：",
  downloadFailed: "下载/安装失败：",
};

interface LocalConfig {
  localOcrExecutablePath?: string;
  localOcrTimeoutMs?: number;
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  paddleOcrReleaseTag?: string;
  paddleOcrReleasePath?: string;
  paddleOcrInstallDir?: string;
  paddleOcrReleaseAssetName?: string;
  paddleOcrReleaseCheckedAt?: string;
}

interface GitHubAsset {
  name: string;
  browser_download_url: string;
  size?: number;
}

interface GitHubRelease {
  tag_name: string;
  name?: string;
  html_url: string;
  published_at?: string;
  body?: string;
  assets?: GitHubAsset[];
}

interface DownloadResult {
  path: string;
  installDir: string;
  bytes: number;
}

interface ProgressPayload {
  phase: string;
  downloaded: number;
  total?: number;
  percent: number;
}

interface StatusResult {
  ok: boolean;
  path: string;
  exists: boolean;
  isFile: boolean;
  parentExists: boolean;
}

function summarizeRelease(body?: string) {
  if (!body) return T.officialNoLog;
  return body
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(0, 8)
    .join("\n");
}

function formatBytes(bytes?: number) {
  if (!bytes || bytes <= 0) return "-";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / 1024 / 1024).toFixed(1) + " MB";
}

function pickWindowsAsset(release: GitHubRelease) {
  return (release.assets || []).find((asset) => {
    const name = asset.name.toLowerCase();
    return name.includes("windows") && name.includes("x64") && name.endsWith(".7z");
  });
}

export default function OcrConfig() {
  const [config, setConfig] = useState<LocalConfig>({});
  const [latest, setLatest] = useState<GitHubRelease | null>(null);
  const [latestAsset, setLatestAsset] = useState<GitHubAsset | null>(null);
  const [status, setStatus] = useState<StatusResult | null>(null);
  const [checking, setChecking] = useState(false);
  const [checkingStatus, setCheckingStatus] = useState(false);
  const [downloading, setDownloading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [downloadSize, setDownloadSize] = useState<number | undefined>();
  const [downloadProgress, setDownloadProgress] = useState<ProgressPayload | null>(null);
  const [movingDir, setMovingDir] = useState(false);

  useEffect(() => {
    loadConfig();
    let unlisten: (() => void) | undefined;
    listen<ProgressPayload>("ocr-download-progress", (event) => setDownloadProgress(event.payload))
      .then((fn) => { unlisten = fn; })
      .catch(() => {});
    return () => { if (unlisten) unlisten(); };
  }, []);

  const loadConfig = async () => {
    const configStr = await invoke<string>("get_config");
    const parsed = configStr ? JSON.parse(configStr) : {};
    const next = {
      ...parsed,
      useLocalOcr: true,
      fallbackToRemoteOcr: false,
      localOcrTimeoutMs: parsed.localOcrTimeoutMs || 15000,
    };
    setConfig(next);
    await checkOcrStatus(next.localOcrExecutablePath, false);
  };

  const saveConfig = async (patch: Partial<LocalConfig> = {}, showMessage = true) => {
    setSaving(true);
    try {
      const next = { ...config, ...patch, useLocalOcr: true, fallbackToRemoteOcr: false };
      await invoke("save_config", { configStr: JSON.stringify(next) });
      setConfig(next);
      if (showMessage) message.success(T.saved);
    } catch (error: any) {
      message.error(T.saveFailed + (error?.message || error));
    } finally {
      setSaving(false);
    }
  };

  const checkOcrStatus = async (path?: string, showMessage = true) => {
    setCheckingStatus(true);
    try {
      const result = await invoke<StatusResult>("check_local_ocr_status", {
        executablePath: path || config.localOcrExecutablePath || null,
      });
      setStatus(result);
      if (showMessage) message.success(T.checkedStatus);
      return result;
    } catch (error: any) {
      setStatus({ ok: false, path: path || config.localOcrExecutablePath || "", exists: false, isFile: false, parentExists: false });
      if (showMessage) message.error(T.statusFailed + (error?.message || error));
      return null;
    } finally {
      setCheckingStatus(false);
    }
  };

  const checkLatest = async () => {
    setChecking(true);
    try {
      const response = await fetch(REPO_API, { headers: { Accept: "application/vnd.github+json" } });
      if (!response.ok) throw new Error("GitHub HTTP " + response.status);
      const release = await response.json() as GitHubRelease;
      const asset = pickWindowsAsset(release);
      if (!asset) throw new Error(T.noWindowsAsset);
      setLatest(release);
      setLatestAsset(asset);
      setDownloadSize(asset.size);
      await saveConfig({ paddleOcrReleaseCheckedAt: new Date().toLocaleString() }, false);
      message.success(T.checkedLatest + release.tag_name);
    } catch (error: any) {
      message.error(T.checkFailed + (error?.message || error));
    } finally {
      setChecking(false);
    }
  };

  const downloadLatest = async () => {
    if (!latest || !latestAsset) {
      message.info(T.checkFirst);
      return;
    }
    setDownloading(true);
    try {
      setDownloadProgress({ phase: "准备下载", downloaded: 0, total: latestAsset.size, percent: 1 });
      const result = await invoke<DownloadResult>("download_paddleocr_release", {
        url: latestAsset.browser_download_url,
        tag: latest.tag_name,
        installDir: config.paddleOcrInstallDir || null,
      });
      setDownloadSize(result.bytes);
      await saveConfig({
        localOcrExecutablePath: result.path,
        paddleOcrReleaseTag: latest.tag_name,
        paddleOcrReleasePath: undefined,
        paddleOcrInstallDir: result.installDir,
        paddleOcrReleaseAssetName: latestAsset.name,
        paddleOcrReleaseCheckedAt: new Date().toLocaleString(),
      }, false);
      await checkOcrStatus(result.path, false);
      setDownloadProgress({ phase: "完成", downloaded: result.bytes, total: result.bytes, percent: 100 });
      message.success(T.downloaded + latest.tag_name);
    } catch (error: any) {
      message.error(T.downloadFailed + (error?.message || error));
    } finally {
      setDownloading(false);
    }
  };

  const moveOcrDir = async () => {
    setMovingDir(true);
    try {
      const targetDir = await invoke<string | null>("choose_ocr_install_dir");
      if (!targetDir) return;
      message.loading({ content: T.moving, key: "move-ocr", duration: 0 });
      const result = await invoke<DownloadResult>("move_ocr_runtime", {
        targetDir,
        executablePath: config.localOcrExecutablePath || null,
      });
      await saveConfig({
        localOcrExecutablePath: result.path,
        paddleOcrInstallDir: result.installDir,
      }, false);
      await checkOcrStatus(result.path, false);
      message.success({ content: T.moved, key: "move-ocr" });
    } catch (error: any) {
      message.error({ content: (error?.message || error), key: "move-ocr" });
    } finally {
      setMovingDir(false);
    }
  };

  const openOcrDir = async () => {
    const exePath = config.localOcrExecutablePath || status?.path;
    const installDir = config.paddleOcrInstallDir;
    try {
      if (exePath && exePath !== "-") {
        await revealItemInDir(exePath);
        return;
      }
    } catch (error) {
      if (!installDir) throw error;
    }
    if (installDir) {
      await openPath(installDir);
      return;
    }
    message.info("\u8bf7\u5148\u4e0b\u8f7d\u6216\u9009\u62e9 PaddleOCR-json.exe");
  };

  const hasUpdate = Boolean(latest && latest.tag_name !== config.paddleOcrReleaseTag);
  const statusTag = status?.ok ? <Tag color="green">{T.statusOk}</Tag> : status ? <Tag color="red">{T.statusBad}</Tag> : <Tag>{T.statusUnknown}</Tag>;

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
