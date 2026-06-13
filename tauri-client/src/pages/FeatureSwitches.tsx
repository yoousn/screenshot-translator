import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Switch, Typography, message } from "antd";
import {
  EyeOutlined,
  AimOutlined,
  ColumnWidthOutlined,
  EditOutlined,
  PoweroffOutlined,
} from "@ant-design/icons";
import type { Config } from "../types/config";

const { Text } = Typography;

type FeatureSwitchRow = {
  key: keyof Config;
  label: string;
  desc: string;
  icon: React.ReactNode;
  defaultVal: boolean;
};

const ROWS: FeatureSwitchRow[] = [
  { key: "enableMagnifier", label: "放大镜 / 取色器", desc: "截图时显示像素放大镜与 HEX 取色", icon: <EyeOutlined />, defaultVal: true },
  { key: "enableVisualDetection", label: "智能窗口磁吸", desc: "自动识别窗口/控件边界", icon: <AimOutlined />, defaultVal: false },
  { key: "enablePreciseSelection", label: "精确选区输入框", desc: "可输入宽高与方向键微调", icon: <ColumnWidthOutlined />, defaultVal: true },
  { key: "enableLiveAnnotation", label: "实时标注", desc: "画笔边画边显示（P0-1 已生效）", icon: <EditOutlined />, defaultVal: true },
  { key: "autoStart", label: "开机自启", desc: "开机时自动启动软件", icon: <PoweroffOutlined />, defaultVal: false },
];

export default function FeatureSwitches() {
  const [config, setConfig] = useState<Config>({});
  const [loading, setLoading] = useState(true);

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
    try {
      const next = { ...config, [key]: value };
      setConfig(next);
      await invoke("save_config", { configStr: JSON.stringify(next, null, 4) });

      // Special handling for autoStart
      if (key === "autoStart") {
        await invoke("set_autostart_enabled", { enabled: value });
      }
    } catch {
      message.error("保存配置失败");
    }
  };

  if (loading) return null;

  return (
    <div style={{ padding: "20px 24px", maxWidth: 560, margin: "0 auto" }}>
      <Text strong style={{ fontSize: 18, display: "block", marginBottom: 16, color: "#0f172a" }}>
        功能开关
      </Text>
      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {ROWS.map((row) => {
          const val = (config as Record<string, unknown>)[row.key];
          const checked = val !== undefined ? !!val : row.defaultVal;
          return (
            <div
              key={row.key}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "12px 14px",
                borderRadius: 10,
                background: "rgba(255,255,255,0.72)",
                border: "1px solid rgba(226,232,240,0.7)",
                transition: "box-shadow 180ms ease",
              }}
              className="btn-press"
            >
              <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                <span style={{ fontSize: 16, color: "#64748b" }}>{row.icon}</span>
                <div>
                  <div style={{ fontWeight: 600, fontSize: 13, color: "#1e293b" }}>{row.label}</div>
                  <div style={{ opacity: 0.55, fontSize: 11, color: "#475569", marginTop: 1 }}>{row.desc}</div>
                </div>
              </div>
              <Switch
                size="small"
                checked={checked}
                onChange={(v) => toggle(row.key, v)}
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}
