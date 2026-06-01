import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openPath } from "@tauri-apps/plugin-opener";
import { Alert, App as AntdApp, Button, Card, Descriptions, Form, Input, Progress, Select, Space, Typography } from "antd";
import { ReloadOutlined, VideoCameraOutlined } from "@ant-design/icons";

const { Paragraph, Text, Title } = Typography;

type RecordingInfo = {
  ffmpegFound: boolean;
  ffmpegPath?: string | null;
  isRecording: boolean;
  audioDevices: string[];
};

type RecordingForm = {
  fps: number;
  resolution: string;
  audioMode: string;
  micDevice?: string;
  systemAudioDevice?: string;
  outputDir?: string;
  ffmpegPath?: string;
};

const defaultInfo: RecordingInfo = {
  ffmpegFound: false,
  ffmpegPath: null,
  isRecording: false,
  audioDevices: [],
};

type FfmpegReleaseInfo = {
  tag: string;
  pageUrl?: string | null;
  assetName: string;
  downloadUrl: string;
  size?: number | null;
  installDir: string;
};

type DownloadProgress = {
  phase: string;
  downloaded: number;
  total?: number | null;
  percent: number;
};

const defaultFormValues: RecordingForm = {
  fps: 30,
  resolution: "1080p",
  audioMode: "none",
  ffmpegPath: "",
  outputDir: "",
};

const formatAudioDeviceLabel = (device: string) => {
  if (device === "wasapi:default") return "WASAPI 默认输出（系统声音）";
  if (device.startsWith("wasapi:")) return `WASAPI：${device.slice("wasapi:".length)}`;
  if (device.startsWith("dshow:")) return device.slice("dshow:".length);
  return device;
};

const isLikelySystemAudioDevice = (device: string) => /wasapi:|stereo mix|立体声|混音|loopback|virtual audio|output|speaker|扬声器/i.test(device);
const isLikelyMicrophoneDevice = (device: string) => !isLikelySystemAudioDevice(device);

export default function Recording() {
  const { message } = AntdApp.useApp();
  const [form] = Form.useForm<RecordingForm>();
  const [info, setInfo] = useState<RecordingInfo>(defaultInfo);
  const [loading, setLoading] = useState(false);
  const [recordingPath, setRecordingPath] = useState("");
  const [recordingStartedAt, setRecordingStartedAt] = useState<number | null>(null);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [releaseInfo, setReleaseInfo] = useState<FfmpegReleaseInfo | null>(null);
  const [checkingRelease, setCheckingRelease] = useState(false);
  const [downloadingFfmpeg, setDownloadingFfmpeg] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);

  const audioMode = Form.useWatch("audioMode", form) || "none";
  const micDevices = useMemo(() => info.audioDevices.filter(isLikelyMicrophoneDevice), [info.audioDevices]);
  const systemAudioDevices = useMemo(() => info.audioDevices.filter(isLikelySystemAudioDevice), [info.audioDevices]);
  const micOptions = useMemo(() => micDevices.map((device) => ({ label: formatAudioDeviceLabel(device), value: device })), [micDevices]);
  const systemAudioOptions = useMemo(() => systemAudioDevices.map((device) => ({ label: formatAudioDeviceLabel(device), value: device })), [systemAudioDevices]);
  const needsMic = audioMode === "mic" || audioMode === "system_mic";
  const needsSystemAudio = audioMode === "system" || audioMode === "system_mic";
  const cannotRecordSelectedAudio = (needsMic && micOptions.length === 0) || (needsSystemAudio && systemAudioOptions.length === 0);

  useEffect(() => {
    if (needsSystemAudio && !form.getFieldValue("systemAudioDevice")) {
      const preferred = systemAudioDevices[0];
      if (preferred) form.setFieldValue("systemAudioDevice", preferred);
    }
    if (needsMic && !form.getFieldValue("micDevice")) {
      const preferred = micDevices[0];
      if (preferred) form.setFieldValue("micDevice", preferred);
    }
  }, [form, micDevices, needsMic, needsSystemAudio, systemAudioDevices]);

  const loadInfo = async () => {
    try {
      setLoading(true);
      const next = await invoke<RecordingInfo>("get_recording_info");
      setInfo(next);
    } catch (error: any) {
      message.error("读取录屏状态失败：" + (error?.message || error));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    const loadInitialConfig = async () => {
      let savedValues: Partial<RecordingForm> = {};
      try {
        const config = JSON.parse(await invoke<string>("get_config"));
        savedValues = config.recordingOptions || {};
        savedValues.ffmpegPath = config.recordingFfmpegPath || "";
      } catch {}
      form.setFieldsValue({ ...defaultFormValues, ...savedValues });
      await loadInfo();
    };
    loadInitialConfig();
  }, []);

  useEffect(() => {
    const unlisten = listen<DownloadProgress>("ffmpeg-download-progress", (event) => {
      setDownloadProgress(event.payload);
    });
    return () => {
      unlisten.then((dispose) => dispose()).catch(() => undefined);
    };
  }, []);

  useEffect(() => {
    if (!recordingStartedAt || !info.isRecording) {
      setElapsedSeconds(0);
      return;
    }
    const timer = window.setInterval(() => {
      setElapsedSeconds(Math.max(0, Math.floor((Date.now() - recordingStartedAt) / 1000)));
    }, 1000);
    return () => window.clearInterval(timer);
  }, [info.isRecording, recordingStartedAt]);

  const saveRecordingOptions = async (values: RecordingForm) => {
    try {
      const config = JSON.parse(await invoke<string>("get_config"));
      config.recordingOptions = {
        fps: values.fps,
        resolution: values.resolution,
        audioMode: values.audioMode,
        micDevice: values.micDevice || "",
        systemAudioDevice: values.systemAudioDevice || "",
        outputDir: values.outputDir || "",
      };
      config.recordingFfmpegPath = values.ffmpegPath?.trim() || "";
      await invoke("save_config", { configStr: JSON.stringify(config, null, 2) });
    } catch (error: any) {
      message.error("保存录屏配置失败：" + (error?.message || error));
    }
  };

  const startRecording = async () => {
    try {
      const values = await form.validateFields();
      if (cannotRecordSelectedAudio) {
        message.error("当前音频模式缺少可用设备，请换成静音/麦克风，或启用 Stereo Mix / 虚拟声卡后刷新。");
        return;
      }
      await saveRecordingOptions(values);
      const path = await invoke<string>("start_recording", {
        options: {
          fps: values.fps,
          resolution: values.resolution,
          audio_mode: values.audioMode,
          mic_device: values.micDevice || null,
          system_audio_device: values.systemAudioDevice || null,
          output_dir: values.outputDir || null,
        },
      });
      setRecordingPath(path);
      setRecordingStartedAt(Date.now());
      message.success("录屏已开始");
      await loadInfo();
    } catch (error: any) {
      message.error("启动录屏失败：" + (error?.message || error));
    }
  };

  const stopRecording = async () => {
    try {
      await invoke("stop_recording");
      setRecordingStartedAt(null);
      message.success("录屏已停止");
      await loadInfo();
    } catch (error: any) {
      message.error("停止录屏失败：" + (error?.message || error));
    }
  };



  const saveFfmpegPath = async (ffmpegPath: string) => {
    try {
      const config = JSON.parse(await invoke<string>("get_config"));
      config.recordingFfmpegPath = ffmpegPath.trim();
      await invoke("save_config", { configStr: JSON.stringify(config, null, 2) });
    } catch (error: any) {
      message.error("保存 ffmpeg 路径失败：" + (error?.message || error));
      throw error;
    }
  };

  const chooseFfmpeg = async () => {
    try {
      const currentPath = form.getFieldValue("ffmpegPath") || info.ffmpegPath || "";
      const file = await invoke<string | null>("choose_ffmpeg_executable", { currentPath });
      if (file) {
        form.setFieldValue("ffmpegPath", file);
        await saveFfmpegPath(file);
        message.success("ffmpeg 路径已保存");
        await loadInfo();
      }
    } catch (error: any) {
      message.error("选择 ffmpeg 失败：" + (error?.message || error));
    }
  };

  const clearFfmpeg = async () => {
    form.setFieldValue("ffmpegPath", "");
    await saveFfmpegPath("");
    await loadInfo();
  };

  const checkFfmpegRelease = async () => {
    try {
      setCheckingRelease(true);
      const next = await invoke<FfmpegReleaseInfo>("get_ffmpeg_release_info");
      setReleaseInfo(next);
      message.success(`发现官方 ffmpeg：${next.assetName}`);
    } catch (error: any) {
      message.error("检查 ffmpeg 更新失败：" + (error?.message || error));
    } finally {
      setCheckingRelease(false);
    }
  };

  const downloadFfmpeg = async () => {
    try {
      setDownloadingFfmpeg(true);
      setDownloadProgress({ phase: "准备下载", downloaded: 0, total: null, percent: 1 });
      const targetRelease = releaseInfo || await invoke<FfmpegReleaseInfo>("get_ffmpeg_release_info");
      setReleaseInfo(targetRelease);
      const result = await invoke<{ path: string; installDir: string; bytes: number }>("download_ffmpeg_release", {
        url: targetRelease.downloadUrl,
        tag: targetRelease.tag,
      });
      form.setFieldValue("ffmpegPath", result.path);
      await saveFfmpegPath(result.path);
      message.success("ffmpeg 已下载并安装");
      await loadInfo();
    } catch (error: any) {
      message.error("下载 ffmpeg 失败：" + (error?.message || error));
    } finally {
      setDownloadingFfmpeg(false);
    }
  };

  const chooseOutputDir = async () => {
    try {
      const currentDir = form.getFieldValue("outputDir") || "";
      const dir = await invoke<string | null>("choose_recording_output_dir", { currentDir });
      if (dir) form.setFieldValue("outputDir", dir);
    } catch (error: any) {
      message.error("选择输出目录失败：" + (error?.message || error));
    }
  };

  const openOutputDir = async () => {
    try {
      const dir = form.getFieldValue("outputDir");
      if (dir) await openPath(dir);
      else if (recordingPath) await openPath(recordingPath.replace(/[\\/][^\\/]+$/, ""));
      else message.info("请先选择输出目录或完成一次录屏");
    } catch (error: any) {
      message.error("打开输出目录失败：" + (error?.message || error));
    }
  };

  return (
    <Card bordered={false} style={{ borderRadius: 12, boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid #e8e8e8", paddingBottom: 16, marginBottom: 24 }}>
        <div>
          <Title level={4} style={{ margin: 0 }}>录屏</Title>
          <Paragraph type="secondary" style={{ fontSize: 12, margin: "4px 0 0 0" }}>
            支持全屏录制、30/60 帧、480P/720P/1080P/原画和基础音频设备选择。
          </Paragraph>
        </div>
        <Button icon={<ReloadOutlined />} onClick={loadInfo} loading={loading}>刷新状态</Button>
      </div>

      {!info.ffmpegFound && (
        <Alert
          type="warning"
          showIcon
          style={{ marginBottom: 16 }}
          message="未找到 ffmpeg.exe"
          description="可以点击下方选择 ffmpeg.exe；也可以将 ffmpeg.exe 放到软件同级 ffmpeg\\ffmpeg.exe。"
        />
      )}

      <Descriptions size="small" column={1} bordered style={{ marginBottom: 20 }}>
        <Descriptions.Item label="ffmpeg">{info.ffmpegFound ? info.ffmpegPath : "未找到"}</Descriptions.Item>
        <Descriptions.Item label="录制状态">{info.isRecording ? <Text type="danger">录制中</Text> : "未录制"}</Descriptions.Item>
        {info.isRecording && <Descriptions.Item label="已录制">{elapsedSeconds} 秒</Descriptions.Item>}
        <Descriptions.Item label="麦克风设备">{micOptions.length > 0 ? `${micOptions.length} 个` : "未检测到"}</Descriptions.Item>
        <Descriptions.Item label="系统声音设备">{systemAudioOptions.length > 0 ? `${systemAudioOptions.length} 个` : "未检测到"}</Descriptions.Item>
        {recordingPath && <Descriptions.Item label="最近输出">{recordingPath}</Descriptions.Item>}
      </Descriptions>
      {info.ffmpegFound && systemAudioOptions.length === 0 && (
        <Alert
          type="info"
          showIcon
          style={{ marginBottom: 16 }}
          message="未检测到系统声音设备"
          description="当前 ffmpeg 或系统没有暴露 WASAPI / Stereo Mix / 虚拟声卡回环设备；可以先用静音或麦克风录制，若要录系统声请启用 Stereo Mix 或安装虚拟声卡后刷新。"
        />
      )}

      <Form form={form} layout="vertical" disabled={info.isRecording}>
        <Form.Item label="ffmpeg.exe 路径" name="ffmpegPath">
          <Input.Group compact>
            <Form.Item name="ffmpegPath" noStyle>
              <Input style={{ width: "calc(100% - 176px)" }} placeholder="可留空自动查找软件同级 ffmpeg\\ffmpeg.exe；也可选择现有 ffmpeg.exe" />
            </Form.Item>
            <Button style={{ width: 88 }} onClick={chooseFfmpeg}>选择</Button>
            <Button style={{ width: 88 }} onClick={clearFfmpeg}>清空</Button>
          </Input.Group>
        </Form.Item>
        <Space style={{ marginTop: -8, marginBottom: 16 }} wrap>
          <Button icon={<ReloadOutlined />} loading={checkingRelease} onClick={checkFfmpegRelease}>检查官方版本</Button>
          <Button type="primary" loading={downloadingFfmpeg} onClick={downloadFfmpeg}>下载/更新 ffmpeg</Button>
          {releaseInfo && <Text type="secondary">{releaseInfo.tag} / {releaseInfo.assetName}</Text>}
        </Space>
        {downloadProgress && (downloadingFfmpeg || downloadProgress.percent < 100) && (
          <Progress percent={downloadProgress.percent} status={downloadProgress.percent >= 100 ? "success" : "active"} format={() => downloadProgress.phase} style={{ marginBottom: 16 }} />
        )}
        <Space size={16} wrap align="start">
          <Form.Item label="帧率" name="fps" rules={[{ required: true }]}>
            <Select style={{ width: 120 }} options={[{ label: "30 FPS", value: 30 }, { label: "60 FPS", value: 60 }]} />
          </Form.Item>
          <Form.Item label="分辨率" name="resolution" rules={[{ required: true }]}>
            <Select style={{ width: 140 }} options={[{ label: "480P", value: "480p" }, { label: "720P", value: "720p" }, { label: "1080P", value: "1080p" }, { label: "原画", value: "original" }]} />
          </Form.Item>
          <Form.Item label="音频" name="audioMode" rules={[{ required: true }]}>
            <Select
              style={{ width: 180 }}
              options={[
                { label: "静音", value: "none" },
                { label: "麦克风", value: "mic" },
                { label: "系统声音", value: "system" },
                { label: "系统声音 + 麦克风", value: "system_mic" },
              ]}
            />
          </Form.Item>
        </Space>

        {needsMic && (
          <Form.Item label="麦克风设备" name="micDevice" rules={[{ required: true, message: "请选择麦克风设备" }]}>
            <Select showSearch options={micOptions} placeholder="选择麦克风设备" />
          </Form.Item>
        )}

        {needsSystemAudio && (
          <Form.Item label="系统声音设备" name="systemAudioDevice" rules={[{ required: true, message: "请选择系统声音设备" }]}>
            <Select showSearch options={systemAudioOptions} placeholder="选择 WASAPI / Stereo Mix / 虚拟声卡等系统声音设备" />
          </Form.Item>
        )}

        <Form.Item label="输出目录" name="outputDir">
          <Input.Group compact>
            <Form.Item name="outputDir" noStyle>
              <Input style={{ width: "calc(100% - 176px)" }} placeholder="留空则保存到应用数据目录 recordings" />
            </Form.Item>
            <Button style={{ width: 88 }} onClick={chooseOutputDir}>选择</Button>
            <Button style={{ width: 88 }} onClick={openOutputDir}>打开</Button>
          </Input.Group>
        </Form.Item>
      </Form>

      <Space>
        <Button type="primary" icon={<VideoCameraOutlined />} disabled={!info.ffmpegFound || info.isRecording || cannotRecordSelectedAudio} onClick={startRecording}>开始录屏</Button>
        <Button danger disabled={!info.isRecording} onClick={stopRecording}>停止录屏</Button>
      </Space>

      <Alert
        type="info"
        showIcon
        style={{ marginTop: 18 }}
        message="音频说明"
        description="系统声音需要 ffmpeg 和 Windows 暴露 WASAPI / Stereo Mix / 虚拟声卡回环设备；麦克风通常可以直接选择。"
      />
    </Card>
  );
}
