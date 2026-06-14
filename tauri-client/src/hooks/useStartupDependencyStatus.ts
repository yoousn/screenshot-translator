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

let cachedStartupSnapshot: StartupDependencySnapshot | null = null;
let startupProbeOncePromise: Promise<StartupDependencySnapshot | null> | null = null;

export const readStartupReadinessSnapshot = async () => {
  if (cachedStartupSnapshot) return cachedStartupSnapshot;
  const cached = await invoke<StartupDependencySnapshot>("get_startup_readiness_snapshot");
  if (cached && cached.pending !== true) {
    cachedStartupSnapshot = cached;
  }
  return cached;
};

const runStartupReadinessProbe = async () => {
  const next = await invoke<StartupDependencySnapshot>("run_startup_readiness_probe");
  cachedStartupSnapshot = next;
  return next;
};

export const runStartupReadinessProbeOnce = () => {
  if (!startupProbeOncePromise) {
    startupProbeOncePromise = runStartupReadinessProbe().catch((error) => {
      startupProbeOncePromise = null;
      throw error;
    });
  }
  return startupProbeOncePromise;
};

export const refreshStartupReadinessSnapshot = () => {
  startupProbeOncePromise = runStartupReadinessProbe().catch((error) => {
    startupProbeOncePromise = null;
    throw error;
  });
  return startupProbeOncePromise;
};

export default function useStartupDependencyStatus() {
  const [snapshot, setSnapshot] = useState<StartupDependencySnapshot | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setChecking(true);
    setError(null);
    try {
      const next = await refreshStartupReadinessSnapshot();
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
    readStartupReadinessSnapshot()
      .then((cached) => {
        if (!cancelled && cached && cached.pending !== true) {
          setSnapshot(cached);
        }
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) {
          setChecking(true);
          setError(null);
          runStartupReadinessProbeOnce()
            .then((next) => {
              if (!cancelled) setSnapshot(next);
            })
            .catch((caught: any) => {
              if (!cancelled) setError(caught?.message || String(caught));
            })
            .finally(() => {
              if (!cancelled) setChecking(false);
            });
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return { snapshot, checking, error, refresh };
}
