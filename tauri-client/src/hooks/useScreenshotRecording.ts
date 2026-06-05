import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "antd";
import type { Rect } from "../types/screenshot";
import { getPhysicalSelection } from "../utils/screenshotImage";
import { openRecordingWindows } from "../utils/recordingWindows";

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
  imageRef: React.MutableRefObject<HTMLImageElement | null>;
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
  screenshotModeRef,
  triggerRender,
  setCurrentRect,
  setSelection,
  setHoverCandidate,
  resetScreenshotState,
}: UseScreenshotRecordingProps) {
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
    const selection = getCurrentPhysicalSelection();
    const origin = await getCurrentWindow().outerPosition().catch(() => ({ x: 0, y: 0 }));
    return {
      id: "region",
      title: "翻译结果",
      x: Math.round(origin.x + selection.x),
      y: Math.round(origin.y + selection.y),
      w: Math.round(selection.w),
      h: Math.round(selection.h),
    };
  };

  const rectFromAbsoluteTarget = async (target: RecordingTarget) => {
    const canvas = canvasRef.current;
    const image = imageRef.current;
    const origin = await getCurrentWindow().outerPosition().catch(() => ({ x: 0, y: 0 }));
    const scaleX = image && canvas ? image.naturalWidth / canvas.width : 1;
    const scaleY = image && canvas ? image.naturalHeight / canvas.height : 1;
    return {
      x: Math.round((target.x - origin.x) / scaleX),
      y: Math.round((target.y - origin.y) / scaleY),
      w: Math.round(target.w / scaleX),
      h: Math.round(target.h / scaleY),
      kind: "window",
    } as Rect;
  };

  const isLikelySystemAudioDevice = (device: string) => /wasapi:|stereo mix|立体声|混音|loopback|virtual audio|output|speaker|扬声器/i.test(device);
  const isLikelyMicrophoneDevice = (device: string) => !isLikelySystemAudioDevice(device);

  const formatAudioDeviceLabel = (device: string) => {
    if (device === "wasapi:default") return "系统声音（默认输出）";
    if (device.startsWith("wasapi:")) return `系统声音：${device.slice("wasapi:".length)}`;
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
      throw new Error("未找到 ffmpeg.exe，请先在模型/视频配置里下载或选择 FFmpeg。");
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
          message.info("Please select a recording area first");
          triggerRender();
          return;
        }
      } else if (mode === "window") {
        const target = targets.windows.find((item) => item.id === selectedWindowTargetId) || targets.windows[0];
        if (!target) throw new Error("No recordable window detected");
        setSelectedWindowTargetId(target.id);
        await applyRecordingTarget(target);
        setRecordingPickerMode("window");
        message.info("请选择要录制的窗口，蓝框确认无误后点击确认。");
        return;
      } else {
        const target = targets.displays.find((item) => item.id === selectedDisplayTargetId) || targets.displays[0];
        if (!target) throw new Error("No display detected");
        setSelectedDisplayTargetId(target.id);
        await applyRecordingTarget(target);
        setRecordingPickerMode("display");
        message.info("请选择要录制的显示器，蓝框确认无误后点击确认。");
        return;
      }

      await startRecording();
    } catch (error: any) {
      setRecordingStatus("idle");
      message.error(`Failed to enter recording mode: ${error?.message || error}`);
    }
  };

  const cancelRecordingTargetPicker = () => {
    setRecordingPickerMode(null);
    recordingRegionRef.current = null;
    setRecordingMode("region");
    message.destroy();
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
    if ((recordingAudioMode === "system" || recordingAudioMode === "system_mic") && !devices.system) throw new Error("当前未检测到系统声音设备");
    if ((recordingAudioMode === "mic" || recordingAudioMode === "system_mic") && !devices.mic) throw new Error("当前未检测到麦克风设备");
    const region = recordingModeRef.current === "region" ? await getCurrentAbsoluteSelection() : recordingRegionRef.current;
    if (!region || region.w <= 0 || region.h <= 0) throw new Error("请先选择有效录制区域");
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

  const startRecording = async () => {
    if (isRecordingBusyRef.current) return;
    try {
      const active = await invoke<boolean>('is_recording_active').catch(() => false);
      if (active) {
        message.error('当前已有录像正在进行，请先停止');
        return;
      }
      setIsRecordingBusy(true);
      const options = await buildRecordingOptions();
      const normalizedOptions = { ...options, fps: 30, resolution: "1080p", output_dir: null };
      const region = { x: normalizedOptions.region_x, y: normalizedOptions.region_y, w: normalizedOptions.region_w, h: normalizedOptions.region_h };
      setRecordingPickerMode(null);
      setRecordingStatus("ready");
      console.log("[screenshot-trace] startRecording: openRecordingWindows before, recordingStatus=", recordingStatusRef.current, "shouldCloseScreenshot=", true);
      await openRecordingWindows({
        options: normalizedOptions,
        countdownSeconds: 0,
        autoStart: false,
      }, region);
      console.log("[screenshot-trace] startRecording: openRecordingWindows after");
      const win = getCurrentWindow();
      console.log("[screenshot-trace] startRecording: closing screenshot window before");
      await win.setAlwaysOnTop(false).catch(() => {});
      await win.hide().catch(() => {});
      console.log("[screenshot-trace] startRecording: closing screenshot window after");
      await invoke('set_capturing_state', { state: false }).catch(() => {});
    } catch (error: any) {
      setRecordingStatus("idle");
      message.error("Failed to open recording controls: " + (error?.message || error));
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
      if (segments.length === 0) throw new Error("没有可保存的录屏片段");
      const win = getCurrentWindow();
      await win.setAlwaysOnTop(false).catch(() => {});
      await win.hide().catch(() => {});
      await invoke('set_capturing_state', { state: false }).catch(() => {});
      const savedPath = await invoke<string>("concat_recording_segments", { segmentPaths: segments });
      await invoke("cleanup_recording_files", { paths: segments }).catch(() => {});
      recordingSegmentsRef.current = [];
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);
      setRecordingStatus("idle");
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
      message.success(`录屏已保存：${savedPath}`);
      triggerRender();
    } catch (error: any) {
      message.error("完成录屏失败：" + (error?.message || error));
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
      message.info("已取消录屏并清理临时片段");
    } finally {
      recordingSegmentsRef.current = [];
      setRecordingStartedAt(null);
      setRecordingElapsedMs(0);
      setRecordingStatus("idle");
      setIsRecordingBusy(false);
      resetScreenshotState();
      await invoke("cancel_screenshot", { label: getCurrentWindow().label }).catch(() => {});
      triggerRender();
    }
  };

  const clearRecordingState = () => {
    recordingSegmentsRef.current = [];
    recordingRegionRef.current = null;
    setRecordingStatus("idle");
    setIsRecordingBusy(false);
    setRecordingStartedAt(null);
    setRecordingElapsedMs(0);
    setRecordingPickerMode(null);
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
