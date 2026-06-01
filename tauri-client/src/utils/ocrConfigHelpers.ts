export const REPO_API = "https://api.github.com/repos/hiroi-sora/PaddleOCR-json/releases/latest";
export const REPO_URL = "https://github.com/hiroi-sora/PaddleOCR-json";

export const T = {
  pageTitle: "OCR Models / Video Recording",
  pageDesc: "Manage the lightweight OCR runtime, compatibility OCR, and FFmpeg recording dependency.",
  repoTitle: "RapidOCR ONNX is the recommended OCR runtime",
  repoDesc: "Choose or install a RapidOCR ONNX runtime directory with ocr-runtime.json. PaddleOCR-json remains supported as a compatibility mode, but it is no longer the recommended default for screenshots.",
  localTitle: "OCR Runtime",
  exePath: "OCR runtime path",
  exePlaceholder: "Leave empty to auto-detect app ocr runtime, or choose a RapidOCR ONNX / PaddleOCR-json runtime directory.",
  timeout: "Local OCR timeout (ms)",
  save: "Save OCR config",
  saved: "OCR config saved",
  openDir: "Open directory",
  mode: "OCR mode",
  localOnly: "Local runtime",
  status: "Status",
  statusUnknown: "Not checked",
  statusOk: "Available",
  statusBad: "Unavailable",
  checkStatus: "Check runtime",
  checkedStatus: "OCR runtime checked",
  updateTitle: "Compatibility OCR download",
  check: "Check release",
  downloadFirst: "Download compatibility OCR",
  downloadUpdate: "Update compatibility OCR",
  officialRepo: "Runtime repository",
  downloadedVersion: "Installed version",
  latestVersion: "Latest version",
  assetName: "Runtime asset",
  lastChecked: "Last checked",
  installDir: "Install directory",
  downloadSize: "Download size",
  notDownloaded: "Not installed",
  notChecked: "Not checked",
  hasUpdate: "Update available",
  fullLog: "Release notes",
  openInstallDir: "Open OCR directory",
  moveInstallDir: "Move OCR directory",
  moving: "Moving OCR directory...",
  moved: "OCR directory moved",
  checkFirst: "Check release first",
  noWindowsAsset: "No Windows x64 runtime asset found in latest release.",
  officialNoLog: "No release notes provided.",
  checkedLatest: "Latest compatibility OCR version: ",
  downloaded: "Installed compatibility OCR ",
  saveFailed: "Save failed: ",
  checkFailed: "Release check failed: ",
  statusFailed: "Runtime check failed: ",
  downloadFailed: "Download/install failed: ",
};

export interface LocalConfig {
  localOcrExecutablePath?: string;
  localOcrTimeoutMs?: number;
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  paddleOcrReleaseTag?: string;
  paddleOcrReleasePath?: string;
  paddleOcrInstallDir?: string;
  paddleOcrReleaseAssetName?: string;
  paddleOcrReleaseCheckedAt?: string;
}

export interface GitHubAsset {
  name: string;
  browser_download_url: string;
  size?: number;
}

export interface GitHubRelease {
  tag_name: string;
  name?: string;
  html_url: string;
  published_at?: string;
  body?: string;
  assets?: GitHubAsset[];
}

export interface DownloadResult {
  path: string;
  installDir: string;
  bytes: number;
}

export interface ProgressPayload {
  phase: string;
  downloaded: number;
  total?: number;
  percent: number;
}

export interface RuntimeManifest {
  id?: string;
  name?: string;
  engine?: string;
  version?: string;
  entry?: string;
  protocol?: string;
  outputAdapter?: string;
  languages?: string[];
}

export interface StatusResult {
  ok: boolean;
  path: string;
  exists: boolean;
  isFile: boolean;
  parentExists: boolean;
  runtimeManifest?: RuntimeManifest | null;
}

export function summarizeRelease(body?: string) {
  if (!body) return T.officialNoLog;
  return body
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(0, 8)
    .join("\n");
}

export function formatBytes(bytes?: number) {
  if (!bytes || bytes <= 0) return "-";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / 1024 / 1024).toFixed(1) + " MB";
}

export function pickWindowsAsset(release: GitHubRelease) {
  return (release.assets || []).find((asset) => {
    const name = asset.name.toLowerCase();
    return name.includes("windows") && name.includes("x64") && (name.endsWith(".7z") || name.endsWith(".zip"));
  });
}
