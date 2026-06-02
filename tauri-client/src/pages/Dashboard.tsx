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
import DashboardDiagnosticsCard from "../components/dashboard/DashboardDiagnosticsCard";
import DashboardHero from "../components/dashboard/DashboardHero";
import DashboardReadiness from "../components/dashboard/DashboardReadiness";
import DashboardStats from "../components/dashboard/DashboardStats";
import DelayedCountdownOverlay from "../components/screenshot/DelayedCountdownOverlay";
import useDiagnosticsReport from "../hooks/useDiagnosticsReport";
import { useI18n } from "../i18n";

interface Config {
  serverUrl?: string;
  clientToken?: string;
  channel?: string;
  targetLang?: string;
  hotkey?: string;
}

interface DashboardProps {
  onStartScreenshot: () => void;
  onNavigate: (key: string) => void;
  shortcutError?: string | null;
  serverStatus: "checking" | "online" | "offline";
  responseTime: number | null;
}

const targetLanguageLabel = (targetLang: string | undefined, fallback: string) => {
  if (!targetLang || targetLang === "zh" || targetLang === "zh-CN") return fallback;
  return targetLang.toUpperCase();
};

export default function Dashboard({ onStartScreenshot, onNavigate, shortcutError, serverStatus, responseTime }: DashboardProps) {
  const [config, setConfig] = useState<Config>({});
  const [delayedCountdown, setDelayedCountdown] = useState<number | null>(null);
  const [delayedActive, setDelayedActive] = useState(false);
  const diagnostics = useDiagnosticsReport();
  const { text } = useI18n();
  const T = text.dashboard;

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

  const statusText = serverStatus === "online" ? text.status.online : serverStatus === "offline" ? text.status.offline : text.status.checking;
  const statusColor = serverStatus === "online" ? "green" : serverStatus === "offline" ? "red" : "orange";

  const functionList: DashboardActionItem[] = [
    {
      title: T.screenshot,
      description: T.screenshotDesc,
      icon: <CameraOutlined style={{ fontSize: 18, color: "#1677ff" }} />,
      hotkey: config.hotkey || "Alt+A",
      buttonText: T.primaryAction,
      danger: Boolean(shortcutError),
      onClick: onStartScreenshot,
    },
    {
      title: T.translate,
      description: T.translateDesc,
      icon: <TranslationOutlined style={{ fontSize: 18, color: "#1677ff" }} />,
      hotkey: "Ctrl+Q",
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
      hotkey: "Ctrl+D",
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
      <DashboardHero title={T.title} description={T.desc} buttonText={T.primaryAction} onStartScreenshot={onStartScreenshot} />
      <DashboardStats
        hotkey={config.hotkey || "Alt+A"}
        ocrModeLabel={T.ocrModeValue}
        targetLang={targetLanguageLabel(config.targetLang, T.targetLangDefault)}
        serverTitle={T.service}
        serverValue={responseTime ? `${responseTime}ms` : statusText}
        serverStatusText={statusText}
        serverStatusColor={statusColor}
        labels={{ hotkey: T.hotkey, ocrMode: T.ocrMode, targetLang: T.targetLang }}
      />
      <DashboardReadiness
        labels={T}
        serverStatus={serverStatus}
        shortcutError={shortcutError}
        onOpenModels={() => onNavigate("ocr-config")}
        onOpenSettings={() => onNavigate("settings")}
      />
      <DashboardDiagnosticsCard
        labels={{ ...T, ...text.config }}
        report={diagnostics.report}
        loading={diagnostics.loading}
        error={diagnostics.error}
        onRefresh={diagnostics.refresh}
        onOpenModels={() => onNavigate("ocr-config")}
        onOpenSettings={() => onNavigate("settings")}
      />
      <DashboardActionList
        title={T.commandCenter}
        delayedTitle={T.delayed}
        delayedActive={delayedActive}
        delayedCancelText={T.delayedCancel}
        defaultButtonText={T.run}
        items={functionList}
      />
    </Space>
  );
}
