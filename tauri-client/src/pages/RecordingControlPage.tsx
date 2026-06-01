import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { App as AntdApp, ConfigProvider } from "antd";
import type { RecordingWindowPayload } from "../utils/recordingWindows";
import RecordingControlHud from "../components/recording/RecordingControlHud";

type OverlayStatus = "countdown" | "recording" | "paused" | "saving";

const formatRecordingTime = (ms: number) => {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const minutes = Math.floor(totalSeconds / 60).toString().padStart(2, "0");
  const seconds = (totalSeconds % 60).toString().padStart(2, "0");
  return `${minutes}:${seconds}`;
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

function RecordingControlContent() {
  const winRef = useRef(getCurrentWindow());
  const allowCloseRef = useRef(false);
  const sessionRef = useRef<RecordingWindowPayload | null>(null);
  const segmentsRef = useRef<string[]>([]);
  const activeStartedAtRef = useRef<number | null>(null);
  const accumulatedMsRef = useRef(0);
  const cancelledRef = useRef(false);
  const sessionStartedRef = useRef(false);
  const [status, setStatus] = useState<OverlayStatus>("countdown");
  const [elapsedMs, setElapsedMs] = useState(0);
  const [countdown, setCountdown] = useState<number | null>(null);
  const [busy, setBusy] = useState(false);
  const statusRef = useRef<OverlayStatus>("countdown");
  const busyRef = useRef(false);
  const { message } = AntdApp.useApp();

  const setOverlayStatus = (nextStatus: OverlayStatus) => {
    statusRef.current = nextStatus;
    setStatus(nextStatus);
  };

  const setOverlayBusy = (nextBusy: boolean) => {
    busyRef.current = nextBusy;
    setBusy(nextBusy);
  };

  const dismissOverlay = async () => {
    cancelledRef.current = true;
    allowCloseRef.current = true;
    await Promise.all([
      withTimeout(invoke("hide_recording_overlay").catch(() => {}), 150),
      withTimeout(emit("recording-ended").catch(() => {}), 150),
      withTimeout(winRef.current.hide().catch(() => {}), 150),
    ]);
  };

  const closeOverlay = async () => {
    await dismissOverlay();
    await withTimeout(winRef.current.close().catch(() => {}), 300);
  };

  const startSegment = async () => {
    const current = sessionRef.current;
    if (!current) return;
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

  const runCountdownAndStart = async (seconds: number) => {
    try {
      const normalized = Math.max(0, Math.floor(seconds));
      setOverlayStatus("countdown");
      if (normalized > 0) {
        for (let value = normalized; value > 0; value -= 1) {
          setCountdown(value);
          await new Promise((resolve) => window.setTimeout(resolve, 1000));
          if (cancelledRef.current) return;
        }
      }
      if (cancelledRef.current) return;
      setCountdown(null);
      await startSegment();
    } catch (error: any) {
      message.error(`启动录制失败：${error?.message || error}`);
      await closeOverlay();
    }
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

  const finishRecording = async () => {
    if (busyRef.current || statusRef.current === "countdown") return;
    setOverlayBusy(true);
    setOverlayStatus("saving");
    try {
      if (activeStartedAtRef.current !== null) await stopActiveSegment();
      if (cancelledRef.current) return;
      const segments = [...segmentsRef.current];
      if (segments.length === 0) throw new Error("没有可保存的录屏片段");
      await winRef.current.setAlwaysOnTop(false).catch(() => {});
      await winRef.current.hide().catch(() => {});
      const savedPath = await invoke<string>("concat_recording_segments", { segmentPaths: segments });
      if (cancelledRef.current) return;
      await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      segmentsRef.current = [];
      message.success(`录屏已保存：${savedPath}`);
      await closeOverlay();
    } catch (error: any) {
      if (cancelledRef.current) return;
      setOverlayStatus(activeStartedAtRef.current === null ? "paused" : "recording");
      await winRef.current.show().catch(() => {});
      await winRef.current.setAlwaysOnTop(true).catch(() => {});
      message.error(`保存录制失败：${error?.message || error}`);
    } finally {
      setOverlayBusy(false);
    }
  };

  const cancelRecording = async () => {
    if (cancelledRef.current) return;
    cancelledRef.current = true;
    setOverlayBusy(true);
    setOverlayStatus("saving");
    try {
      await dismissOverlay();
      await stopActiveSegment(true);
      const segments = [...segmentsRef.current];
      if (segments.length > 0) await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      segmentsRef.current = [];
      await withTimeout(winRef.current.close().catch(() => {}), 300);
    } finally {
      setOverlayBusy(false);
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
      runCountdownAndStart(event.payload.countdownSeconds);
    }).then((unsub) => {
      unlistenSession = unsub;
      emit("recording-overlay-ready").catch(() => {});
    });

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
        finishRecording();
      } else if (event.key === "Escape") {
        event.preventDefault();
        cancelRecording();
      } else if (event.code === "Space") {
        event.preventDefault();
        statusRef.current === "recording" ? pauseRecording() : resumeRecording();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [status, busy]);

  const isPaused = status === "paused";
  const isCounting = status === "countdown";

  return (
    <RecordingControlHud
      status={status}
      elapsedText={formatRecordingTime(elapsedMs)}
      countdown={countdown}
      busy={busy}
      onPause={pauseRecording}
      onResume={resumeRecording}
      onSave={finishRecording}
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
