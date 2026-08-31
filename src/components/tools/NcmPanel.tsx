import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { convertNcm, errMsg, formatDuration } from "../../api/tauri";
import type { NcmInfo } from "../../types/tauri";
import { useError } from "../ErrorContext";

export function NcmPanel() {
  const { show } = useError();
  const [processing, setProcessing] = useState(false);
  const [result, setResult] = useState<NcmInfo | null>(null);
  const [outputDir, setOutputDir] = useState("");

  async function pickFile() {
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "网易云音乐", extensions: ["ncm"] }],
      });
      if (typeof picked === "string") {
        await processFile(picked);
      }
    } catch (e) {
      show("选择文件失败: " + errMsg(e));
    }
  }

  async function pickOutputDir() {
    try {
      const picked = await openDialog({ directory: true, multiple: false });
      if (typeof picked === "string") {
        setOutputDir(picked);
      }
    } catch (e) {
      show("选择目录失败: " + errMsg(e));
    }
  }

  async function processFile(filePath: string) {
    if (!outputDir) {
      show("请先选择输出目录");
      return;
    }
    setProcessing(true);
    setResult(null);
    try {
      const r = await convertNcm(filePath, outputDir);
      setResult(r);
    } catch (e) {
      show("转换失败: " + errMsg(e));
    } finally {
      setProcessing(false);
    }
  }

  return (
    <div className="card">
      <h3 className="card-title">🎵 NCM → MP3/FLAC</h3>
      <p className="muted" style={{ fontSize: 12, marginBottom: 16 }}>
        网易云音乐 .ncm 格式转换，保留封面和歌词
      </p>

      <div className="field">
        <label className="label">输出目录</label>
        <div className="row">
          <input
            className="input"
            type="text"
            placeholder="选择输出目录..."
            value={outputDir}
            onChange={(e) => setOutputDir(e.target.value)}
          />
          <button className="btn" onClick={pickOutputDir}>浏览</button>
        </div>
      </div>

      <button
        className="btn btn-primary"
        style={{ width: "100%", marginBottom: 16 }}
        onClick={pickFile}
        disabled={processing || !outputDir}
      >
        {processing ? "转换中..." : "🎵 选择 NCM 文件"}
      </button>

      {result && (
        <div style={{ background: "#1a1a2e", borderRadius: 6, padding: "12px 14px" }}>
          <div style={{ color: "#4ade80", fontSize: 13, marginBottom: 8 }}>
            ✅ 转换完成
          </div>

          {result.coverData && (
            <div style={{ marginBottom: 10 }}>
              <img
                src={`data:image/jpeg;base64,${result.coverData}`}
                alt="cover"
                style={{ width: 80, height: 80, objectFit: "cover", borderRadius: 6 }}
              />
            </div>
          )}

          <div style={{ fontSize: 13, color: "#d1d5db", lineHeight: 1.8 }}>
            {result.title && <div>🎤 {result.title}</div>}
            {result.artist && <div>👤 {result.artist}</div>}
            {result.album && <div>💿 {result.album}</div>}
            {result.durationSec && <div>⏱ {formatDuration(result.durationSec)}</div>}
            <div style={{ marginTop: 4, color: "#6366f1" }}>📦 {result.format.toUpperCase()}</div>
          </div>

          <div style={{ fontSize: 11, color: "#9ca3af", marginTop: 8, wordBreak: "break-all" }}>
            输出: <span style={{ color: "#93c5fd" }}>{result.outputPath}</span>
          </div>
        </div>
      )}
    </div>
  );
}
