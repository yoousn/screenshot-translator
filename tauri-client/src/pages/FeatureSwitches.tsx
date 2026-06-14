import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AimOutlined, BorderOuterOutlined, ColumnWidthOutlined, EyeOutlined } from "@ant-design/icons";
import { Switch, Typography, message } from "antd";
import type { Config } from "../types/config";

const { Text } = Typography;

type FeatureSwitchRow = {
  keys: Array<keyof Config>;
  label: string;
  desc: string;
  icon: ReactNode;
  defaultVal: boolean;
  activeWhen?: "all" | "any";
};

const ROWS: FeatureSwitchRow[] = [
  {
    keys: ["enableMagnifier", "enableColorPicker"],
    label: "放大镜 / 取色器",
    desc: "截图时显示像素放大镜与 HEX 取色",
    icon: <EyeOutlined />,
    defaultVal: true,
    activeWhen: "all",
  },
  {
    keys: ["enableVisualDetection", "enableUiControlDetection"],
    label: "智能区域识别",
    desc: "单击时识别整窗、大区域、按钮和输入框等候选区域",
    icon: <AimOutlined />,
    defaultVal: false,
    activeWhen: "any",
  },
  {
    keys: ["edgeSnapEnabled"],
    label: "拖拽边缘吸附",
    desc: "拖拽选区边缘靠近候选区域时贴边；依赖上方识别结果",
    icon: <BorderOuterOutlined />,
    defaultVal: true,
  },
  {
    keys: ["enablePreciseSelection"],
    label: "精确选区调整",
    desc: "选区后可用方向键微调位置与宽高",
    icon: <ColumnWidthOutlined />,
    defaultVal: true,
  },
];

const containerStyle: CSSProperties = { padding: "20px 0 28px", width: "min(100%, 960px)", margin: "0 auto" };
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
const rowLeftStyle: CSSProperties = { display: "flex", alignItems: "center", gap: 16, minWidth: 0 };
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

  const isRowChecked = (row: FeatureSwitchRow) => {
    const values = row.keys.map((key) => (config as Record<string, unknown>)[key]);
    const isEnabled = (value: unknown) => (value !== undefined ? !!value : row.defaultVal);
    return row.activeWhen === "any" ? values.some(isEnabled) : values.every(isEnabled);
  };

  const toggle = async (row: FeatureSwitchRow, value: boolean) => {
    const savingKey = row.keys.join("|");
    setSaving(savingKey);
    const prev = config;
    const next = row.keys.reduce<Config>((acc, key) => ({ ...acc, [key]: value }), { ...config });
    setConfig(next);
    try {
      await invoke("save_config", { configStr: JSON.stringify(next, null, 4) });
      message.success(value ? "已开启 · 已保存" : "已关闭 · 已保存");
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
        开启或关闭截图工具的可选能力，切换后会自动保存并在下一次截图中生效。
      </Text>
      <div style={listStyle}>
        {ROWS.map((row) => {
          const checked = isRowChecked(row);
          const savingKey = row.keys.join("|");
          return (
            <div key={savingKey} style={rowStyle}>
              <div style={rowLeftStyle}>
                <span style={iconChipStyle(checked)}>{row.icon}</span>
                <div>
                  <div style={labelStyle}>{row.label}</div>
                  <div style={descStyle}>{row.desc}</div>
                </div>
              </div>
              <Switch
                checked={checked}
                loading={saving === savingKey}
                onChange={(value) => toggle(row, value)}
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
