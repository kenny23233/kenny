import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { extractPsdLayers, errMsg } from "../../api/tauri";
import type { PsdResult, PsdLayerInfo } from "../../types/tauri";
import { useError } from "../ErrorContext";

export function PsdPanel() {
  const { show } = useError();
  const [processing, setProcessing] = useState(false);
  const [result, setResult] = useState<PsdResult | null>(null);
  const [outputDir, setOutputDir] = useState("");

  async function pickFile() {
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "PSD/PSB", extensions: ["psd", "psb"] }],
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
      const r = await extractPsdLayers(filePath, outputDir);
      setResult(r);
    } catch (e) {
      show("提取失败: " + errMsg(e));
    } finally {
      setProcessing(false);
    }
  }

  return (
    <div className="card">
      <h3 className="card-title">🎨 PSD/PSB 图层提取</h3>
      <p className="muted" style={{ fontSize: 12, marginBottom: 16 }}>
        拖入 PSD/PSB 文件，自动提取合并图像和图层列表
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
        {processing ? "处理中..." : "📂 选择 PSD/PSB 文件"}
      </button>

      {result && (
        <div>
          <div className="result-item" style={{ background: "#1a1a2e", borderRadius: 6, padding: "10px 14px", marginBottom: 8 }}>
            <div style={{ color: "#4ade80", fontSize: 13 }}>
              ✅ 提取完成 — {result.layerCount} 个图层
            </div>
            <div style={{ fontSize: 12, color: "#9ca3af", marginTop: 4 }}>
              复合图: <span style={{ color: "#93c5fd" }}>{result.compositePath}</span>
            </div>
          </div>

          <div style={{ maxHeight: 200, overflowY: "auto" }}>
            {result.layers.map((l: PsdLayerInfo) => (
              <div key={l.index} style={{ fontSize: 12, padding: "4px 0", borderBottom: "1px solid #2a2a3e", color: "#d1d5db" }}>
                <span style={{ color: "#9ca3af" }}>#{l.index}</span> {l.name} — {l.width}×{l.height}px
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
