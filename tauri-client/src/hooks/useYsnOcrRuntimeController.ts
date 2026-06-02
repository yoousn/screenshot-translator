import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { message } from "antd";
import { OCR_MODEL_PACK_PROGRESS_EVENT } from "../ocr-models";
import type { OcrModelPackOperation, YsnOcrManagedSourceDryRunResult, YsnOcrManagedSourceImportResult, YsnOcrManagedSourceTemplateResult, YsnOcrModelPackCommandResult, YsnOcrRuntimeStatus, YsnOcrSelfTestResult } from "../ocr-models";
import { useI18n } from "../i18n";

export default function useYsnOcrRuntimeController() {
  const { text } = useI18n();
  const labels = text.config;
  const [status, setStatus] = useState<YsnOcrRuntimeStatus | null>(null);
  const [loadingStatus, setLoadingStatus] = useState(false);
  const [selfTesting, setSelfTesting] = useState(false);
  const [runningPackAction, setRunningPackAction] = useState<string | null>(null);
  const [importingManagedSources, setImportingManagedSources] = useState(false);
  const [dryRunningManagedSources, setDryRunningManagedSources] = useState(false);
  const [creatingManagedSourceTemplate, setCreatingManagedSourceTemplate] = useState(false);
  const [lastSelfTest, setLastSelfTest] = useState<YsnOcrSelfTestResult | null>(null);
  const [lastOperation, setLastOperation] = useState<OcrModelPackOperation | null>(null);
  const [lastManagedSourceImport, setLastManagedSourceImport] = useState<YsnOcrManagedSourceImportResult | null>(null);
  const [lastManagedSourceDryRun, setLastManagedSourceDryRun] = useState<YsnOcrManagedSourceDryRunResult | null>(null);
  const [lastManagedSourceTemplate, setLastManagedSourceTemplate] = useState<YsnOcrManagedSourceTemplateResult | null>(null);

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

  const importManagedSourceIndex = async () => {
    setImportingManagedSources(true);
    try {
      const indexPath = await invoke<string | null>("choose_ysn_ocr_managed_source_index_file", { currentPath: null });
      if (!indexPath) return null;
      const result = await invoke<YsnOcrManagedSourceImportResult>("import_ysn_ocr_managed_source_index", { indexPath });
      setLastManagedSourceImport(result);
      if (result.ok) message.success((result.message || labels.managedSourceImportSuccess).replace("{count}", String(result.updatedCount || 0)));
      else message.warning(result.message || labels.managedSourceImportPending);
      await refreshStatus();
      return result;
    } catch (error: any) {
      message.error(labels.managedSourceImportFailed + (error?.message || error));
      return null;
    } finally {
      setImportingManagedSources(false);
    }
  };


  const dryRunManagedSourceIndex = async () => {
    setDryRunningManagedSources(true);
    try {
      const indexPath = await invoke<string | null>("choose_ysn_ocr_managed_source_index_file", { currentPath: null });
      if (!indexPath) return null;
      const result = await invoke<YsnOcrManagedSourceDryRunResult>("dry_run_ysn_ocr_managed_source_index", { indexPath, packId: null });
      setLastManagedSourceDryRun(result);
      if (result.ok) message.success(result.message || labels.managedSourceDryRunPassed);
      else message.warning(result.message || labels.managedSourceDryRunBlocked);
      return result;
    } catch (error: any) {
      message.error(labels.managedSourceDryRunFailed + (error?.message || error));
      return null;
    } finally {
      setDryRunningManagedSources(false);
    }
  };
  const createManagedSourceTemplate = async () => {
    setCreatingManagedSourceTemplate(true);
    try {
      const result = await invoke<YsnOcrManagedSourceTemplateResult>("create_ysn_ocr_managed_source_index_template");
      setLastManagedSourceTemplate(result);
      message.success(labels.managedSourceTemplateCreated.replace("{count}", String(result.modelCount || 0)));
      return result;
    } catch (error: any) {
      message.error(labels.managedSourceTemplateFailed + (error?.message || error));
      return null;
    } finally {
      setCreatingManagedSourceTemplate(false);
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
    importingManagedSources,
    dryRunningManagedSources,
    creatingManagedSourceTemplate,
    lastSelfTest,
    lastOperation,
    lastManagedSourceImport,
    lastManagedSourceDryRun,
    lastManagedSourceTemplate,
    refreshStatus,
    runSelfTest,
    installPack,
    updatePack,
    importManagedSourceIndex,
    dryRunManagedSourceIndex,
    createManagedSourceTemplate,
  };
}

