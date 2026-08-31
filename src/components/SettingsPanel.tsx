import { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  checkUpdate,
  autoInstallAndRestart,
  errMsg,
  formatBytes,
  getAppDataDir,
  getBgutilStatus,
  getSettings,
  listenDownloadProgress,
  setSetting,
  type DownloadProgress,
} from "../api/tauri";
import type { BgutilStatus, Settings, UpdateCheckResult } from "../types/tauri";
import { useError } from "./ErrorContext";

/** bgutil POT server 状态指示器 */
function BgutilStatus() {
  const [status, setStatus] = useState<BgutilStatus | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const s = await getBgutilStatus();
        if (!cancelled) setStatus(s);
      } catch (e) {
        // 后端可能没暴露这个命令 (旧版本)
        if (!cancelled) setStatus(null);
      }
    };
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  if (!status) {
    return (
      <div className="field">
        <label className="label">YouTube 反爬 (bgutil)</label>
        <div className="muted">未检测到 bgutil POT server</div>
      </div>
    );
  }

  const dot = status.available ? "🟢" : status.server_dir_found && status.node_found ? "🟡" : "🔴";
  const txt = status.available
    ? `运行中 (${status.url})`
    : status.server_dir_found && status.node_found
      ? "未启动 (下次启动时尝试)"
      : "未配置 (需要 node.exe + bgutil/server/build/main.js)";

  return (
    <div className="field">
      <label className="label">YouTube 反爬 (bgutil POT server)</label>
      <div className="row" style={{ gap: 8, alignItems: "center" }}>
        <span style={{ fontSize: 14 }}>{dot}</span>
        <span style={{ fontSize: 13 }}>{txt}</span>
      </div>
      <div className="muted mt-2">
        状态: node {status.node_found ? "✓" : "✗"} / deno {status.deno_found ? "✓" : "✗"} /{" "}
        bgutil server 目录 {status.server_dir_found ? "✓" : "✗"} / 端口 {status.port_in_use ? "占用" : "空闲"}
      </div>
      <div className="muted mt-2">
        部分视频可能仍被拒绝 — YouTube 对部分老/热门视频检测更严,需配合 Cookies。
      </div>
    </div>
  );
}

/** 热更新检查器 */
function UpdateChecker() {
  const { show } = useError();
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [result, setResult] = useState<UpdateCheckResult | null>(null);
  const [appDataDir, setAppDataDir] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    getAppDataDir().then(setAppDataDir).catch(() => {});
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listenDownloadProgress((p) => {
      setProgress(p);
      if (p.status === "finished" || p.status === "error") {
        setInstalling(false);
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  async function handleCheck() {
    setChecking(true);
    setResult(null);
    setProgress(null);
    try {
      const r = await checkUpdate();
      setResult(r);
    } catch (e) {
      show("检查更新失败: " + errMsg(e));
    } finally {
      setChecking(false);
    }
  }

  async function handleInstall() {
    if (!result?.manifest) return;
    setInstalling(true);
    setProgress({
      percent: 0,
      downloaded_bytes: 0,
      total_bytes: result.manifest.msiSizeBytes ?? null,
      status: "downloading",
      message: "正在准备更新...",
    });
    try {
      // 一键自动化：下载 → 静默安装 → 退出旧版 → 启动新版
      await autoInstallAndRestart(result.manifest.msiPath);
    } catch (e) {
      show("自动更新失败: " + errMsg(e));
      setInstalling(false);
      setProgress(null);
    }
  }

  function copyPath() {
    navigator.clipboard.writeText(appDataDir).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  // 是否在下载中（HTTP 路径会有明显进度）
  const isDownloading = progress?.status === "downloading" && installing;
  const isDone = progress?.status === "finished" && !installing;
  const isHttp =
    result?.manifest?.msiPath?.startsWith("http://") ||
    result?.manifest?.msiPath?.startsWith("https://");

  const statusColor = !result
    ? undefined
    : result.updateAvailable
      ? "#34d399"
      : result.latestVersion
        ? "#fb923c"
        : "#9ca3af";

  const statusLabel = !result
    ? undefined
    : result.updateAvailable
      ? `发现新版本 v${result.latestVersion}`
      : result.latestVersion
        ? `已是最新 v${result.currentVersion}`
        : "未检测到更新";

  return (
    <div className="field">
      <label className="label">热更新</label>

      <div className="row" style={{ gap: 8, marginBottom: 8 }}>
        <button
          className="btn"
          onClick={handleCheck}
          disabled={checking || installing}
        >
          {checking ? "检查中..." : "检查更新"}
        </button>

        {result && (
          <button
            className="btn btn-primary"
            onClick={handleInstall}
            disabled={!result.updateAvailable || installing}
          >
            {installing
              ? "下载/安装/重启中..."
              : result.updateAvailable
                ? "🚀 立即更新并自动重启"
                : "已是最新"}
          </button>
        )}
      </div>

      {/* 进度条 */}
      {isDownloading && (
        <div style={{ marginBottom: 8 }}>
          <div style={{ display: "flex", justifyContent: "space-between", fontSize: 12, color: "#9ca3af", marginBottom: 4 }}>
            <span>{isHttp ? "下载 MSI 中..." : "正在启动安装程序..."}</span>
            <span>{progress?.percent.toFixed(1)}%</span>
          </div>
          <div
            style={{
              height: 6,
              borderRadius: 3,
              background: "#2a2a3e",
              overflow: "hidden",
            }}
          >
            <div
              style={{
                height: "100%",
                width: `${progress?.percent ?? 0}%`,
                background: "linear-gradient(90deg, #6366f1, #8b5cf6)",
                borderRadius: 3,
                transition: "width 0.3s ease",
              }}
            />
          </div>
          {progress?.message && (
            <div style={{ fontSize: 11, color: "#9ca3af", marginTop: 3 }}>
              {progress.message}
            </div>
          )}
        </div>
      )}

      {/* 安装完成提示 */}
      {isDone && (
        <div
          style={{
            background: "#1a2e1a",
            borderRadius: 6,
            padding: "8px 12px",
            fontSize: 12,
            color: "#4ade80",
            marginBottom: 6,
          }}
        >
          ✅ {progress?.message ?? "更新完成"}
          <br />
          <span style={{ color: "#9ca3af" }}>
            旧版正在退出，新版安装完成后会自动启动...
          </span>
        </div>
      )}

      {result && (
        <div
          style={{
            background: "#1a1a2e",
            borderRadius: 6,
            padding: "8px 12px",
            fontSize: 12,
            color: statusColor,
            marginBottom: 6,
          }}
        >
          {statusLabel}
          <br />
          <span style={{ color: "#9ca3af" }}>{result.message}</span>
        </div>
      )}

      {result?.manifest && result.updateAvailable && (
        <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 4 }}>
          <div>📦 {result.manifest.releaseNotes}</div>
          {result.manifest.msiSizeBytes && (
            <div style={{ marginTop: 2 }}>
              大小: {formatBytes(result.manifest.msiSizeBytes)}
            </div>
          )}
          <div style={{ marginTop: 2, wordBreak: "break-all" }}>
            路径:{" "}
            <span style={{ color: isHttp ? "#fbbf24" : "#93c5fd" }}>
              {result.manifest.msiPath}
            </span>
            {isHttp && (
              <span style={{ marginLeft: 6, color: "#6366f1" }}>（远程 URL，将自动下载）</span>
            )}
          </div>
        </div>
      )}

      <div className="muted mt-2" style={{ fontSize: 11, lineHeight: 1.6 }}>
        <div>
          manifest 文件路径：
          <code style={{ color: "#93c5fd" }}>
            {appDataDir || "(加载中...)"}\update-manifest.json
          </code>
          <button
            className="btn"
            style={{ marginLeft: 6, fontSize: 11, padding: "1px 6px" }}
            onClick={copyPath}
          >
            {copied ? "已复制 ✓" : "复制"}
          </button>
        </div>
        <div style={{ marginTop: 2 }}>
          将 update-manifest.json 放入上述目录即可热更新（无需访问 GitHub）。
          {isHttp ? "MSI 路径支持 HTTP/HTTPS URL，远程自动下载。" : "MSI 路径支持本地路径或 UNC 网络路径。"}
        </div>
      </div>
    </div>
  );
}

/** settings key 集中在这里, 避免拼写错误 */
const KEY = {
  saveDir: "default_save_dir",
  formatPref: "default_format_preference",
  proxy: "proxy",
} as const;

export function SettingsPanel({
  onSettingsLoaded,
}: {
  /** 把保存目录回传给 App, 让 DownloadPanel 显示默认值 */
  onSettingsLoaded?: (saveDir: string) => void;
}) {
  const { show } = useError();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saveDir, setSaveDir] = useState("");
  const [formatPref, setFormatPref] = useState("bestvideo+bestaudio/best");
  const [proxy, setProxy] = useState("");

  useEffect(() => {
    (async () => {
      try {
        const s = await getSettings();
        hydrate(s);
      } catch (e) {
        show("加载设置失败: " + errMsg(e));
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  function hydrate(s: Settings) {
    setSaveDir(s.default_save_dir ?? "");
    setFormatPref(s.default_format_preference ?? "bestvideo+bestaudio/best");
    setProxy(s.proxy ?? "");
    onSettingsLoaded?.(s.default_save_dir ?? "");
  }

  async function handlePickDir() {
    try {
      const picked = await openDialog({ directory: true, multiple: false });
      if (typeof picked === "string") setSaveDir(picked);
    } catch (e) {
      show("选择目录失败: " + errMsg(e));
    }
  }

  async function handleSave() {
    setSaving(true);
    try {
      await setSetting(KEY.saveDir, saveDir);
      await setSetting(KEY.formatPref, formatPref);
      // proxy 为空时存空串, 后端是 String 不是 Option, 保持一致
      await setSetting(KEY.proxy, proxy);
      show("✅ 设置已保存");
      onSettingsLoaded?.(saveDir);
    } catch (e) {
      show("保存失败: " + errMsg(e));
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <div>
        <div className="card muted">加载中...</div>
      </div>
    );
  }

  return (
    <div>
      <div className="card">
        <h3 className="card-title">下载设置</h3>

        <div className="field">
          <label className="label">下载目录</label>
          <div className="row">
            <input
              className="input"
              type="text"
              placeholder="例如 D:\Downloads\video"
              value={saveDir}
              onChange={(e) => setSaveDir(e.target.value)}
            />
            <button className="btn" onClick={handlePickDir}>
              浏览...
            </button>
          </div>
        </div>

        <div className="field">
          <label className="label">默认格式偏好 (yt-dlp format string)</label>
          <input
            className="input"
            type="text"
            value={formatPref}
            onChange={(e) => setFormatPref(e.target.value)}
            placeholder="bestvideo+bestaudio/best"
          />
          <div className="muted mt-2">
            常用: <code>best</code> / <code>bestvideo+bestaudio</code> /{" "}
            <code>bv*+ba/b</code>
          </div>
        </div>

        <div className="field">
          <label className="label">代理 (可选)</label>
          <input
            className="input"
            type="text"
            value={proxy}
            onChange={(e) => setProxy(e.target.value)}
            placeholder="http://127.0.0.1:7890  或  socks5://..."
          />
        </div>

        <BgutilStatus />

        <UpdateChecker />
      </div>

      <div className="row">
        <button
          className="btn btn-primary"
          onClick={handleSave}
          disabled={saving}
        >
          {saving ? "保存中..." : "保存"}
        </button>
      </div>
    </div>
  );
}
