import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { App as AntdApp, ConfigProvider } from "antd";
import type { RecordingWindowPayload } from "../utils/recordingWindows";
import RecordingControlHud from "../components/recording/RecordingControlHud";

type OverlayStatus = "ready" | "countdown" | "recording" | "paused" | "saving" | "saved";

const formatRecordingTime = (ms: number) => {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(totalSeconds / 3600).toString().padStart(2, "0");
  const minutes = Math.floor((totalSeconds % 3600) / 60).toString().padStart(2, "0");
  const seconds = (totalSeconds % 60).toString().padStart(2, "0");
  return `${hours}:${minutes}:${seconds}`;
};

const withTimeout = async <T,>(task: Promise<T>, ms: number): Promise<T | null> => {
  let timeoutId: number | undefined;
  try {
    return await Promise.race([
      task,
      new Promise<null>((resolve) => {
        timeoutId = window.setTimeout(() => resolve(null), ms);
      }),
    ]);
  } finally {
    if (timeoutId !== undefined) window.clearTimeout(timeoutId);
  }
};

const closeWindowIfExists = async (label: string) => {
  const win = await WebviewWindow.getByLabel(label).catch(() => null);
  if (!win) return;
  await win.destroy().catch(() => win.close().catch(() => {}));
};

const setWindowCaptureExcludedIfExists = async (label: string, excluded: boolean) => {
  await invoke("set_window_capture_excluded", { label, excluded }).catch(() => {});
};

const setRecordingCaptureShield = async (excluded: boolean) => {
  await Promise.all([
    setWindowCaptureExcludedIfExists("main", excluded),
    setWindowCaptureExcludedIfExists("screenshot", excluded),
    setWindowCaptureExcludedIfExists("recording_control", excluded),
    setWindowCaptureExcludedIfExists("recording_notice", excluded),
  ]);
};

function RecordingControlContent() {
  const winRef = useRef(getCurrentWindow());
  const allowCloseRef = useRef(false);
  const sessionRef = useRef<RecordingWindowPayload | null>(null);
  const segmentsRef = useRef<string[]>([]);
  const activeStartedAtRef = useRef<number | null>(null);
  const accumulatedMsRef = useRef(0);
  const cancelledRef = useRef(false);
  const sessionStartedRef = useRef(false);
  const [status, setStatus] = useState<OverlayStatus>("ready");
  const [elapsedMs, setElapsedMs] = useState(0);
  const [countdown, setCountdown] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const [savedPath, setSavedPath] = useState<string | null>(null);
  const [outputDir, setOutputDir] = useState<string | null>(null);
  const statusRef = useRef<OverlayStatus>("ready");
  const busyRef = useRef(false);
  const { message } = AntdApp.useApp();

  const setOverlayStatus = (nextStatus: OverlayStatus) => {
    statusRef.current = nextStatus;
    setStatus(nextStatus);
    invoke("set_recording_overlay_status", { status: nextStatus }).catch(() => {});
  };

  const setOverlayBusy = (nextBusy: boolean) => {
    busyRef.current = nextBusy;
    setBusy(nextBusy);
  };

  const dismissOverlay = async (notifyParent = true) => {
    cancelledRef.current = true;
    allowCloseRef.current = true;
    await setRecordingCaptureShield(false);
    if (sessionRef.current?.restoreMainWindow) {
      const main = await WebviewWindow.getByLabel("main").catch(() => null);
      await main?.show().catch(() => {});
    }
    await Promise.all([
      withTimeout(invoke("hide_recording_overlay").catch(() => {}), 150),
      withTimeout(closeWindowIfExists("recording_notice"), 150),
      notifyParent ? withTimeout(emit("recording-ended").catch(() => {}), 150) : Promise.resolve(null),
      withTimeout(winRef.current.hide().catch(() => {}), 150),
    ]);
  };

  const closeOverlay = async () => {
    await dismissOverlay(true);
    await withTimeout(winRef.current.close().catch(() => {}), 300);
  };

  const startSegment = async () => {
    const current = sessionRef.current;
    if (!current) throw new Error("Recording session is not ready");
    const path = await invoke<string>("start_recording", { options: current.options });
    if (cancelledRef.current) {
      await withTimeout(invoke("cancel_recording_process").catch(() => {}), 800);
      await invoke("cleanup_recording_files", { paths: [path] }).catch(() => {});
      return;
    }
    segmentsRef.current = [...segmentsRef.current, path];
    activeStartedAtRef.current = Date.now();
    setOverlayStatus("recording");
  };

  const stopActiveSegment = async (fastCancel = false) => {
    if (activeStartedAtRef.current !== null) {
      accumulatedMsRef.current += Date.now() - activeStartedAtRef.current;
      activeStartedAtRef.current = null;
      setElapsedMs(accumulatedMsRef.current);
    }
    const command = fastCancel ? "cancel_recording_process" : "stop_recording";
    await withTimeout(invoke(command).catch(() => {}), fastCancel ? 800 : 1100);
  };

  const startRecording = async () => {
    if (busyRef.current || statusRef.current !== "ready") return;
    setSavedPath(null);
    segmentsRef.current = [];
    activeStartedAtRef.current = null;
    accumulatedMsRef.current = 0;
    cancelledRef.current = false;
    setElapsedMs(0);
    await closeWindowIfExists("recording_notice").catch(() => {});
    setOverlayBusy(true);
    try {
      const seconds = Math.max(0, Math.floor(sessionRef.current?.countdownSeconds || 0));
      if (seconds > 0) {
        setOverlayStatus("countdown");
        for (let value = seconds; value > 0; value -= 1) {
          setCountdown(value);
          await new Promise((resolve) => window.setTimeout(resolve, 1000));
          if (cancelledRef.current) return;
        }
      }
      setCountdown(null);
      await setRecordingCaptureShield(true);
      await startSegment();
    } catch (error: any) {
      message.error(`启动录制失败：${error?.message || error}`);
      setOverlayStatus("ready");
    } finally {
      setOverlayBusy(false);
    }
  };

  const finishRecording = async () => {
    if (busyRef.current || statusRef.current === "countdown" || statusRef.current === "ready" || statusRef.current === "saved") return;
    setOverlayBusy(true);
    setOverlayStatus("saving");
    try {
      if (activeStartedAtRef.current !== null) await stopActiveSegment();
      if (cancelledRef.current) return;
      const segments = [...segmentsRef.current];
      if (segments.length === 0) throw new Error("No recording segment to save");
      const nextSavedPath = await invoke<string>("concat_recording_segments", { segmentPaths: segments });
      if (cancelledRef.current) return;
      await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      segmentsRef.current = [];
      setSavedPath(nextSavedPath);
      setOverlayStatus("ready");
      const noticeShown = await showSavedNotice();
      if (!noticeShown) {
        message.success(`录制已保存：${nextSavedPath}`);
      }
    } catch (error: any) {
      if (cancelledRef.current) return;
      setOverlayStatus(activeStartedAtRef.current === null ? "paused" : "recording");
      message.error(`保存录制失败：${error?.message || error}`);
    } finally {
      setOverlayBusy(false);
    }
  };

  const toggleRecord = async () => {
    if (statusRef.current === "recording" || statusRef.current === "paused") {
      await finishRecording();
      return;
    }
    await startRecording();
  };

  const pauseRecording = async () => {
    if (busyRef.current || statusRef.current !== "recording") return;
    setOverlayBusy(true);
    setOverlayStatus("paused");
    try {
      await stopActiveSegment();
    } catch (error: any) {
      setOverlayStatus("recording");
      message.error(`暂停录制失败：${error?.message || error}`);
    } finally {
      setOverlayBusy(false);
    }
  };

  const resumeRecording = async () => {
    if (busyRef.current || statusRef.current !== "paused") return;
    setOverlayBusy(true);
    try {
      await startSegment();
    } catch (error: any) {
      message.error(`继续录制失败：${error?.message || error}`);
    } finally {
      setOverlayBusy(false);
    }
  };

  const cancelRecording = async () => {
    if (statusRef.current === "saved") {
      await closeOverlay();
      return;
    }
    if (cancelledRef.current) return;
    cancelledRef.current = true;
    setOverlayBusy(true);
    setOverlayStatus("saving");
    try {
      await dismissOverlay(true);
      await stopActiveSegment(true);
      const segments = [...segmentsRef.current];
      if (segments.length > 0) await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      segmentsRef.current = [];
      await withTimeout(winRef.current.close().catch(() => {}), 300);
    } finally {
      setOverlayBusy(false);
    }
  };

  const showSavedNotice = async () => {
    const rect = sessionRef.current?.noticeRect;
    if (!rect) return false;
    const noticeSize = { w: 340, h: 52 };
    const screenInfo = window.screen as Screen & { availLeft?: number; availTop?: number };
    const screenLeft = screenInfo.availLeft || 0;
    const screenTop = screenInfo.availTop || 0;
    const screenRight = screenLeft + screenInfo.availWidth;
    const screenBottom = screenTop + screenInfo.availHeight;
    const centeredX = rect.x + Math.round((rect.w - noticeSize.w) / 2);
    const centeredY = rect.y + Math.round((rect.h - noticeSize.h) / 2);
    const x = Math.min(Math.max(screenLeft + 8, centeredX), Math.max(screenLeft + 8, screenRight - noticeSize.w - 8));
    const y = Math.min(Math.max(screenTop + 8, centeredY), Math.max(screenTop + 8, screenBottom - noticeSize.h - 8));

    try {
      await closeWindowIfExists("recording_notice");
      const notice = new WebviewWindow("recording_notice", {
        url: `index.html?text=${encodeURIComponent("录制已保存")}`,
        title: "Recording Saved",
        decorations: false,
        transparent: true,
        alwaysOnTop: true,
        skipTaskbar: true,
        resizable: false,
        minimizable: false,
        maximizable: false,
        shadow: false,
        width: noticeSize.w,
        height: noticeSize.h,
        x,
        y,
      });
      notice.once("tauri://created", () => {
        invoke("set_window_capture_excluded", { label: "recording_notice", excluded: true }).catch(() => {});
      });
      return true;
    } catch {
      return false;
    }
  };

  useEffect(() => {
    let unlistenSession: (() => void) | null = null;
    let unlistenClose: (() => void) | null = null;
    listen<RecordingWindowPayload>("recording-overlay-session", (event) => {
      if (sessionStartedRef.current) return;
      sessionStartedRef.current = true;
      cancelledRef.current = false;
      sessionRef.current = event.payload;
      segmentsRef.current = [];
      activeStartedAtRef.current = null;
      accumulatedMsRef.current = 0;
      setElapsedMs(0);
      setSavedPath(null);
      setOverlayStatus("ready");
      if (event.payload.autoStart) window.setTimeout(() => startRecording(), 0);
    }).then((unsub) => {
      unlistenSession = unsub;
      emit("recording-overlay-ready").catch(() => {});
    });

    invoke<string>("get_default_recording_output_dir")
      .then(setOutputDir)
      .catch(() => {});

    winRef.current.onCloseRequested((event) => {
      if (allowCloseRef.current) return;
      event.preventDefault();
      cancelRecording();
    }).then((unsub) => { unlistenClose = unsub; });

    return () => {
      unlistenSession?.();
      unlistenClose?.();
    };
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => {
      if (activeStartedAtRef.current !== null) {
        setElapsedMs(accumulatedMsRef.current + Date.now() - activeStartedAtRef.current);
      }
    }, 250);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && (event.key === "s" || event.key === "S")) {
        event.preventDefault();
        toggleRecord();
      } else if (event.key === "Escape") {
        event.preventDefault();
        cancelRecording();
      } else if (event.code === "Space") {
        event.preventDefault();
        statusRef.current === "recording" ? pauseRecording() : statusRef.current === "paused" ? resumeRecording() : startRecording();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const openVideoFolder = async () => {
    try {
      const folder = savedPath
        ? savedPath.replace(/[\\/][^\\/]+$/, "")
        : outputDir || await invoke<string>("get_default_recording_output_dir");
      setOutputDir(folder);
      await invoke("open_path_in_file_manager", { path: folder });
    } catch (error: any) {
      message.error(`打开视频目录失败：${error?.message || error}`);
    }
  };

  const copySavedVideo = async () => {
    if (!savedPath) return;
    try {
      await invoke("copy_file_to_clipboard", { path: savedPath });
      message.success("视频文件已复制到剪贴板");
    } catch {
      await navigator.clipboard.writeText(savedPath);
      message.info("视频路径已复制");
    }
  };

  const audioLabel = (() => {
    const mode = sessionRef.current?.options.audio_mode || "none";
    if (mode === "system_mic") return "系统 + 麦克风";
    if (mode === "system") return "系统声音";
    if (mode === "mic") return "麦克风";
    return "静音";
  })();

  return (
    <RecordingControlHud
      status={status}
      elapsedText={formatRecordingTime(elapsedMs)}
      countdown={countdown}
      busy={busy}
      hasSavedVideo={Boolean(savedPath)}
      audioLabel={audioLabel}
      onToggleRecord={toggleRecord}
      onPause={pauseRecording}
      onResume={resumeRecording}
      onOpenFolder={openVideoFolder}
      onCopy={copySavedVideo}
      onCancel={cancelRecording}
    />
  );
}

export default function RecordingControlPage() {
  return (
    <ConfigProvider theme={{ token: { borderRadius: 12, colorPrimary: "#2563eb" } }}>
      <AntdApp>
        <RecordingControlContent />
      </AntdApp>
    </ConfigProvider>
  );
}
