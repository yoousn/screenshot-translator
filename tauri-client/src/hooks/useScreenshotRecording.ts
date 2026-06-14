import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { App as AntdApp } from "antd";
import { useI18n } from "../i18n";
import type { Rect, ScreenshotPhysicalBounds } from "../types/screenshot";
import { getDesktopPhysicalSelection, getPhysicalSelection } from "../utils/screenshotImage";
import { closeRecordingBorderWindows, openRecordingWindows } from "../utils/recordingWindows";
import { traceLog } from "../utils/debugLog";

export type RecordingStatus = "idle" | "ready" | "recording";
export type RecordingMode = "region" | "window" | "display";

export type RecordingInfo = {
  ffmpegFound: boolean;
  ffmpegPath?: string;
  isRecording: boolean;
  audioDevices?: string[];
};

export type RecordingTarget = {
  id: string;
  title: string;
  exeName?: string;
  processPath?: string;
  iconDataUrl?: string | null;
  x: number;
  y: number;
  w: number;
  h: number;
};

export type RecordingTargets = {
  windows: RecordingTarget[];
  displays: RecordingTarget[];
};

interface UseScreenshotRecordingProps {
  rectRef: React.MutableRefObject<Rect>;
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
  imageRef: React.MutableRefObject<(HTMLImageElement | HTMLCanvasElement) | null>;
  displayedPhysicalBoundsRef: React.MutableRefObject<ScreenshotPhysicalBounds | null>;
  screenshotModeRef: React.MutableRefObject<string>;
  triggerRender: () => void;
  setCurrentRect: (next: Rect, syncState?: boolean) => void;
  setSelection: (selected: boolean) => void;
  setHoverCandidate: (candidate: Rect | null) => void;
  resetScreenshotState: () => void;
}

export function useScreenshotRecording({
  rectRef,
  canvasRef,
  imageRef,
  displayedPhysicalBoundsRef,
  screenshotModeRef,
  triggerRender,
  setCurrentRect,
  setSelection,
  setHoverCandidate,
  resetScreenshotState,
}: UseScreenshotRecordingProps) {
  const { message } = AntdApp.useApp();
  const { text } = useI18n();
  const labels = text.config;
  const t = (key: string, replacements: Record<string, string> = {}) => {
    let value = labels[key] || key;
    for (const [name, replacement] of Object.entries(replacements)) {
      value = value.replace(`{${name}}`, replacement);
    }
    return value;
  };

  const [recordingStatus, setRecordingStatusState] = useState<RecordingStatus>("idle");
  const [recordingPickerMode, setRecordingPickerModeState] = useState<"window" | "display" | null>(null);
  const [recordingFps, setRecordingFps] = useState(30);
  const [recordingResolution, setRecordingResolution] = useState("1080p");
  const [recordingAudioMode, setRecordingAudioMode] = useState("none");
  const [recordingMode, setRecordingModeState] = useState<RecordingMode>("region");
  const [recordingTargets, setRecordingTargets] = useState<RecordingTargets>({ windows: [], displays: [] });
  const [selectedWindowTargetId, setSelectedWindowTargetId] = useState<string | null>(null);
  const [selectedDisplayTargetId, setSelectedDisplayTargetId] = useState<string | null>(null);
  const [recordingInfo, setRecordingInfo] = useState<RecordingInfo | null>(null);
  const [isRecordingBusy, setIsRecordingBusyState] = useState(false);
  const [recordingStartedAt, setRecordingStartedAtState] = useState<number | null>(null);
  const [recordingElapsedMs, setRecordingElapsedMs] = useState(0);

  const recordingStatusRef = useRef<RecordingStatus>("idle");
  const recordingPickerModeRef = useRef<"window" | "display" | null>(null);
  const recordingModeRef = useRef<RecordingMode>("region");
  const recordingRegionRef = useRef<RecordingTarget | null>(null);
  const isRecordingBusyRef = useRef(false);
  const recordingStartedAtRef = useRef<number | null>(null);
  const recordingSegmentsRef = useRef<string[]>([]);

  const setRecordingStatus = (status: RecordingStatus) => {
    recordingStatusRef.current = status;
    setRecordingStatusState(status);
  };

  const setRecordingPickerMode = (mode: "window" | "display" | null) => {
    recordingPickerModeRef.current = mode;
    setRecordingPickerModeState(mode);
  };

  const setRecordingMode = (mode: RecordingMode) => {
    recordingModeRef.current = mode;
    setRecordingModeState(mode);
  };

  const setIsRecordingBusy = (busy: boolean) => {
    isRecordingBusyRef.current = busy;
    setIsRecordingBusyState(busy);
  };

  const setRecordingStartedAt = (startedAt: number | null) => {
    recordingStartedAtRef.current = startedAt;
    setRecordingStartedAtState(startedAt);
  };

  useEffect(() => {
    if (recordingStatus !== "recording" || !recordingStartedAt) return;
    const updateElapsed = () => setRecordingElapsedMs(Date.now() - recordingStartedAt);
    updateElapsed();
    const timer = window.setInterval(updateElapsed, 500);
    return () => window.clearInterval(timer);
  }, [recordingStatus, recordingStartedAt]);

  const getCurrentPhysicalSelection = () => getPhysicalSelection({
    canvas: canvasRef.current,
    image: imageRef.current as any,
    rect: rectRef.current,
  });

  const getCurrentAbsoluteSelection = async (): Promise<RecordingTarget> => {
    const physicalBounds = displayedPhysicalBoundsRef.current;
    if (physicalBounds) {
      const selection = getDesktopPhysicalSelection({
        canvas: canvasRef.current,
        image: imageRef.current as any,
        rect: rectRef.current,
        physicalBounds,
      });
      return {
        id: "region",
        title: t("recordingRegionTitle"),
        x: Math.round(selection.x),
        y: Math.round(selection.y),
        w: Math.round(selection.width),
        h: Math.round(selection.height),
      };
    }

    const selection = getCurrentPhysicalSelection();
    const origin = await getCurrentWindow().outerPosition().catch(() => ({ x: 0, y: 0 }));
    return {
      id: "region",
      title: t("recordingRegionTitle"),
      x: Math.round(origin.x + selection.x),
      y: Math.round(origin.y + selection.y),
      w: Math.round(selection.w),
      h: Math.round(selection.h),
    };
  };

  const rectFromAbsoluteTarget = async (target: RecordingTarget) => {
    const canvas = canvasRef.current;
    const image = imageRef.current;
    const physicalBounds = displayedPhysicalBoundsRef.current;
    if (canvas && physicalBounds) {
      const scaleX = canvas.width / Math.max(1, physicalBounds.width);
      const scaleY = canvas.height / Math.max(1, physicalBounds.height);
      return {
        x: Math.round((target.x - physicalBounds.x) * scaleX),
        y: Math.round((target.y - physicalBounds.y) * scaleY),
        w: Math.round(target.w * scaleX),
        h: Math.round(target.h * scaleY),
        kind: "window",
      } as Rect;
    }

    const origin = await getCurrentWindow().outerPosition().catch(() => ({ x: 0, y: 0 }));
    const imageWidth = image instanceof HTMLImageElement ? image.naturalWidth : image?.width;
    const imageHeight = image instanceof HTMLImageElement ? image.naturalHeight : image?.height;
    const scaleX = imageWidth && canvas ? imageWidth / canvas.width : 1;
    const scaleY = imageHeight && canvas ? imageHeight / canvas.height : 1;
    return {
      x: Math.round((target.x - origin.x) / scaleX),
      y: Math.round((target.y - origin.y) / scaleY),
      w: Math.round(target.w / scaleX),
      h: Math.round(target.h / scaleY),
      kind: "window",
    } as Rect;
  };

  const isLikelySystemAudioDevice = (device: string) =>
    /wasapi:|stereo mix|立体声|混音|loopback|virtual audio|output|speaker|扬声器/i.test(device);
  const isLikelyMicrophoneDevice = (device: string) => !isLikelySystemAudioDevice(device);

  const formatAudioDeviceLabel = (device: string) => {
    if (device === "wasapi:default") return t("recordingSystemAudioDefault");
    if (device.startsWith("wasapi:")) {
      return t("recordingSystemAudioPrefix", { device: device.slice("wasapi:".length) });
    }
    if (device.startsWith("dshow:")) return device.slice("dshow:".length);
    return device;
  };

  const getRecordingDevices = () => {
    const devices = recordingInfo?.audioDevices || [];
    return {
      mic: devices.find(isLikelyMicrophoneDevice) || null,
      system: devices.find(isLikelySystemAudioDevice) || null,
      hasSystem: devices.some(isLikelySystemAudioDevice),
    };
  };

  const loadRecordingPrerequisites = async () => {
    const [info, targets] = await Promise.all([
      invoke<RecordingInfo>("get_recording_info"),
      invoke<RecordingTargets>("get_recording_targets").catch(() => ({ windows: [], displays: [] })),
    ]);
    setRecordingInfo(info);
    setRecordingTargets(targets);
    if (!selectedWindowTargetId && targets.windows.length > 0) setSelectedWindowTargetId(targets.windows[0].id);
    if (!selectedDisplayTargetId && targets.displays.length > 0) setSelectedDisplayTargetId(targets.displays[0].id);
    if (!info.ffmpegFound) {
      throw new Error(t("recordingFfmpegMissing"));
    }
    return { info, targets };
  };

  const applyRecordingTarget = async (target: RecordingTarget) => {
    recordingRegionRef.current = target;
    const nextRect = await rectFromAbsoluteTarget(target);
    setCurrentRect(nextRect, true);
    setSelection(true);
    setHoverCandidate(null);
    triggerRender();
  };

  const enterRecordingMode = async (mode: RecordingMode = "region") => {
    if (isRecordingBusyRef.current) return;
    try {
      setRecordingMode(mode);
      const { targets } = await loadRecordingPrerequisites();
      recordingSegmentsRef.current = [];
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);

      if (mode === "region") {
        if (!rectRef.current.w || !rectRef.current.h) {
          if (screenshotModeRef.current !== "record") {
            screenshotModeRef.current = "record";
            setCurrentRect({ x: 0, y: 0, w: 0, h: 0 }, true);
            setSelection(false);
          }
          message.info(t("recordingSelectAreaFirst"));
          triggerRender();
          return;
        }
      } else if (mode === "window") {
        const target = targets.windows.find((item) => item.id === selectedWindowTargetId) || targets.windows[0];
        if (!target) throw new Error(t("recordingNoWindow"));
        setSelectedWindowTargetId(target.id);
        await applyRecordingTarget(target);
        setRecordingPickerMode("window");
        message.info(t("recordingPickWindow"));
        return;
      } else {
        const target = targets.displays.find((item) => item.id === selectedDisplayTargetId) || targets.displays[0];
        if (!target) throw new Error(t("recordingNoDisplay"));
        setSelectedDisplayTargetId(target.id);
        await applyRecordingTarget(target);
        setRecordingPickerMode("display");
        message.info(t("recordingPickDisplay"));
        return;
      }

      await startRecording();
    } catch (error: any) {
      setRecordingStatus("idle");
      message.error(t("recordingEnterFailed", { error: String(error?.message || error) }));
    }
  };

  const cancelRecordingTargetPicker = () => {
    setRecordingPickerMode(null);
    recordingRegionRef.current = null;
    setRecordingMode("region");
    message.destroy("recording");
    if (!screenshotModeRef.current || screenshotModeRef.current === "normal") {
      setCurrentRect({ x: 0, y: 0, w: 0, h: 0 }, true);
      setSelection(false);
    }
    triggerRender();
  };

  const confirmRecordingTargetPicker = async () => {
    if (!recordingPickerModeRef.current) return;
    setRecordingPickerMode(null);
    await startRecording();
  };

  const selectRecordingTarget = async (mode: "window" | "display", targetId: string) => {
    const list = mode === "window" ? recordingTargets.windows : recordingTargets.displays;
    const target = list.find((item) => item.id === targetId);
    if (!target) return;
    if (mode === "window") setSelectedWindowTargetId(targetId);
    if (mode === "display") setSelectedDisplayTargetId(targetId);
    await applyRecordingTarget(target);
  };

  const buildRecordingOptions = async () => {
    const devices = getRecordingDevices();
    if ((recordingAudioMode === "system" || recordingAudioMode === "system_mic") && !devices.system) {
      throw new Error(t("recordingMissingSystemAudio"));
    }
    if ((recordingAudioMode === "mic" || recordingAudioMode === "system_mic") && !devices.mic) {
      throw new Error(t("recordingMissingMicrophone"));
    }
    const region = recordingModeRef.current === "region" ? await getCurrentAbsoluteSelection() : recordingRegionRef.current;
    if (!region || region.w <= 0 || region.h <= 0) throw new Error(t("recordingSelectValidRegion"));
    return {
      fps: recordingFps,
      resolution: recordingResolution,
      audio_mode: recordingAudioMode,
      mic_device: devices.mic,
      system_audio_device: devices.system,
      output_dir: null,
      region_x: Math.round(region.x),
      region_y: Math.round(region.y),
      region_w: Math.round(region.w),
      region_h: Math.round(region.h),
    };
  };

  const clampRecordingFps = (value: number | null | undefined) => {
    if (!Number.isFinite(value)) return 30;
    return Math.min(60, Math.max(10, Math.round(Number(value))));
  };

  const startRecording = async () => {
    if (isRecordingBusyRef.current) return;
    try {
      const info = await invoke("get_recording_info").catch(() => null) as { isRecording?: boolean } | null;
      const active = !!info?.isRecording;
      if (active) {
        message.error(t("recordingAlreadyActive"));
        return;
      }
      setIsRecordingBusy(true);
      const options = await buildRecordingOptions();
      const normalizedOptions = {
        ...options,
        fps: clampRecordingFps(options.fps),
        resolution: options.resolution || "1080p",
        output_dir: options.output_dir ?? null,
      };
      const region = { x: normalizedOptions.region_x, y: normalizedOptions.region_y, w: normalizedOptions.region_w, h: normalizedOptions.region_h };
      setRecordingPickerMode(null);
      setRecordingStatus("ready");
      traceLog("[screenshot-trace] startRecording: openRecordingWindows before", {
        recordingStatus: recordingStatusRef.current,
        shouldCloseScreenshot: true,
      });
      await openRecordingWindows({
        options: normalizedOptions,
        countdownSeconds: 0,
        autoStart: false,
      }, region);
      traceLog("[screenshot-trace] startRecording: openRecordingWindows after");
      const win = getCurrentWindow();
      traceLog("[screenshot-trace] startRecording: closing screenshot window before");
      await win.setAlwaysOnTop(false).catch(() => {});
      await win.hide().catch(() => {});
      traceLog("[screenshot-trace] startRecording: closing screenshot window after");
      await invoke("set_capturing_state", { state: false }).catch(() => {});
    } catch (error: any) {
      setRecordingStatus("idle");
      message.error(t("recordingOpenControlsFailed", { error: String(error?.message || error) }));
    } finally {
      setIsRecordingBusy(false);
    }
  };

  const finishRecording = async () => {
    if (recordingStatusRef.current === "ready") {
      await startRecording();
      return;
    }
    if (isRecordingBusyRef.current || recordingStatusRef.current === "idle") return;
    try {
      setIsRecordingBusy(true);
      if (recordingStatusRef.current === "recording") await invoke("stop_recording");
      const segments = [...recordingSegmentsRef.current];
      if (segments.length === 0) throw new Error(t("recordingNoSegments"));
      const win = getCurrentWindow();
      await win.setAlwaysOnTop(false).catch(() => {});
      await win.hide().catch(() => {});
      await invoke("set_capturing_state", { state: false }).catch(() => {});
      const savedPath = await invoke<string>("concat_recording_segments", { segmentPaths: segments });
      await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      recordingSegmentsRef.current = [];
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);
      setRecordingStatus("idle");
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
      message.success(t("recordingSaved", { path: savedPath }));
      triggerRender();
      setTimeout(() => {
        closeRecordingBorderWindows([], { source: "recording-finish", hideMain: true }).catch(console.error);
      }, 100);
    } catch (error: any) {
      message.error(t("recordingFinishFailed", { error: String(error?.message || error) }));
      setRecordingStatus("recording");
      await getCurrentWindow().show().catch(() => {});
    } finally {
      setIsRecordingBusy(false);
    }
  };

  const cancelRecording = async () => {
    if (recordingStatusRef.current === "idle" && recordingSegmentsRef.current.length === 0) return;
    const segments = [...recordingSegmentsRef.current];
    try {
      setIsRecordingBusy(true);
      await invoke("cancel_recording_process").catch(() => {});
      await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      message.info(t("recordingCancelled"));
    } finally {
      recordingSegmentsRef.current = [];
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);
      setRecordingStatus("idle");
      setIsRecordingBusy(false);
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
      triggerRender();
      setTimeout(() => {
        closeRecordingBorderWindows([], { source: "recording-cancel", hideMain: true }).catch(console.error);
      }, 100);
    }
  };

  const clearRecordingState = () => {
    const hadRecordingActivity =
      recordingStatusRef.current !== "idle" ||
      recordingSegmentsRef.current.length > 0 ||
      recordingPickerModeRef.current !== null ||
      recordingStartedAtRef.current !== null ||
      isRecordingBusyRef.current ||
      recordingRegionRef.current !== null;

    recordingSegmentsRef.current = [];
    recordingRegionRef.current = null;
    setRecordingStatus("idle");
    setIsRecordingBusy(false);
    setRecordingStartedAt(null);
    setRecordingElapsedMs(0);
    setRecordingPickerMode(null);
    if (hadRecordingActivity) {
      setTimeout(() => {
        closeRecordingBorderWindows([], { source: "clearRecordingState-active", hideMain: true }).catch(console.error);
      }, 100);
    }
  };

  return {
    recordingStatus,
    recordingPickerMode,
    recordingFps,
    recordingResolution,
    recordingAudioMode,
    recordingMode,
    recordingTargets,
    selectedWindowTargetId,
    selectedDisplayTargetId,
    recordingInfo,
    isRecordingBusy,
    recordingStartedAt,
    recordingElapsedMs,
    recordingStatusRef,
    recordingPickerModeRef,
    recordingModeRef,
    isRecordingBusyRef,
    recordingStartedAtRef,
    recordingSegmentsRef,
    setRecordingFps,
    setRecordingResolution,
    setRecordingAudioMode,
    setRecordingStatus,
    setRecordingPickerMode,
    setRecordingMode,
    enterRecordingMode,
    cancelRecordingTargetPicker,
    confirmRecordingTargetPicker,
    selectRecordingTarget,
    startRecording,
    finishRecording,
    cancelRecording,
    clearRecordingState,
    formatAudioDeviceLabel,
    getRecordingDevices,
    loadRecordingPrerequisites,
  };
}
