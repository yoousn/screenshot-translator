import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message, Space } from "antd";
import {
  CameraOutlined,
  ClockCircleOutlined,
  DesktopOutlined,
  ScanOutlined,
  TranslationOutlined,
} from "@ant-design/icons";
import DashboardActionList, { type DashboardActionItem } from "../components/dashboard/DashboardActionList";
import DashboardHero from "../components/dashboard/DashboardHero";
import DashboardStats from "../components/dashboard/DashboardStats";
import DelayedCountdownOverlay from "../components/screenshot/DelayedCountdownOverlay";

const T = {
  title: "控制面板",
  desc: "管理截图、本地 OCR 和翻译流程。当前 OCR 强制在客户端本地执行，不再上传图片到 N100 做云端 OCR。",
  screenshot: "截图",
  screenshotDesc: "点击或通过快捷键开始框选截图。",
  translate: "截图翻译",
  translateDesc: "框选后先在本机 OCR，再只把文本发给 N100 翻译。",
  delayed: "延时截图",
  delayedDesc: "3 秒后开始截图，适合捕获菜单、下拉框或悬停状态。",
  ocr: "本地识字 OCR",
  ocrDesc: "框选后在客户端本地识别文字，结果自动复制到剪贴板。",
  fullscreen: "全屏复制",
  fullscreenDesc: "快速截取当前屏幕并复制到剪贴板。",
  run: "开始",
  server: "翻译服务",
  online: "在线",
  offline: "离线",
  checking: "检测中",
  hotkey: "截图快捷键",
  ocrMode: "OCR 模式",
  localOnly: "本地强制",
  targetLang: "目标语言",
  startScreenshot: "开始截图",
  startTranslateFailed: "启动截图翻译失败：",
  delayedInfo: "3 秒后开始截图，请准备好要截取的内容。",
  delayedCancel: "取消倒计时",
  delayedStarting: "即将开始截图",
  fullscreenCopied: "全屏截图已复制到剪贴板。",
  fullscreenFailed: "全屏截图失败：",
};

interface Config {
  serverUrl?: string;
  clientToken?: string;
  channel?: string;
  targetLang?: string;
  hotkey?: string;
}

interface DashboardProps {
  onStartScreenshot: () => void;
  shortcutError?: string | null;
  serverStatus: "checking" | "online" | "offline";
  responseTime: number | null;
  onRefreshStatus: () => void;
}

export default function Dashboard({ onStartScreenshot, shortcutError, serverStatus, responseTime }: DashboardProps) {
  const [config, setConfig] = useState<Config>({});
  const [delayedCountdown, setDelayedCountdown] = useState<number | null>(null);
  const [delayedActive, setDelayedActive] = useState(false);

  useEffect(() => {
    loadConfig();
  }, []);

  useEffect(() => {
    if (!delayedActive || delayedCountdown === null) return;
    if (delayedCountdown <= 0) {
      setDelayedCountdown(null);
      setDelayedActive(false);
      window.setTimeout(onStartScreenshot, 150);
      return;
    }
    const timer = window.setTimeout(() => setDelayedCountdown((prev) => (prev !== null ? prev - 1 : null)), 1000);
    return () => window.clearTimeout(timer);
  }, [delayedActive, delayedCountdown, onStartScreenshot]);

  const loadConfig = async () => {
    try {
      const configStr = await invoke<string>("get_config");
      setConfig(JSON.parse(configStr || "{}"));
    } catch (error) {
      console.error("Failed to load config:", error);
    }
  };

  const startTranslateScreenshot = async () => {
    try {
      await invoke("start_screenshot", { mode: "translate" });
    } catch (error: any) {
      message.error(`${T.startTranslateFailed}${error?.message || error}`);
    }
  };

  const handleDelayedScreenshot = () => {
    if (delayedActive) {
      setDelayedCountdown(null);
      setDelayedActive(false);
      return;
    }
    setDelayedCountdown(3);
    setDelayedActive(true);
    message.info(T.delayedInfo);
  };

  const handleFullscreenCapture = async () => {
    try {
      await invoke("quick_fullscreen_capture");
      message.success(T.fullscreenCopied);
    } catch (error: any) {
      message.error(`${T.fullscreenFailed}${error?.message || error}`);
    }
  };

  const statusText = serverStatus === "online" ? T.online : serverStatus === "offline" ? T.offline : T.checking;
  const statusColor = serverStatus === "online" ? "green" : serverStatus === "offline" ? "red" : "orange";

  const functionList: DashboardActionItem[] = [
    {
      title: T.screenshot,
      description: T.screenshotDesc,
      icon: <CameraOutlined style={{ fontSize: 18, color: "#1677ff" }} />,
      hotkey: config.hotkey || "Alt+A",
      buttonText: T.startScreenshot,
      danger: Boolean(shortcutError),
      onClick: onStartScreenshot,
    },
    {
      title: T.translate,
      description: T.translateDesc,
      icon: <TranslationOutlined style={{ fontSize: 18, color: "#1677ff" }} />,
      hotkey: "Alt+T",
      buttonText: T.translate,
      onClick: startTranslateScreenshot,
    },
    {
      title: T.delayed,
      description: T.delayedDesc,
      icon: <ClockCircleOutlined style={{ fontSize: 18, color: delayedActive ? "#1677ff" : "#fa8c16" }} />,
      hotkey: delayedActive ? `${delayedCountdown}s` : "Timer",
      buttonText: delayedActive ? `${delayedCountdown}s` : T.delayed,
      onClick: handleDelayedScreenshot,
    },
    {
      title: T.ocr,
      description: T.ocrDesc,
      icon: <ScanOutlined style={{ fontSize: 18, color: "#722ed1" }} />,
      hotkey: "工具栏",
      buttonText: T.screenshot,
      onClick: onStartScreenshot,
    },
    {
      title: T.fullscreen,
      description: T.fullscreenDesc,
      icon: <DesktopOutlined style={{ fontSize: 18, color: "#2f54eb" }} />,
      hotkey: "Instant",
      buttonText: T.fullscreen,
      onClick: handleFullscreenCapture,
    },
  ];

  return (
    <Space direction="vertical" size={16} style={{ width: "100%" }}>
      {delayedActive && delayedCountdown !== null && (
        <DelayedCountdownOverlay countdown={delayedCountdown} title={T.delayedStarting} onCancel={handleDelayedScreenshot} />
      )}
      <DashboardHero title={T.title} description={T.desc} buttonText={T.startScreenshot} onStartScreenshot={onStartScreenshot} />
      <DashboardStats
        hotkey={config.hotkey || "Alt+A"}
        ocrModeLabel={T.localOnly}
        targetLang={(config.targetLang || "zh").toUpperCase()}
        serverTitle={T.server}
        serverValue={responseTime ? `${responseTime}ms` : statusText}
        serverStatusText={statusText}
        serverStatusColor={statusColor}
        labels={{ hotkey: T.hotkey, ocrMode: T.ocrMode, targetLang: T.targetLang }}
      />
      <DashboardActionList
        title={T.title}
        delayedTitle={T.delayed}
        delayedActive={delayedActive}
        delayedCancelText={T.delayedCancel}
        defaultButtonText={T.run}
        items={functionList}
      />
    </Space>
  );
}
