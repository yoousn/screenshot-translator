import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type DiagnosticsIssue = {
  code: string;
  message: string;
  module: string;
  nextAction: string;
  severity: "error" | "warning" | "info" | string;
};

export type DiagnosticsReport = {
  health?: {
    ready: boolean;
    issueCount: number;
    criticalCount: number;
    issues: DiagnosticsIssue[];
    issuesByModule?: Record<string, number>;
    readinessByModule?: Record<string, {
      ready: boolean;
      readySteps: number;
      totalSteps: number;
      firstBlockedStep?: {
        id?: string;
        label?: string;
        description?: string;
        nextAction?: string;
      } | null;
    }>;
  };
  recovery?: Record<string, string>;
};

export default function useDiagnosticsReport() {
  const [report, setReport] = useState<DiagnosticsReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const next = await invoke<DiagnosticsReport>("get_diagnostics_report");
      setReport(next);
      return next;
    } catch (caught: any) {
      const message = caught?.message || String(caught);
      setError(message);
      return null;
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { report, loading, error, refresh };
}
