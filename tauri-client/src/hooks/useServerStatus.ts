import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const DEFAULT_SERVER_URL = "https://ocr.yousn.me";

export default function useServerStatus() {
  const [serverUrl, setServerUrl] = useState<string>(DEFAULT_SERVER_URL);
  const [isOnline, setIsOnline] = useState<"checking" | "online" | "offline">("checking");
  const [isChecking, setIsChecking] = useState(false);
  const [responseTime, setResponseTime] = useState<number | null>(null);

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
        setIsOnline("online");
        setResponseTime(Math.round(performance.now() - start));
      } else {
        setIsOnline("offline");
        setResponseTime(null);
      }
    } catch {
      setIsOnline("offline");
      setResponseTime(null);
    } finally {
      window.clearTimeout(timeoutId);
      setIsChecking(false);
    }
  }, []);

  const fetchServerUrl = useCallback(async () => {
    try {
      const configStr = await invoke<string>("get_config");
      const parsedConfig = JSON.parse(configStr);
      const nextUrl = parsedConfig.serverUrl || DEFAULT_SERVER_URL;
      setServerUrl(nextUrl);
      checkStatus(nextUrl);
    } catch (error) {
      console.error("Failed to load config for App layout:", error);
      setServerUrl(DEFAULT_SERVER_URL);
      checkStatus(DEFAULT_SERVER_URL);
    }
  }, [checkStatus]);

  useEffect(() => {
    fetchServerUrl();
  }, [fetchServerUrl]);

  return { serverUrl, isOnline, isChecking, responseTime, checkStatus, fetchServerUrl };
}
