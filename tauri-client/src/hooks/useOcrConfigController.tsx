import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { App as AntdApp } from "antd";
import type { LocalConfig } from "../utils/ocrConfigHelpers";
import { useI18n } from "../i18n";
import { useConfigHotReload } from "./useConfigHotReload";

export default function useOcrConfigController() {
  const { text } = useI18n();
  const { message } = AntdApp.useApp();
  const labels = text.config;
  const [config, setConfig] = useState<LocalConfig>({});
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    const configStr = await invoke<string>("get_config");
    const parsed = configStr ? JSON.parse(configStr) : {};
    setConfig({
      ...parsed,
      useLocalOcr: true,
      fallbackToRemoteOcr: false,
      localOcrTimeoutMs: parsed.localOcrTimeoutMs || 15000,
      rapidOcrModelVersion: ["v6", "v5", "v4"].includes(parsed.rapidOcrModelVersion)
        ? parsed.rapidOcrModelVersion
        : "v6",
      rapidOcrMode: parsed.rapidOcrMode || "auto",
      rapidOcrWorkerEnabled: parsed.rapidOcrWorkerEnabled !== false,
    });
  };

  // 工单②：监听配置变更广播，使主窗口各设置入口（模型管理 / OCR 配置等）
  // 在其它窗口或页面改了配置后能自动同步，无需手动刷新或重启。
  useConfigHotReload(loadConfig);

  const saveConfig = async (patch: Partial<LocalConfig> = {}, showMessage = true) => {
    setSaving(true);
    try {
      const next = {
        ...config,
        ...patch,
        useLocalOcr: true,
        fallbackToRemoteOcr: false,
        rapidOcrModelVersion: patch.rapidOcrModelVersion || config.rapidOcrModelVersion || "v6",
        rapidOcrMode: patch.rapidOcrMode || config.rapidOcrMode || "auto",
        rapidOcrWorkerEnabled:
          typeof patch.rapidOcrWorkerEnabled === "boolean"
            ? patch.rapidOcrWorkerEnabled
            : config.rapidOcrWorkerEnabled !== false,
      };
      await invoke("save_config", { configStr: JSON.stringify(next) });
      setConfig(next);
      if (showMessage) message.success(labels.ocrConfigSaved);
      return true;
    } catch (error: any) {
      message.error(labels.ocrConfigSaveFailed + (error?.message || error));
      return false;
    } finally {
      setSaving(false);
    }
  };

  return { config, setConfig, saveConfig, saving };
}
