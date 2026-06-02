import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_TRANSLATION_SERVICE_URL } from "../utils/translationService";

const buildStatusServerUrl = (config: any) => (
  config.preferLanServer && config.lanServerUrl ? config.lanServerUrl : (config.serverUrl || DEFAULT_TRANSLATION_SERVICE_URL)
);

export type TranslationServiceMetadata = {
  active_channel?: string;
  glossary_version?: string;
  glossary_loaded?: boolean;
  glossary_terms?: number;
  quality_flags?: Record<string, boolean>;
};

export default function useServerStatus() {
  const [serverUrl, setServerUrl] = useState<string>(DEFAULT_TRANSLATION_SERVICE_URL);
  const [isOnline, setIsOnline] = useState<"checking" | "online" | "offline">("checking");
  const [isChecking, setIsChecking] = useState(false);
  const [responseTime, setResponseTime] = useState<number | null>(null);
  const [translationMetadata, setTranslationMetadata] = useState<TranslationServiceMetadata | null>(null);

  const checkStatus = useCallback(async (url: string) => {
    setIsChecking(true);
    setIsOnline("checking");
    const start = performance.now();
    const controller = new AbortController();
    const timeoutId = window.setTimeout(() => controller.abort(), 4000);
    try {
      const response = await fetch(`${url.replace(/\/$/, "")}/api/health`, {
        method: "GET",
        signal: controller.signal,
      });
      if (response.ok) {
        const data = await response.json().catch(() => null);
        setIsOnline("online");
        setResponseTime(Math.round(performance.now() - start));
        setTranslationMetadata(data?.translation || null);
      } else {
        setIsOnline("offline");
        setResponseTime(null);
        setTranslationMetadata(null);
      }
    } catch {
      setIsOnline("offline");
      setResponseTime(null);
      setTranslationMetadata(null);
    } finally {
      window.clearTimeout(timeoutId);
      setIsChecking(false);
    }
  }, []);

  const fetchServerUrl = useCallback(async () => {
    try {
      const configStr = await invoke<string>("get_config");
      const parsedConfig = JSON.parse(configStr);
      const nextUrl = buildStatusServerUrl(parsedConfig);
      setServerUrl(nextUrl);
      checkStatus(nextUrl);
    } catch (error) {
      console.error("Failed to load config for App layout:", error);
      setServerUrl(DEFAULT_TRANSLATION_SERVICE_URL);
      checkStatus(DEFAULT_TRANSLATION_SERVICE_URL);
    }
  }, [checkStatus]);

  useEffect(() => {
    fetchServerUrl();
  }, [fetchServerUrl]);

  return { serverUrl, isOnline, isChecking, responseTime, translationMetadata, checkStatus, fetchServerUrl };
}
