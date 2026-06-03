import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message } from "antd";
import type { LocalConfig } from "../utils/ocrConfigHelpers";
import { useI18n } from "../i18n";

export default function useOcrConfigController() {
  const { text } = useI18n();
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
      rapidOcrModelVersion: parsed.rapidOcrModelVersion === "v4" ? "v4" : "v5",
      rapidOcrMode: parsed.rapidOcrMode || "auto",
      rapidOcrWorkerEnabled: parsed.rapidOcrWorkerEnabled !== false,
    });
  };

  const saveConfig = async (patch: Partial<LocalConfig> = {}, showMessage = true) => {
    setSaving(true);
    try {
      const next = {
        ...config,
        ...patch,
        useLocalOcr: true,
        fallbackToRemoteOcr: false,
        rapidOcrModelVersion: patch.rapidOcrModelVersion || config.rapidOcrModelVersion || "v5",
        rapidOcrMode: patch.rapidOcrMode || config.rapidOcrMode || "auto",
        rapidOcrWorkerEnabled:
          typeof patch.rapidOcrWorkerEnabled === "boolean"
            ? patch.rapidOcrWorkerEnabled
            : config.rapidOcrWorkerEnabled !== false,
      };
      await invoke("save_config", { configStr: JSON.stringify(next) });
      setConfig(next);
      if (showMessage) message.success(labels.ocrConfigSaved);
    } catch (error: any) {
      message.error(labels.ocrConfigSaveFailed + (error?.message || error));
    } finally {
      setSaving(false);
    }
  };

  return { config, setConfig, saveConfig, saving };
}
