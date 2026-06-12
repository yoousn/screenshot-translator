import React from "react";
import { Space, Tag } from "antd";
import { ApiOutlined, CloudOutlined, VideoCameraOutlined } from "@ant-design/icons";
import type { TranslationServiceMetadata } from "../../hooks/useServerStatus";
import type { StartupDependencySnapshot } from "../../hooks/useStartupDependencyStatus";

type TranslationStatus = "checking" | "online" | "offline";
type StatusKind = "ready" | "checking" | "missing" | "unavailable";

type DependencyStatusBarProps = {
  translationStatus: TranslationStatus;
  translationChecking: boolean;
  translationMetadata: TranslationServiceMetadata | null;
  dependencySnapshot: StartupDependencySnapshot | null;
  dependencyChecking: boolean;
  dependencyError?: string | null;
  onOpenTranslationSettings: () => void;
  onOpenModelManagement: () => void;
  onOpenDependencies: () => void;
};

type StatusPill = {
  key: string;
  label: string;
  detail: string;
  kind: StatusKind;
  icon: React.ReactNode;
  onClick: () => void;
};

const statusMeta = {
  ready: { color: "success", text: "可用", tone: "#16a34a", background: "#f0fdf4", border: "#bbf7d0" },
  checking: { color: "processing", text: "检测中", tone: "#2563eb", background: "#eff6ff", border: "#bfdbfe" },
  missing: { color: "warning", text: "缺失", tone: "#f97316", background: "#fff7ed", border: "#fed7aa" },
  unavailable: { color: "error", text: "不可用", tone: "#dc2626", background: "#fef2f2", border: "#fecaca" },
} as const;

function DependencyPill({ item }: { item: StatusPill }) {
  const meta = statusMeta[item.kind];

  return (
    <button
      type="button"
      title={`${item.label}: ${item.detail}`}
      onClick={item.onClick}
      style={{
        height: 30,
        padding: "0 8px",
        borderRadius: 999,
        border: `1px solid ${meta.border}`,
        background: meta.background,
        color: "#0f172a",
        cursor: "pointer",
        font: "inherit",
        display: "inline-flex",
        alignItems: "center",
      }}
    >
      <Space size={5}>
        <span style={{ color: meta.tone, display: "inline-flex" }}>{item.icon}</span>
        <span style={{ fontSize: 12, fontWeight: 700 }}>{item.label}</span>
        <Tag color={meta.color} style={{ margin: 0, borderRadius: 999, fontSize: 11, lineHeight: "18px" }}>
          {meta.text}
        </Tag>
      </Space>
    </button>
  );
}

export default function DependencyStatusBar({
  translationStatus,
  translationChecking,
  translationMetadata,
  dependencySnapshot,
  dependencyChecking,
  dependencyError,
  onOpenTranslationSettings,
  onOpenModelManagement,
  onOpenDependencies,
}: DependencyStatusBarProps) {
  const rapidOcr = dependencySnapshot?.rapidOcr || null;
  const recording = dependencySnapshot?.recording || null;
  const translationKind: StatusKind = translationStatus === "checking" || translationChecking ? "checking" : translationStatus === "online" ? "ready" : "unavailable";
  const ocrKind: StatusKind = dependencyChecking && !rapidOcr
    ? "checking"
    : rapidOcr?.ready
      ? "ready"
      : rapidOcr?.missingModelFiles?.length
        ? "missing"
        : "unavailable";
  const ffmpegKind: StatusKind = dependencyChecking && !recording
    ? "checking"
    : recording?.ffmpegFound
      ? "ready"
      : "missing";

  const items: StatusPill[] = [
    {
      key: "translation",
      label: "翻译",
      detail: translationStatus === "online"
        ? `当前通道：${translationMetadata?.active_channel || "默认"}`
        : translationStatus === "checking"
          ? "正在检测翻译服务"
          : "翻译服务不可用，请到系统设置检查通道",
      kind: translationKind,
      icon: <CloudOutlined />,
      onClick: onOpenTranslationSettings,
    },
    {
      key: "ocr",
      label: "识字",
      detail: rapidOcr?.ready
        ? `Rapid OCR ${(rapidOcr.rapidOcrModelVersion || "v5").toUpperCase()} 已可用`
        : rapidOcr?.missingModelFiles?.length
          ? `缺少 ${rapidOcr.missingModelFiles.length} 个模型文件`
          : rapidOcr?.lastError || dependencyError || "RapidOCR 尚未通过检测",
      kind: ocrKind,
      icon: <ApiOutlined />,
      onClick: onOpenModelManagement,
    },
    {
      key: "ffmpeg",
      label: "FFmpeg",
      detail: recording?.ffmpegFound
        ? recording.ffmpegPath || "已检测到 ffmpeg.exe"
        : "未找到 ffmpeg.exe，录屏前需要下载或选择",
      kind: ffmpegKind,
      icon: <VideoCameraOutlined />,
      onClick: onOpenDependencies,
    },
  ];

  return (
    <Space size={6} wrap={false}>
      {items.map((item) => (
        <DependencyPill key={item.key} item={item} />
      ))}
    </Space>
  );
}
