import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RecordingInfo } from "../components/config/types";
import type { RapidOcrStatus } from "../ocr-models";

export type StartupDependencySnapshot = {
  checkedAt?: string | null;
  ready?: boolean;
  pending?: boolean;
  rapidOcr?: RapidOcrStatus | null;
  recording?: RecordingInfo | null;
};

export default function useStartupDependencyStatus() {
  const [snapshot, setSnapshot] = useState<StartupDependencySnapshot | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setChecking(true);
    setError(null);
    try {
      const next = await invoke<StartupDependencySnapshot>("run_startup_readiness_probe");
      setSnapshot(next);
      return next;
    } catch (caught: any) {
      const message = caught?.message || String(caught);
      setError(message);
      return null;
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    invoke<StartupDependencySnapshot>("get_startup_readiness_snapshot")
      .then((cached) => {
        if (!cancelled && cached && cached.pending !== true) {
          setSnapshot(cached);
        }
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) {
          void refresh();
        }
      });
    return () => {
      cancelled = true;
    };
  }, [refresh]);

  return { snapshot, checking, error, refresh };
}
