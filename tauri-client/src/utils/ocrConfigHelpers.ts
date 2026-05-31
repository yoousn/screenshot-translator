export const REPO_API = "https://api.github.com/repos/hiroi-sora/PaddleOCR-json/releases/latest";
export const REPO_URL = "https://github.com/hiroi-sora/PaddleOCR-json";

export const T = {
  pageTitle: "OCR 配置",
  pageDesc: "当前版本强制使用客户端本地 PaddleOCR-json 做 OCR 识别；N100 后端只接收文本进行翻译，不再接收图片做云端 OCR。",
  repoTitle: "PaddleOCR-json 运行包说明",
  repoDesc: "这里下载的是可直接运行的 PaddleOCR-json Windows x64 发布包，不再下载 PaddlePaddle/PaddleOCR 源码包。下载后会自动解压到应用本地 OCR 运行目录，删除压缩包，并把 PaddleOCR-json.exe 设置为默认调用路径。",
  localTitle: "本地 OCR 执行配置",
  exePath: "PaddleOCR-json.exe 物理路径",
  exePlaceholder: "留空则优先使用应用数据目录中的 OCR 运行包，其次使用内置 resources/ocr/PaddleOCR-json.exe",
  timeout: "本地 OCR 超时限制 (ms)",
  save: "保存本地 OCR 配置",
  saved: "OCR 配置已保存",
  openDir: "打开所在目录",
  mode: "OCR 模式",
  localOnly: "强制本地",
  status: "可用状态",
  statusUnknown: "未检查",
  statusOk: "可用",
  statusBad: "不可用",
  checkStatus: "手动检查可用状态",
  checkedStatus: "OCR 状态检查完成",
  updateTitle: "PaddleOCR-json 更新",
  check: "手动检查更新",
  downloadFirst: "下载并安装最新版",
  downloadUpdate: "更新并安装最新版",
  officialRepo: "运行包仓库",
  downloadedVersion: "已安装版本",
  latestVersion: "最新版本",
  assetName: "运行包文件",
  lastChecked: "上次检查",
  installDir: "安装目录",
  downloadSize: "下载大小",
  notDownloaded: "未安装",
  notChecked: "未检查",
  hasUpdate: "有更新",
  fullLog: "完整日志",
  openInstallDir: "打开 OCR 所在目录",
  moveInstallDir: "移动 OCR 目录",
  moving: "正在移动 OCR 目录...",
  moved: "OCR 目录已移动",
  checkFirst: "请先检查更新",
  noWindowsAsset: "最新 Release 未找到 Windows x64 .7z 运行包。",
  officialNoLog: "官方 Release 未提供更新说明。",
  checkedLatest: "检测到 PaddleOCR-json 最新版本：",
  downloaded: "已安装 PaddleOCR-json ",
  saveFailed: "保存失败：",
  checkFailed: "检查更新失败：",
  statusFailed: "检查可用状态失败：",
  downloadFailed: "下载/安装失败：",
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

export interface StatusResult {
  ok: boolean;
  path: string;
  exists: boolean;
  isFile: boolean;
  parentExists: boolean;
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
    return name.includes("windows") && name.includes("x64") && name.endsWith(".7z");
  });
}
