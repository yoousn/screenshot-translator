import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Switch, Typography, message } from "antd";
import {
  EyeOutlined,
  AimOutlined,
  ColumnWidthOutlined,
  EditOutlined,
  PoweroffOutlined,
  AppstoreOutlined,
  BorderOuterOutlined,
} from "@ant-design/icons";
import type { Config } from "../types/config";

const { Text } = Typography;

type FeatureSwitchRow = {
  key: keyof Config;
  label: string;
  desc: string;
  icon: ReactNode;
  defaultVal: boolean;
};

const ROWS: FeatureSwitchRow[] = [
  { key: "enableMagnifier", label: "放大镜 / 取色器", desc: "截图时显示像素放大镜与 HEX 取色", icon: <EyeOutlined />, defaultVal: true },
  { key: "enableVisualDetection", label: "智能窗口磁吸", desc: "拖动时自动识别整窗/大区域并吸附", icon: <AimOutlined />, defaultVal: false },
  { key: "enableUiControlDetection", label: "UI 控件检测", desc: "识别按钮、输入框等控件边界并吸附", icon: <AppstoreOutlined />, defaultVal: false },
  { key: "edgeSnapEnabled", label: "窗口边缘吸附", desc: "拖动框选时自动贴近附近窗口边缘", icon: <BorderOuterOutlined />, defaultVal: true },
  { key: "enablePreciseSelection", label: "精确选区调整", desc: "选区后可用方向键微调位置与宽高", icon: <ColumnWidthOutlined />, defaultVal: true },
  { key: "enableLiveAnnotation", label: "实时标注", desc: "画笔边画边显示（恒开）", icon: <EditOutlined />, defaultVal: true },
  { key: "autoStart", label: "开机自启", desc: "开机时自动启动软件", icon: <PoweroffOutlined />, defaultVal: false },
];

const containerStyle: CSSProperties = { padding: "28px 32px", maxWidth: 720, margin: "0 auto" };
const titleStyle: CSSProperties = { fontSize: 20, display: "block", marginBottom: 6, color: "#0f172a" };
const subtitleStyle: CSSProperties = { display: "block", marginBottom: 20, fontSize: 13, color: "#94a3b8" };
const listStyle: CSSProperties = { display: "flex", flexDirection: "column", gap: 12 };
const rowStyle: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  padding: "18px 22px",
  borderRadius: 14,
  background: "rgba(255,255,255,0.9)",
  border: "1px solid rgba(226,232,240,0.9)",
  boxShadow: "0 1px 2px rgba(15,23,42,0.04)",
};
const rowLeftStyle: CSSProperties = { display: "flex", alignItems: "center", gap: 16 };
const labelStyle: CSSProperties = { fontWeight: 600, fontSize: 15, color: "#1e293b" };
const descStyle: CSSProperties = { fontSize: 12.5, color: "#64748b", marginTop: 3 };

const iconChipStyle = (checked: boolean): CSSProperties => ({
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  width: 40,
  height: 40,
  borderRadius: 10,
  fontSize: 20,
  color: checked ? "#2563eb" : "#94a3b8",
  background: checked ? "rgba(37,99,235,0.1)" : "rgba(148,163,184,0.12)",
  transition: "color 180ms ease, background 180ms ease",
});

export default function FeatureSwitches() {
  const [config, setConfig] = useState<Config>({});
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const raw = await invoke<string>("get_config");
        setConfig(JSON.parse(raw || "{}"));
      } catch {
        message.error("加载配置失败");
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const toggle = async (key: keyof Config, value: boolean) => {
    setSaving(key);
    const prev = config;
    const next = { ...config, [key]: value };
    setConfig(next);
    try {
      await invoke("save_config", { configStr: JSON.stringify(next, null, 4) });
      if (key === "autoStart") {
        await invoke("set_autostart_enabled", { enabled: value });
      }
      message.success(value ? "已开启·已保存" : "已关闭·已保存");
    } catch {
      setConfig(prev);
      message.error("保存失败，已撤销更改");
    } finally {
      setSaving(null);
    }
  };

  if (loading) return null;

  return (
    <div style={containerStyle}>
      <Text strong style={titleStyle}>
        功能开关
      </Text>
      <Text style={subtitleStyle}>
        开启或关闭截图工具的各项能力，切换后自动保存并立即生效。
      </Text>
      <div style={listStyle}>
        {ROWS.map((row) => {
          const val = (config as Record<string, unknown>)[row.key];
          const checked = val !== undefined ? !!val : row.defaultVal;
          return (
            <div key={row.key} style={rowStyle}>
              <div style={rowLeftStyle}>
                <span style={iconChipStyle(checked)}>{row.icon}</span>
                <div>
                  <div style={labelStyle}>{row.label}</div>
                  <div style={descStyle}>{row.desc}</div>
                </div>
              </div>
              <Switch
                checked={checked}
                loading={saving === row.key}
                onChange={(v) => toggle(row.key, v)}
                checkedChildren="开"
                unCheckedChildren="关"
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
