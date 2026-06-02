import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { message } from "antd";
import { OCR_MODEL_PACK_PROGRESS_EVENT } from "../ocr-models";
import type { OcrModelPackOperation, YsnOcrModelPackCommandResult, YsnOcrRuntimeStatus, YsnOcrSelfTestResult } from "../ocr-models";
import { useI18n } from "../i18n";

export default function useYsnOcrRuntimeController() {
  const { text } = useI18n();
  const labels = text.config;
  const [status, setStatus] = useState<YsnOcrRuntimeStatus | null>(null);
  const [loadingStatus, setLoadingStatus] = useState(false);
  const [selfTesting, setSelfTesting] = useState(false);
  const [runningPackAction, setRunningPackAction] = useState<string | null>(null);
  const [lastSelfTest, setLastSelfTest] = useState<YsnOcrSelfTestResult | null>(null);
  const [lastOperation, setLastOperation] = useState<OcrModelPackOperation | null>(null);

  const refreshStatus = async () => {
    setLoadingStatus(true);
    try {
      const next = await invoke<YsnOcrRuntimeStatus>("get_ysn_ocr_status");
      setStatus(next);
      return next;
    } catch (error: any) {
      message.error(labels.ysnRuntimeStatusFailed + (error?.message || error));
      return null;
    } finally {
      setLoadingStatus(false);
    }
  };

  const runSelfTest = async () => {
    setSelfTesting(true);
    try {
      const result = await invoke<YsnOcrSelfTestResult>("run_ysn_ocr_self_test");
      setLastSelfTest(result);
      if (result.ok) message.success(labels.ysnSelfTestPassed);
      else message.warning(result.message || labels.ysnSelfTestPending);
      await refreshStatus();
      return result;
    } catch (error: any) {
      message.error(labels.ysnSelfTestFailed + (error?.message || error));
      return null;
    } finally {
      setSelfTesting(false);
    }
  };

  const installPack = async (packId: string) => {
    setRunningPackAction(packId);
    try {
      const result = await invoke<YsnOcrModelPackCommandResult>("install_ysn_ocr_model_pack", { packId });
      message.info(result.message || labels.modelPackInstallerPending);
      await refreshStatus();
      return result;
    } catch (error: any) {
      message.error(labels.modelPackInstallFailed + (error?.message || error));
      return null;
    } finally {
      setRunningPackAction(null);
    }
  };

  const updatePack = async (packId: string) => {
    setRunningPackAction(packId);
    try {
      const result = await invoke<YsnOcrModelPackCommandResult>("update_ysn_ocr_model_pack", { packId });
      message.info(result.message || labels.modelPackUpdaterPending);
      await refreshStatus();
      return result;
    } catch (error: any) {
      message.error(labels.modelPackUpdateFailed + (error?.message || error));
      return null;
    } finally {
      setRunningPackAction(null);
    }
  };


  useEffect(() => {
    refreshStatus();
    let dispose: (() => void) | null = null;
    listen<OcrModelPackOperation>(OCR_MODEL_PACK_PROGRESS_EVENT, (event) => {
      setLastOperation(event.payload);
    }).then((unlisten) => {
      dispose = unlisten;
    }).catch(() => {});
    return () => {
      if (dispose) dispose();
    };
  }, []);

  return {
    status,
    loadingStatus,
    selfTesting,
    runningPackAction,
    lastSelfTest,
    lastOperation,
    refreshStatus,
    runSelfTest,
    installPack,
    updatePack,
  };
}

