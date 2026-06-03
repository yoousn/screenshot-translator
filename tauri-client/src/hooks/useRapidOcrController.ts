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
  const [workerBusy, setWorkerBusy] = useState(false);
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

  const startWorker = async () => {
    setWorkerBusy(true);
    try {
      await invoke("start_rapid_ocr_worker");
      message.success("RapidOCR 常驻识别服务已启动。");
      await refreshStatus();
    } catch (error: any) {
      message.error(`启动 RapidOCR 常驻服务失败：${error?.message || error}`);
    } finally {
      setWorkerBusy(false);
    }
  };

  const stopWorker = async () => {
    setWorkerBusy(true);
    try {
      await invoke("stop_rapid_ocr_worker");
      message.success("RapidOCR 常驻识别服务已停止。");
      await refreshStatus();
    } catch (error: any) {
      message.error(`停止 RapidOCR 常驻服务失败：${error?.message || error}`);
    } finally {
      setWorkerBusy(false);
    }
  };

  const restartWorker = async () => {
    setWorkerBusy(true);
    try {
      await invoke("restart_rapid_ocr_worker");
      message.success("RapidOCR 常驻识别服务已重启。");
      await refreshStatus();
    } catch (error: any) {
      message.error(`重启 RapidOCR 常驻服务失败：${error?.message || error}`);
    } finally {
      setWorkerBusy(false);
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
    workerBusy,
    lastSelfTest,
    refreshStatus,
    runSelfTest,
    startWorker,
    stopWorker,
    restartWorker,
  };
}
