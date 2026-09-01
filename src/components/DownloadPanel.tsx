import { useEffect, useRef, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  cancelDownload,
  errMsg,
  formatBytes,
  formatDuration,
  listenProgress,
  openParserWindow,
  pickDefaultFormat,
  probeUrl,
  startDownload,
} from "../api/tauri";
import type { FormatInfo, ProgressEvent, VideoInfo } from "../types/tauri";
import { useError } from "./ErrorContext";

interface Props {
  /** 外部传入的 URL (历史"重下"按钮用), 变化时覆盖当前输入 */
  urlSeed?: string | null;
  /** 保存目录初始值 (来自 settings) */
  defaultSaveDir?: string;
}

export function DownloadPanel({ urlSeed, defaultSaveDir = "" }: Props) {
  const { show } = useError();

  const [url, setUrl] = useState("");
  const [info, setInfo] = useState<VideoInfo | null>(null);
  const [selectedFormat, setSelectedFormat] = useState<string>("");
  const [saveDir, setSaveDir] = useState<string>(defaultSaveDir);
  const [probeLoading, setProbeLoading] = useState(false);
  const [downloadId, setDownloadId] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressEvent | null>(null);
  const [downloading, setDownloading] = useState(false);

  // 同步外部 urlSeed (历史"重下" 场景)
  const lastSeedRef = useRef<string | null>(null);
  useEffect(() => {
    if (urlSeed && urlSeed !== lastSeedRef.current) {
      lastSeedRef.current = urlSeed;
      setUrl(urlSeed);
    }
  }, [urlSeed]);

  async function handleProbe() {
    if (!url.trim()) return;
    setProbeLoading(true);
    setInfo(null);
    setProgress(null);
    try {
      const result = await probeUrl(url.trim());
      setInfo(result);
      const best = pickDefaultFormat(result.formats);
      if (best) setSelectedFormat(best.format_id);
    } catch (e) {
      show("解析失败: " + errMsg(e));
    } finally {
      setProbeLoading(false);
    }
  }

  async function handlePickDir() {
    try {
      const picked = await openDialog({
        directory: true,
        multiple: false,
        title: "选择保存目录",
      });
      if (typeof picked === "string") {
        setSaveDir(picked);
      }
    } catch (e) {
      show("选择目录失败: " + errMsg(e));
    }
  }

  async function handleDownload() {
    if (!info || !selectedFormat) return;
    setDownloading(true);
    setProgress({
      id: "",
      percent: 0,
      speed: null,
      eta: null,
      downloaded_bytes: 0,
      total_bytes: null,
      status: "downloading",
    });
    try {
      const id = await startDownload(
        url.trim(),
        selectedFormat,
        saveDir.trim() || null,
        info?.title ?? null,
      );
      setDownloadId(id);
      const unlisten = await listenProgress(id, (evt) => {
        setProgress(evt);
        if (evt.status === "finished" || evt.status === "error") {
          setDownloading(false);
          unlisten();
        }
      });
    } catch (e) {
      show("启动下载失败: " + errMsg(e));
      setDownloading(false);
      setProgress(null);
    }
  }

  async function handleCancel() {
    if (!downloadId) return;
    try {
      await cancelDownload(downloadId);
    } catch (e) {
      show("取消失败: " + errMsg(e));
    } finally {
      setDownloading(false);
    }
  }

  /** 第三方解析服务：在 app 内嵌 webview 打开（不开外部浏览器、不弹 cmd） */
  const WEB_PARSERS = [
    { name: "dlpanda", label: "🐼 dlpanda", build: (u: string) => `https://dlpanda.com/zh-CN?url=${encodeURIComponent(u)}` },
    { name: "snaptik", label: "🎵 snaptik", build: (u: string) => `https://snaptik.app/?url=${encodeURIComponent(u)}` },
    { name: "ssstik",  label: "📥 ssstik",  build: (u: string) => `https://ssstik.io/?url=${encodeURIComponent(u)}` },
  ] as const;

  async function handleWebParse(parserName: "dlpanda" | "snaptik" | "ssstik") {
    if (!url.trim()) {
      show("请先粘贴视频 URL");
      return;
    }
    const p = WEB_PARSERS.find((x) => x.name === parserName);
    if (!p) return;
    const target = p.build(url.trim());
    try {
      await openParserWindow(
        `parser-${parserName}`,
        `Web 解析 — ${p.label}`,
        target,
      );
    } catch (e) {
      show("打开解析窗口失败: " + errMsg(e));
    }
  }

  return (
    <div>
      <div className="card">
        <label className="label">视频 URL</label>
        <div className="row">
          <input
            className="input"
            type="text"
            placeholder="https://..."
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleProbe()}
          />
          <button
            className="btn btn-primary"
            onClick={handleProbe}
            disabled={probeLoading || !url.trim()}
          >
            {probeLoading ? "解析中..." : "解析"}
          </button>
        </div>

        {/* 第三方 Web 解析服务 — 平台反爬时用浏览器绕开 */}
        <div className="mt-3" style={{ display: "flex", flexWrap: "wrap", gap: 6, alignItems: "center" }}>
          <span style={{ fontSize: 11, color: "var(--text-dim)", marginRight: 4 }}>
            🛟 反爬过不去？用浏览器解析：
          </span>
          {WEB_PARSERS.map((p) => (
            <button
              key={p.name}
              className="btn btn-sm"
              onClick={() => handleWebParse(p.name as "dlpanda" | "snaptik" | "ssstik")}
              disabled={!url.trim()}
              style={{ fontSize: 11, padding: "3px 8px" }}
            >
              {p.label}
            </button>
          ))}
        </div>
      </div>

      <div className="card">
        <label className="label">保存目录</label>
        <div className="row">
          <input
            className="input"
            type="text"
            placeholder="留空使用默认目录"
            value={saveDir}
            onChange={(e) => setSaveDir(e.target.value)}
          />
          <button className="btn" onClick={handlePickDir}>
            浏览...
          </button>
        </div>
        {defaultSaveDir && !saveDir && (
          <div className="muted mt-2">默认: {defaultSaveDir}</div>
        )}
      </div>

      {info && (
        <div className="card">
          <h3 className="card-title">{info.title}</h3>
          <div className="muted">
            {info.uploader && <span>{info.uploader} · </span>}
            <span>{formatDuration(info.duration)}</span>
          </div>

          <div className="field">
            <label className="label">选格式</label>
            <FormatTable
              formats={info.formats}
              selected={selectedFormat}
              onSelect={setSelectedFormat}
            />
          </div>

          <div className="row mt-4">
            <button
              className="btn btn-primary"
              onClick={handleDownload}
              disabled={!selectedFormat || downloading}
            >
              {downloading ? "下载中..." : "下载"}
            </button>
            <button
              className="btn"
              onClick={handleCancel}
              disabled={!downloading}
            >
              取消
            </button>
          </div>

          {progress && <ProgressView progress={progress} />}
        </div>
      )}
    </div>
  );
}

function FormatTable({
  formats,
  selected,
  onSelect,
}: {
  formats: FormatInfo[];
  selected: string;
  onSelect: (id: string) => void;
}) {
  const show = formats.slice(0, 20);
  return (
    <table className="table">
      <thead>
        <tr>
          <th>ID</th>
          <th>分辨率</th>
          <th>编码</th>
          <th>大小</th>
          <th className="col-check"></th>
        </tr>
      </thead>
      <tbody>
        {show.map((f) => {
          const isSel = selected === f.format_id;
          const codec =
            f.vcodec && f.vcodec !== "none"
              ? f.vcodec
              : f.acodec && f.acodec !== "none"
                ? "🎵 " + f.acodec
                : "-";
          return (
            <tr
              key={f.format_id}
              className={isSel ? "clickable selected" : "clickable"}
              onClick={() => onSelect(f.format_id)}
            >
              <td>{f.format_id}</td>
              <td>{f.resolution || "-"}</td>
              <td>{codec}</td>
              <td>{formatBytes(f.filesize || f.filesize_approx)}</td>
              <td>{isSel && "✓"}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

function ProgressView({ progress }: { progress: ProgressEvent }) {
  return (
    <div className="mt-4">
      <div className="progress-meta">
        <span>
          {progress.status === "downloading" &&
            `下载中 ${progress.percent.toFixed(1)}%`}
          {progress.status === "finished" && "✅ 下载完成"}
          {progress.status === "error" && `❌ ${progress.message ?? "失败"}`}
        </span>
        <span>
          {formatBytes(progress.downloaded_bytes)}
          {progress.total_bytes ? ` / ${formatBytes(progress.total_bytes)}` : ""}
          {progress.speed ? ` · ${formatBytes(progress.speed)}/s` : ""}
          {progress.eta != null ? ` · 剩 ${progress.eta}s` : ""}
        </span>
      </div>
      <div className="progress">
        <div
          className="progress-bar"
          style={{ width: `${Math.max(0, Math.min(100, progress.percent))}%` }}
        />
      </div>
    </div>
  );
}
