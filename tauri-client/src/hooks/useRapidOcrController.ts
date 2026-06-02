import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message } from "antd";
import type { RapidOcrSelfTestResult, RapidOcrStatus } from "../ocr-models";

type UseRapidOcrControllerOptions = {
  autoRefresh?: boolean;
};

export default function useRapidOcrController(options: UseRapidOcrControllerOptions = {}) {
  const { autoRefresh = false } = options;
  const [status, setStatus] = useState<RapidOcrStatus | null>(null);
  const [loadingStatus, setLoadingStatus] = useState(false);
  const [selfTesting, setSelfTesting] = useState(false);
  const [lastSelfTest, setLastSelfTest] = useState<RapidOcrSelfTestResult | null>(null);

  const refreshStatus = async () => {
    setLoadingStatus(true);
    try {
      const next = await invoke<RapidOcrStatus>("get_rapid_ocr_status");
      setStatus(next);
      return next;
    } catch (error: any) {
      message.error(`RapidOCR 状态读取失败：${error?.message || error}`);
      return null;
    } finally {
      setLoadingStatus(false);
    }
  };

  const runSelfTest = async () => {
    setSelfTesting(true);
    try {
      const result = await invoke<RapidOcrSelfTestResult>("run_rapid_ocr_self_test");
      setLastSelfTest(result);
      if (result.ok) {
        message.success("RapidOCR 自测通过。");
      } else {
        message.warning(result.message || "RapidOCR 自测未通过。");
      }
      await refreshStatus();
      return result;
    } catch (error: any) {
      message.error(`RapidOCR 自测失败：${error?.message || error}`);
      return null;
    } finally {
      setSelfTesting(false);
    }
  };

  useEffect(() => {
    if (autoRefresh) {
      refreshStatus();
      return;
    }
    invoke<any>("get_startup_readiness_snapshot")
      .then((snapshot) => {
        if (snapshot?.rapidOcr) setStatus(snapshot.rapidOcr);
      })
      .catch(() => {});
  }, [autoRefresh]);

  return {
    status,
    loadingStatus,
    selfTesting,
    lastSelfTest,
    refreshStatus,
    runSelfTest,
  };
}
