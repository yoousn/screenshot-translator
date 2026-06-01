import React, { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openPath } from "@tauri-apps/plugin-opener";
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
    await Promise.all([
      withTimeout(invoke("hide_recording_overlay").catch(() => {}), 150),
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
    if (busyRef.current || (statusRef.current !== "ready" && statusRef.current !== "saved")) return;
    setSavedPath(null);
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
      await startSegment();
    } catch (error: any) {
      message.error(`Failed to start recording: ${error?.message || error}`);
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
      setOverlayStatus("saved");
      await invoke("hide_recording_overlay").catch(() => {});
      await emit("recording-ended").catch(() => {});
      message.success(`Recording saved: ${nextSavedPath}`);
    } catch (error: any) {
      if (cancelledRef.current) return;
      setOverlayStatus(activeStartedAtRef.current === null ? "paused" : "recording");
      message.error(`Failed to save recording: ${error?.message || error}`);
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
      message.error(`Failed to pause recording: ${error?.message || error}`);
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
      message.error(`Failed to resume recording: ${error?.message || error}`);
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
    const folder = outputDir || await invoke<string>("get_default_recording_output_dir");
    setOutputDir(folder);
    await openPath(folder);
  };

  const copySavedVideo = async () => {
    if (!savedPath) return;
    try {
      await invoke("copy_file_to_clipboard", { path: savedPath });
      message.success("Video copied to clipboard");
    } catch {
      await navigator.clipboard.writeText(savedPath);
      message.info("Video path copied");
    }
  };

  const audioLabel = (() => {
    const mode = sessionRef.current?.options.audio_mode || "none";
    if (mode === "system_mic") return "System + Mic";
    if (mode === "system") return "System";
    if (mode === "mic") return "Mic";
    return "Muted";
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
