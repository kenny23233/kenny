// 集中封装所有后端 invoke 调用, 组件直接 import 函数而不是裸 invoke
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  BgutilStatus,
  CookieInfo,
  FormatInfo,
  HistoryEntry,
  ProgressEvent,
  Settings,
  UpdateCheckResult,
  VideoInfo,
  PsdResult,
  NcmInfo,
  WatermarkOptions,
  WatermarkResult,
} from "../types/tauri";
export type { UpdateCheckResult, UpdateManifest } from "../types/tauri";
// re-export 供需要 ProgressStatus 字面量的组件使用
export type { ProgressStatus } from "../types/tauri";

// ---- 错误归一: invoke 抛错时可能是 string / Error / { message } 三种形态 ----
export function errMsg(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

// ---- 下载 ----
export function probeUrl(url: string): Promise<VideoInfo> {
  return invoke<VideoInfo>("probe_url", { url });
}

export function startDownload(
  url: string,
  formatId: string,
  saveDir: string | null,
  title: string | null,
): Promise<string> {
  return invoke<string>("start_download", { url, formatId, saveDir, title });
}

export function cancelDownload(downloadId: string): Promise<void> {
  return invoke<void>("cancel_download", { downloadId });
}

export function listenProgress(
  downloadId: string,
  onEvent: (e: ProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<ProgressEvent>(`download://${downloadId}`, (evt) =>
    onEvent(evt.payload),
  );
}

// ---- Cookies ----
export function importCookies(filePath: string): Promise<string> {
  return invoke<string>("import_cookies", { filePath });
}

export function listCookies(): Promise<CookieInfo[]> {
  return invoke<CookieInfo[]>("list_cookies");
}

export function deleteCookies(domain: string): Promise<void> {
  return invoke<void>("delete_cookies", { domain });
}

// ---- 历史 ----
export function listHistory(limit = 100): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("list_history", { limit });
}

export function deleteHistory(id: number): Promise<void> {
  return invoke<void>("delete_history", { id });
}

// ---- 设置 ----
export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("set_setting", { key, value });
}

export function checkBinaries(): Promise<void> {
  return invoke("check_binaries");
}

export function getBgutilStatus(): Promise<BgutilStatus> {
  return invoke<BgutilStatus>("get_bgutil_status");
}

// ---- 热更新 ----

/** 检查更新（自动读取 %APPDATA%\video-toolbox\update-manifest.json） */
export function checkUpdate(): Promise<UpdateCheckResult> {
  return invoke<UpdateCheckResult>("check_update");
}

/** 从指定路径读取 manifest 并检查更新（内网文件服务器场景） */
export function checkUpdateWithManifest(manifestPath: string): Promise<UpdateCheckResult> {
  return invoke<UpdateCheckResult>("check_update_with_manifest", { manifestPath });
}

/** 打开 MSI 文件所在目录（直接双击安装） */
export function openMsiFolder(msiPath: string): Promise<void> {
  return invoke<void>("open_msi_folder", { msiPath });
}

/** 获取 app data 目录路径（显示给用户放 manifest） */
export function getAppDataDir(): Promise<string> {
  return invoke<string>("get_app_data_dir");
}

/** 下载 MSI 并启动安装程序（HTTP URL 自动下载，本地路径直接启动） */
export function downloadAndInstall(msiPath: string): Promise<void> {
  return invoke<void>("download_and_install", { msiPath });
}

/** 全自动热更新：下载 → 静默安装 → 退出旧版 → 自动启动新版（一步到位） */
export function autoInstallAndRestart(msiPath: string): Promise<void> {
  return invoke<void>("auto_install_and_restart", { msiPath });
}

/** 下载进度事件类型 */
export interface DownloadProgress {
  percent: number;
  downloaded_bytes: number;
  total_bytes: number | null;
  status: "downloading" | "finished" | "error";
  message: string;
}

/** 监听热更新下载进度事件 */
export function listenDownloadProgress(
  onEvent: (e: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<DownloadProgress>("updater://download-progress", (evt) =>
    onEvent(evt.payload),
  );
}

// ---- 格式化工具 (供 UI 复用) ----
export function formatDuration(s: number | null | undefined): string {
  if (!s) return "-";
  const sec = Math.max(0, Math.floor(s));
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const ss = sec % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, "0")}:${String(ss).padStart(2, "0")}`;
  return `${m}:${String(ss).padStart(2, "0")}`;
}

export function formatBytes(n: number | null | undefined): string {
  if (!n) return "-";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(1)} ${u[i]}`;
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "-";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}

/** 选默认格式: 优先 best* / 否则取第一个有 video 的 / 否则第一个 */
export function pickDefaultFormat(formats: FormatInfo[]): FormatInfo | null {
  if (!formats?.length) return null;
  const best = formats.find((f) => /best/i.test(f.format_note));
  if (best) return best;
  const withVideo = formats.find((f) => f.vcodec && f.vcodec !== "none");
  return withVideo ?? formats[0];
}

/** URL 短显示: 截取 host + 路径前 30 字符 */
export function shortUrl(url: string, max = 48): string {
  if (!url) return "";
  if (url.length <= max) return url;
  return url.slice(0, max - 1) + "…";
}

// ============ 工具箱 API ============

/** PSD/PSB 图层提取 */
export function extractPsdLayers(
  psdPath: string,
  outputDir: string,
): Promise<PsdResult> {
  return invoke<PsdResult>("extract_psd_layers", { psdPath, outputDir });
}

/** NCM → MP3/FLAC 转换 */
export function convertNcm(
  ncmPath: string,
  outputDir: string,
): Promise<NcmInfo> {
  return invoke<NcmInfo>("convert_ncm", { ncmPath, outputDir });
}

/** 图片水印 */
export function applyImageWatermark(
  options: WatermarkOptions,
): Promise<WatermarkResult> {
  return invoke<WatermarkResult>("apply_image_watermark", { options });
}

/** 读取图片为 data URL（用于前端直接显示本地图片） */
export function readImageAsDataUrl(path: string): Promise<string> {
  return invoke<string>("read_image_as_data_url", { path });
}
