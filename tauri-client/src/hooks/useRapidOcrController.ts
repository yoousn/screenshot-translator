import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { App as AntdApp } from "antd";
import type { RapidOcrSelfTestResult, RapidOcrStatus } from "../ocr-models";
import { readStartupReadinessSnapshot } from "./useStartupDependencyStatus";

type UseRapidOcrControllerOptions = {
  autoRefresh?: boolean;
};

export default function useRapidOcrController(options: UseRapidOcrControllerOptions = {}) {
  const { autoRefresh = false } = options;
  const { message } = AntdApp.useApp();
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
        message.success("本地 OCR 初始化完成，当前模型已可用于截图识字。");
      } else {
        message.warning(result.message || "本地 OCR 初始化未通过。");
      }
      await refreshStatus();
      return result;
    } catch (error: any) {
      message.error(`本地 OCR 初始化失败：${error?.message || error}`);
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
    readStartupReadinessSnapshot()
      .then((snapshot) => {
        if (snapshot?.rapidOcr) setStatus(snapshot.rapidOcr);
      })
      .catch(() => {});
  }, [autoRefresh]);

  return {
    status,
    loadingStatus,
    selfTesting,
    initializing: selfTesting,
    workerBusy,
    lastSelfTest,
    refreshStatus,
    runSelfTest,
    initializeAndApply: runSelfTest,
    startWorker,
    stopWorker,
    restartWorker,
  };
}
