import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { message } from "antd";
import type { FfmpegProgress, FfmpegReleaseInfo, RecordingInfo } from "../components/config/types";
import { useI18n } from "../i18n";
import { readStartupReadinessSnapshot } from "./useStartupDependencyStatus";

const FFMPEG_RELEASE_URL = "https://github.com/BtbN/FFmpeg-Builds/releases/latest";

const openContainingDir = async (path: string) => {
  if (!path) return;
  await invoke("open_path_in_file_manager", { path: path.replace(/[\\/][^\\/]+$/, "") });
};

type UseRecordingDependencyControllerOptions = {
  autoCheck?: boolean;
};

export default function useRecordingDependencyController(options: UseRecordingDependencyControllerOptions = {}) {
  const { autoCheck = false } = options;
  const { text } = useI18n();
  const labels = text.config;
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

      try {
        setDefaultVideoDir(await invoke<string>("get_default_recording_output_dir"));
      } catch {}

      try {
        const snapshot = await readStartupReadinessSnapshot();
        if (snapshot?.recording) setRecordingInfo(snapshot.recording);
      } catch {}
    };

    loadInitial();
    if (autoCheck) {
      checkRecordingInfo();
    }

    const unlisten = listen<FfmpegProgress>("ffmpeg-download-progress", (event) => setFfmpegProgress(event.payload));
    return () => { unlisten.then((dispose) => dispose()).catch(() => undefined); };
  }, [autoCheck]);

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

  const saveFfmpegPath = async (path = ffmpegPath) => {
    const stored = JSON.parse(await invoke<string>("get_config"));
    stored.recordingFfmpegPath = path.trim();
    await invoke("save_config", { configStr: JSON.stringify(stored, null, 2) });
    setFfmpegPath(path.trim());
    message.success(labels.ffmpegPathSaved);
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
      message.success(labels.ffmpegReleaseChecked);
    } catch (error: any) {
      message.error(labels.ffmpegReleaseCheckFailed + (error?.message || error));
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
      message.error(labels.ffmpegDownloadFailed + (error?.message || error));
    } finally {
      setDownloadingFfmpeg(false);
    }
  };

  const openFfmpegRepo = async () => openUrl(FFMPEG_RELEASE_URL);
  const openFfmpegDir = async () => openContainingDir(ffmpegPath);
  const openVideoDir = async () => { if (defaultVideoDir) await invoke("open_path_in_file_manager", { path: defaultVideoDir }); };

  return {
    ffmpegPath,
    setFfmpegPath,
    defaultVideoDir,
    ffmpegRelease,
    ffmpegProgress,
    recordingInfo,
    checkingFfmpeg,
    checkingRecordingInfo,
    downloadingFfmpeg,
    saveFfmpegPath,
    chooseFfmpegPath,
    checkFfmpegRelease,
    checkRecordingInfo,
    downloadFfmpegRelease,
    openFfmpegRepo,
    openFfmpegDir,
    openVideoDir,
  };
}
