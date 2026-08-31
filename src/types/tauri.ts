// 后端 Rust 类型的 TypeScript 镜像
// 字段与 src-tauri/src/{types.rs, commands/*.rs, db.rs} 一一对应

export interface VideoInfo {
  id: string;
  title: string;
  uploader: string;
  duration: number;
  thumbnail: string;
  formats: FormatInfo[];
}

export interface FormatInfo {
  format_id: string;
  ext: string;
  resolution: string;
  fps: number | null;
  vcodec: string;
  acodec: string;
  filesize: number | null;
  filesize_approx: number | null;
  tbr: number | null;
  format_note: string;
}

/** 进度状态联合类型, 与后端 ProgressEvent.status 字符串一一对应 */
export type ProgressStatus = "downloading" | "finished" | "error";

/** 运行时常量, 与 ProgressStatus 联合类型保持一致 (测试断言用) */
export const PROGRESS_STATUSES = ["downloading", "finished", "error"] as const;

/** 类型守卫: 接受任何 unknown, 收窄为合法 ProgressStatus */
export function isProgressStatus(s: unknown): s is ProgressStatus {
  return (
    typeof s === "string" &&
    (PROGRESS_STATUSES as readonly string[]).includes(s)
  );
}

export interface ProgressEvent {
  id: string;
  percent: number;
  /** bytes/s */
  speed: number | null;
  /** seconds */
  eta: number | null;
  downloaded_bytes: number;
  total_bytes: number | null;
  status: ProgressStatus;
  message?: string;
}

export interface CookieInfo {
  domain: string;
  path: string;
  size_bytes: number;
  /** ISO8601 */
  last_modified: string;
}

export interface HistoryEntry {
  id: number;
  url: string;
  title: string;
  save_path: string;
  size_bytes: number | null;
  /** ISO8601 */
  downloaded_at: string;
  status: string;
}

export interface Settings {
  default_save_dir: string;
  default_format_preference: string;
  proxy: string | null;
  cookies: CookieInfo[];
}

/** 热更新 manifest */
export interface UpdateManifest {
  version: string;
  date: string;
  releaseNotes: string;
  msiPath: string;
  msiSizeBytes: number | null;
}

/** 版本检查结果 */
export interface UpdateCheckResult {
  latestVersion: string;
  currentVersion: string;
  updateAvailable: boolean;
  message: string;
  manifest: UpdateManifest | null;
}

/** 下载进度 */
export interface DownloadProgress {
  percent: number;
  downloaded_bytes: number;
  total_bytes: number | null;
  status: "downloading" | "finished" | "error";
  message: string;
}

/** bgutil POT server 状态 (用于 Settings 页面状态指示) */
export interface BgutilStatus {
  /** server 是否正在端口 4416 监听 */
  available: boolean;
  url: string;
  node_found: boolean;
  deno_found: boolean;
  server_dir_found: boolean;
  port_in_use: boolean;
}

// ============ 工具箱类型 ============

export interface PsdLayerInfo {
  index: number;
  name: string;
  top: number;
  left: number;
  width: number;
  height: number;
  visible: boolean;
  outputPath: string | null;
}

export interface PsdResult {
  compositePath: string;
  layers: PsdLayerInfo[];
  layerCount: number;
}

export interface NcmInfo {
  title: string | null;
  artist: string | null;
  album: string | null;
  durationSec: number | null;
  coverData: string | null; // base64
  format: string;
  outputPath: string;
}

export interface WatermarkOptions {
  inputPath: string;
  outputDir: string;
  watermarkType: "text" | "image";
  text?: string;
  logoPath?: string;
  position?: "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center" | "tile" | "diagonal";
  color?: string;
  fontSize?: number;
  opacity?: number;
  scale?: number;
  format?: "png" | "jpeg" | "webp";
}

export interface WatermarkResult {
  outputPath: string;
}
