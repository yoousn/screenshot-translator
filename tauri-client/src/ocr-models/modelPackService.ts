import { defaultOcrModelManifest } from "./manifest";
import type { OcrModelManifest, OcrModelPackHealth, OcrModelPackStatus } from "./types";

const brokenStatuses: OcrModelPackStatus[] = ["download-failed", "verify-failed", "install-failed", "self-test-failed", "broken"];

export function getDefaultOcrModelManifest(): OcrModelManifest {
  return defaultOcrModelManifest;
}

export function summarizeOcrModelHealth(manifest: OcrModelManifest): OcrModelPackHealth {
  const requiredPacks = manifest.packs.filter((pack) => pack.required);
  const installed = requiredPacks.filter((pack) => pack.status === "installed").length;
  const missing = requiredPacks.filter((pack) => pack.status === "not-installed").map((pack) => pack.id);
  const broken = manifest.packs.filter((pack) => brokenStatuses.includes(pack.status)).map((pack) => pack.id);
  const updateAvailable = manifest.packs.filter((pack) => pack.status === "update-available").map((pack) => pack.id);

  return {
    installed,
    required: requiredPacks.length,
    missing,
    broken,
    updateAvailable,
    ready: requiredPacks.length > 0 && installed === requiredPacks.length && broken.length === 0,
  };
}

export function getPackStatusColor(status: OcrModelPackStatus) {
  if (status === "installed") return "green";
  if (status === "update-available") return "orange";
  if (status === "downloading" || status === "installing" || status === "self-testing") return "blue";
  if (brokenStatuses.includes(status)) return "red";
  return "default";
}
