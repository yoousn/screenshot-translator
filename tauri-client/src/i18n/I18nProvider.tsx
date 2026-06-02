import React, { createContext, useContext, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import zhCN from "antd/locale/zh_CN";
import enUS from "antd/locale/en_US";
import { dictionaries, normalizeLanguage } from "./dictionaries";
import type { AppLanguage, I18nContextValue } from "./types";

const I18nContext = createContext<I18nContextValue | null>(null);

async function persistLanguage(language: AppLanguage) {
  const configStr = await invoke<string>("get_config");
  const config = JSON.parse(configStr || "{}");
  await invoke("save_config", {
    configStr: JSON.stringify({ ...config, appLanguage: language }, null, 2),
  });
}

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [language, setLanguageState] = useState<AppLanguage>("zh-CN");

  useEffect(() => {
    invoke<string>("get_config")
      .then((configStr) => {
        const config = JSON.parse(configStr || "{}");
        setLanguageState(normalizeLanguage(config.appLanguage || config.language));
      })
      .catch(() => undefined);
  }, []);

  const setLanguage = async (nextLanguage: AppLanguage) => {
    setLanguageState(nextLanguage);
    await persistLanguage(nextLanguage).catch(() => undefined);
  };

  const value = useMemo<I18nContextValue>(() => ({
    language,
    text: dictionaries[language],
    antdLocale: language === "en-US" ? enUS : zhCN,
    setLanguage,
  }), [language]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  const context = useContext(I18nContext);
  if (!context) throw new Error("useI18n must be used within I18nProvider");
  return context;
}
