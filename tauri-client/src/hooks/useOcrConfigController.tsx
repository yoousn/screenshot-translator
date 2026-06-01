import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { message, Tag } from "antd";
import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { DownloadResult, GitHubAsset, GitHubRelease, LocalConfig, ProgressPayload, REPO_API, StatusResult, T, pickWindowsAsset } from "../utils/ocrConfigHelpers";

export default function useOcrConfigController() {
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


  const chooseOcrRuntimeDir = async () => {
    setCheckingStatus(true);
    try {
      const currentDir = config.localOcrExecutablePath || config.paddleOcrInstallDir || "";
      const selectedDir = await invoke<string | null>("choose_ocr_runtime_dir", { currentDir });
      if (!selectedDir) return;
      const result = await checkOcrStatus(selectedDir, false);
      if (!result?.ok) {
        message.error("所选目录没有可用 OCR 运行入口");
        return;
      }
      await saveConfig({
        localOcrExecutablePath: selectedDir,
        paddleOcrInstallDir: selectedDir,
      }, false);
      message.success("OCR 运行包已切换");
    } catch (error: any) {
      message.error("选择 OCR 运行包失败：" + (error?.message || error));
    } finally {
      setCheckingStatus(false);
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
    message.info("Please choose a RapidOCR ONNX or PaddleOCR-json runtime first.");
  };

  const hasUpdate = Boolean(latest && latest.tag_name !== config.paddleOcrReleaseTag);
  const statusTag = status?.ok ? <Tag color="green">{T.statusOk}</Tag> : status ? <Tag color="red">{T.statusBad}</Tag> : <Tag>{T.statusUnknown}</Tag>;

  return {
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
  };
}
