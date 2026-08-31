import { useState, useEffect, useRef, useCallback } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { applyImageWatermark, errMsg, readImageAsDataUrl } from "../../api/tauri";
import type { WatermarkOptions } from "../../types/tauri";
import { useError } from "../ErrorContext";

type WMStyle = "single" | "tile" | "diagonal";
type PosIdx = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8; // 3x3 grid: 左上→右下

const FONTS = [
  "苹方 / 微软雅黑",
  "思源黑体 / Noto Sans",
  "Helvetica / Arial",
  "Georgia / 宋体",
  "Impact / 黑体",
];

const CHECKER_BG = `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'%3E%3Crect width='12' height='12' fill='%23374151'/%3E%3Crect x='12' y='12' width='12' height='12' fill='%23374151'/%3E%3Crect x='12' width='12' height='12' fill='%231f2937'/%3E%3Crect y='12' width='12' height='12' fill='%231f2937'/%3E%3C/svg%3E")`;

export function WmPanel() {
  const { show } = useError();
  const [inputPath, setInputPath] = useState("");
  const [logoPath, setLogoPath] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [outputName, setOutputName] = useState("");
  const [wmType, setWmType] = useState<"text" | "image">("text");
  const [text, setText] = useState("© 原创内容");
  const [font, setFont] = useState(FONTS[0]);
  const [fontSize, setFontSize] = useState(64);
  const [color, setColor] = useState("#FFFFFF");
  const [style, setStyle] = useState<WMStyle>("single");
  const [posIdx, setPosIdx] = useState<PosIdx>(8); // 默认右下角
  const [posX, setPosX] = useState(50);
  const [posY, setPosY] = useState(50);
  const [opacity, setOpacity] = useState(50);
  const [logoScale, setLogoScale] = useState(15);
  const [dragging, setDragging] = useState(false);
  const [processing, setProcessing] = useState(false);
  const [result, setResult] = useState<string | null>(null);

  const [inputSrc, setInputSrc] = useState<string>("");
  const [logoSrc, setLogoSrc] = useState<string>("");
  const [tab, setTab] = useState<"effect" | "original">("effect");
  const [draggingInput, setDraggingInput] = useState(false);
  const previewRef = useRef<HTMLDivElement>(null);
  const watermarkRef = useRef<HTMLDivElement>(null);
  const dragOffsetRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  // 加载图片为 data URL
  useEffect(() => {
    if (!inputPath) { setInputSrc(""); return; }
    let cancelled = false;
    (async () => {
      try {
        const url = await readImageAsDataUrl(inputPath);
        if (!cancelled) setInputSrc(url);
      } catch (e) {
        if (!cancelled) setInputSrc("");
      }
    })();
    return () => { cancelled = true; };
  }, [inputPath]);

  useEffect(() => {
    if (!logoPath) { setLogoSrc(""); return; }
    let cancelled = false;
    (async () => {
      try {
        const url = await readImageAsDataUrl(logoPath);
        if (!cancelled) setLogoSrc(url);
      } catch (e) {
        if (!cancelled) setLogoSrc("");
      }
    })();
    return () => { cancelled = true; };
  }, [logoPath]);

  // Tauri 拖拽监听
  useEffect(() => {
    let unlistenInput: (() => void) | undefined;
    let unlistenLogo: (() => void) | undefined;

    (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlistenInput = await listen<{ paths: string[] }>("tauri://drag-drop", (e) => {
        setDraggingInput(false);
        const p = e.payload.paths[0];
        if (p && isImageFile(p)) setInputPath(p);
      });
      unlistenLogo = await listen<{ paths: string[] }>("tauri://drag-drop", (e) => {
        const p = e.payload.paths[0];
        if (p && isImageFile(p)) setLogoPath(p);
      });
    })();

    return () => { unlistenInput?.(); unlistenLogo?.(); };
  }, []);

  function isImageFile(path: string): boolean {
    const ext = path.split(".").pop()?.toLowerCase() ?? "";
    return ["png", "jpg", "jpeg", "webp", "bmp", "gif"].includes(ext);
  }

  // 3x3 网格 → 百分比位置
  const GRID_POS: { x: number; y: number }[] = [
    { x: 5, y: 5 },     // 0: 左上
    { x: 50, y: 5 },    // 1: 上中
    { x: 95, y: 5 },    // 2: 右上
    { x: 5, y: 50 },    // 3: 左中
    { x: 50, y: 50 },   // 4: 正中
    { x: 95, y: 50 },   // 5: 右中
    { x: 5, y: 95 },    // 6: 左下
    { x: 50, y: 95 },   // 7: 下中
    { x: 95, y: 95 },   // 8: 右下
  ];

  function handleGridClick(idx: PosIdx) {
    setPosIdx(idx);
    setPosX(GRID_POS[idx].x);
    setPosY(GRID_POS[idx].y);
  }

  // 在预览上拖动水印
  const handleWatermarkMouseDown = useCallback((e: React.MouseEvent) => {
    if (!inputSrc) return;
    e.preventDefault();
    e.stopPropagation();
    setDragging(true);
    const preview = previewRef.current;
    if (preview) {
      const rect = preview.getBoundingClientRect();
      dragOffsetRef.current = { x: e.clientX - rect.left, y: e.clientY - rect.top };
    }
  }, [inputSrc]);

  useEffect(() => {
    if (!dragging) return;
    const onMove = (e: MouseEvent) => {
      const preview = previewRef.current;
      if (!preview) return;
      const rect = preview.getBoundingClientRect();
      const x = ((e.clientX - rect.left) / rect.width) * 100;
      const y = ((e.clientY - rect.top) / rect.height) * 100;
      setPosX(Math.max(0, Math.min(100, x)));
      setPosY(Math.max(0, Math.min(100, y)));
    };
    const onUp = () => setDragging(false);
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    return () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
  }, [dragging]);

  // 选择输出目录
  async function pickOutputDir() {
    try {
      const picked = await openDialog({ directory: true, multiple: false });
      if (typeof picked === "string") setOutputDir(picked);
    } catch (e) {
      show("选择目录失败: " + errMsg(e));
    }
  }

  // 处理
  async function handleApply() {
    if (!inputPath) { show("请先拖入图片"); return; }
    if (!outputDir) { show("请选择输出目录"); return; }
    if (wmType === "image" && !logoPath) { show("请先拖入 Logo"); return; }

    setProcessing(true);
    setResult(null);
    try {
      // 把 3x3 位置转成 backend 的 position
      const position = (() => {
        if (style === "tile") return "tile";
        if (style === "diagonal") return "diagonal";
        // 根据 posIdx 转成对应位置
        if (posIdx === 0) return "top-left";
        if (posIdx === 2) return "top-right";
        if (posIdx === 4) return "center";
        if (posIdx === 6) return "bottom-left";
        if (posIdx === 8) return "bottom-right";
        // 其他位置用百分比描述
        if (posY < 33) return posX < 33 ? "top-left" : posX > 66 ? "top-right" : "top-center";
        if (posY > 66) return posX < 33 ? "bottom-left" : posX > 66 ? "bottom-right" : "bottom-center";
        return posX < 33 ? "center-left" : posX > 66 ? "center-right" : "center";
      })() as WatermarkOptions["position"];

      const opts: WatermarkOptions = {
        inputPath, outputDir, watermarkType: wmType,
        text: wmType === "text" ? text : undefined,
        logoPath: wmType === "image" ? logoPath : undefined,
        position,
        color: wmType === "text" ? `${parseInt(color.slice(1,3),16)},${parseInt(color.slice(3,5),16)},${parseInt(color.slice(5,7),16)},${Math.round(opacity * 2.55)}` : undefined,
        fontSize: wmType === "text" ? fontSize : undefined,
        opacity: opacity / 100,
        scale: wmType === "image" ? logoScale / 100 : undefined,
        format: "png",
      };
      const r = await applyImageWatermark(opts);
      setResult(r.outputPath);
    } catch (e) {
      show("水印失败: " + errMsg(e));
    } finally {
      setProcessing(false);
    }
  }

  return (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 320px", height: "calc(100vh - 80px)", gap: 0 }}>
      {/* ===== 左侧：预览区 ===== */}
      <div style={{ display: "flex", flexDirection: "column", padding: 16, overflow: "hidden" }}>
        {/* Tab 切换 */}
        <div style={{ display: "flex", gap: 2, marginBottom: 12 }}>
          <button
            onClick={() => setTab("effect")}
            style={{
              padding: "6px 16px",
              border: "none",
              borderRadius: "6px 6px 0 0",
              background: tab === "effect" ? "#3b82f6" : "rgba(30,30,50,0.5)",
              color: tab === "effect" ? "#fff" : "#9ca3af",
              fontSize: 13,
              fontWeight: tab === "effect" ? 500 : 400,
              cursor: "pointer",
            }}
          >
            水印效果
          </button>
          <button
            onClick={() => setTab("original")}
            style={{
              padding: "6px 16px",
              border: "none",
              borderRadius: "6px 6px 0 0",
              background: tab === "original" ? "#3b82f6" : "rgba(30,30,50,0.5)",
              color: tab === "original" ? "#fff" : "#9ca3af",
              fontSize: 13,
              fontWeight: tab === "original" ? 500 : 400,
              cursor: "pointer",
            }}
          >
            原图
          </button>
        </div>

        {/* 预览容器 */}
        <div
          ref={previewRef}
          onDragOver={(e) => { e.preventDefault(); setDraggingInput(true); }}
          onDragLeave={() => setDraggingInput(false)}
          onDrop={(e) => {
            e.preventDefault();
            setDraggingInput(false);
            const file = e.dataTransfer?.files[0];
            if (file) {
              const path = (file as File & { path?: string }).path;
              if (path && isImageFile(path)) setInputPath(path);
            }
          }}
          onClick={async (e) => {
            // 空白处点击 = 选择文件
            if (e.target === e.currentTarget && !inputSrc) {
              try {
                const picked = await openDialog({ multiple: false, filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "bmp", "gif"] }] });
                if (typeof picked === "string") setInputPath(picked);
              } catch (e) { /* ignore */ }
            }
          }}
          style={{
            flex: 1,
            position: "relative",
            background: tab === "effect" ? "#0a0a14" : `${CHECKER_BG} #0a0a14`,
            backgroundSize: tab === "effect" ? undefined : "24px 24px",
            borderRadius: 8,
            border: `2px dashed ${draggingInput ? "#6366f1" : "transparent"}`,
            overflow: "hidden",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            minHeight: 0,
          }}
        >
          {!inputSrc ? (
            <div style={{ textAlign: "center", color: "#9ca3af", userSelect: "none", cursor: "pointer" }}>
              <div style={{ fontSize: 56, marginBottom: 12 }}>🖼️</div>
              <div style={{ fontSize: 16, fontWeight: 500, color: "#d1d5db" }}>拖拽图片到这里,或点击选择(可一次多张)</div>
              <div style={{ fontSize: 12, marginTop: 8 }}>支持 PNG / JPG / WebP / GIF 等常见格式,全部本地处理</div>
            </div>
          ) : (
            <div style={{ position: "relative", display: "inline-block", maxWidth: "100%", maxHeight: "100%" }}>
              <img
                src={tab === "original" ? inputSrc : inputSrc}
                alt=""
                style={{ maxWidth: "100%", maxHeight: "calc(100vh - 180px)", display: "block", verticalAlign: "top" }}
              />

              {tab === "effect" && wmType === "text" && style === "single" && (
                <div
                  ref={watermarkRef}
                  onMouseDown={handleWatermarkMouseDown}
                  style={{
                    position: "absolute",
                    left: `${posX}%`,
                    top: `${posY}%`,
                    transform: "translate(-50%, -50%)",
                    color: color,
                    opacity: opacity / 100,
                    fontSize: `${fontSize}px`,
                    fontFamily: font.split(" / ")[0],
                    fontWeight: 600,
                    textShadow: "0 2px 8px rgba(0,0,0,0.5), 0 0 4px rgba(0,0,0,0.3)",
                    whiteSpace: "nowrap",
                    userSelect: "none",
                    cursor: dragging ? "grabbing" : "grab",
                    padding: "4px 8px",
                  }}
                >
                  {text}
                </div>
              )}

              {tab === "effect" && wmType === "text" && style === "tile" && (
                <div style={{ position: "absolute", inset: 0, pointerEvents: "none", overflow: "hidden" }}>
                  {Array.from({ length: 50 }).map((_, i) => {
                    const r = Math.floor(i / 7);
                    const c = i % 7;
                    return (
                      <span
                        key={i}
                        style={{
                          position: "absolute",
                          left: `${5 + c * 15}%`,
                          top: `${5 + r * 15}%`,
                          color: color,
                          opacity: opacity / 100,
                          fontSize: `${fontSize}px`,
                          fontFamily: font.split(" / ")[0],
                          fontWeight: 600,
                          textShadow: "0 1px 4px rgba(0,0,0,0.5)",
                          whiteSpace: "nowrap",
                          userSelect: "none",
                        }}
                      >
                        {text}
                      </span>
                    );
                  })}
                </div>
              )}

              {tab === "effect" && wmType === "text" && style === "diagonal" && (
                <div
                  style={{
                    position: "absolute",
                    inset: 0,
                    pointerEvents: "none",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    transform: "rotate(-30deg)",
                  }}
                >
                  <span
                    style={{
                      color: color,
                      opacity: opacity / 100,
                      fontSize: `${fontSize}px`,
                      fontFamily: font.split(" / ")[0],
                      fontWeight: 600,
                      textShadow: "0 2px 8px rgba(0,0,0,0.5)",
                      whiteSpace: "nowrap",
                      userSelect: "none",
                      padding: "20px 60px",
                      background: "rgba(0,0,0,0.05)",
                    }}
                  >
                    {text}
                  </span>
                </div>
              )}

              {tab === "effect" && wmType === "image" && logoSrc && (
                <img
                  src={logoSrc}
                  onMouseDown={handleWatermarkMouseDown}
                  style={{
                    position: "absolute",
                    left: `${posX}%`,
                    top: `${posY}%`,
                    transform: "translate(-50%, -50%)",
                    maxWidth: `${logoScale}%`,
                    maxHeight: `${logoScale}%`,
                    opacity: opacity / 100,
                    cursor: dragging ? "grabbing" : "grab",
                    userSelect: "none",
                  }}
                />
              )}
            </div>
          )}
        </div>

        {/* 状态行 */}
        <div style={{ fontSize: 12, color: "#6b7280", marginTop: 8, textAlign: "center" }}>
          {inputPath ? `📄 ${inputPath.split(/[/\\]/).pop()}` : "未加载图片"}
        </div>
      </div>

      {/* ===== 右侧：控制面板 ===== */}
      <div style={{ background: "rgba(20, 20, 35, 0.5)", borderLeft: "1px solid rgba(99,102,241,0.15)", padding: 16, overflowY: "auto" }}>
        {/* 导出区 */}
        <div style={{ background: "rgba(30,30,50,0.5)", borderRadius: 8, padding: 12, marginBottom: 16 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
            <span style={{ fontSize: 12, color: "#9ca3af" }}>导出目录</span>
            {inputPath && (
              <button onClick={() => setInputPath("")} style={{ background: "none", border: "none", color: "#9ca3af", fontSize: 11, cursor: "pointer" }}>清除图片</button>
            )}
          </div>
          <div style={{ display: "flex", gap: 6 }}>
            <input
              className="input"
              type="text"
              placeholder="选择输出目录..."
              value={outputDir}
              onChange={(e) => setOutputDir(e.target.value)}
              style={{ flex: 1, fontSize: 12 }}
            />
            <button className="btn" onClick={pickOutputDir} style={{ fontSize: 11 }}>📁</button>
          </div>
          {inputPath && (
            <div style={{ marginTop: 6 }}>
              <input
                className="input"
                type="text"
                placeholder="输出文件名 (留空用原名)"
                value={outputName}
                onChange={(e) => setOutputName(e.target.value)}
                style={{ width: "100%", fontSize: 12 }}
              />
            </div>
          )}
        </div>

        {/* 水印类型 */}
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 6 }}>水印类型</div>
          <div style={{ display: "flex", background: "rgba(30,30,50,0.5)", borderRadius: 6, padding: 3 }}>
            <button
              onClick={() => setWmType("text")}
              style={{
                flex: 1, padding: "6px", border: "none", borderRadius: 4,
                background: wmType === "text" ? "#3b82f6" : "transparent",
                color: wmType === "text" ? "#fff" : "#9ca3af", fontSize: 12, fontWeight: 500, cursor: "pointer",
              }}
            >
              文字水印
            </button>
            <button
              onClick={() => setWmType("image")}
              style={{
                flex: 1, padding: "6px", border: "none", borderRadius: 4,
                background: wmType === "image" ? "#3b82f6" : "transparent",
                color: wmType === "image" ? "#fff" : "#9ca3af", fontSize: 12, fontWeight: 500, cursor: "pointer",
              }}
            >
              Logo 图片
            </button>
          </div>
        </div>

        {/* Logo 拖拽区 */}
        {wmType === "image" && (
          <div style={{ marginBottom: 16 }}>
            <div
              onClick={async () => {
                try {
                  const picked = await openDialog({ multiple: false, filters: [{ name: "Logo", extensions: ["png", "jpg", "jpeg", "webp"] }] });
                  if (typeof picked === "string") setLogoPath(picked);
                } catch (e) { /* ignore */ }
              }}
              onDragOver={(e) => e.preventDefault()}
              onDrop={(e) => {
                e.preventDefault();
                const file = e.dataTransfer?.files[0];
                if (file) {
                  const path = (file as File & { path?: string }).path;
                  if (path && isImageFile(path)) setLogoPath(path);
                }
              }}
              style={{
                border: `2px dashed ${logoPath ? "#4ade80" : "#374151"}`,
                borderRadius: 8, padding: 16, textAlign: "center",
                background: logoPath ? "rgba(74,222,128,0.05)" : "transparent",
                color: logoPath ? "#d1d5db" : "#6b7280",
                fontSize: 12, cursor: "pointer",
              }}
            >
              {logoPath ? `✅ ${logoPath.split(/[/\\]/).pop()}` : "📁 拖入 Logo"}
            </div>
            <div style={{ marginTop: 8, fontSize: 12, color: "#9ca3af", display: "flex", justifyContent: "space-between" }}>
              <span>Logo 大小</span>
              <span>{logoScale}%</span>
            </div>
            <input type="range" min={5} max={50} value={logoScale} onChange={(e) => setLogoScale(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
        )}

        {/* 文字内容 */}
        {wmType === "text" && (
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 6 }}>水印内容</div>
            <input
              className="input"
              type="text"
              value={text}
              onChange={(e) => setText(e.target.value)}
              style={{ width: "100%", fontSize: 13 }}
            />
          </div>
        )}

        {/* 字体 */}
        {wmType === "text" && (
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 6 }}>字体</div>
            <select
              value={font}
              onChange={(e) => setFont(e.target.value)}
              style={{
                width: "100%", padding: "8px 10px", borderRadius: 6,
                background: "rgba(30,30,50,0.5)", color: "#e5e7eb", fontSize: 12,
                border: "1px solid #374151",
              }}
            >
              {FONTS.map((f) => <option key={f} value={f}>{f}</option>)}
            </select>
          </div>
        )}

        {/* 字号 */}
        {wmType === "text" && (
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 6, display: "flex", justifyContent: "space-between" }}>
              <span>字号</span>
              <span style={{ color: "#3b82f6" }}>{fontSize} px</span>
            </div>
            <input type="range" min={10} max={120} value={fontSize} onChange={(e) => setFontSize(Number(e.target.value))} style={{ width: "100%" }} />
          </div>
        )}

        {/* 颜色 */}
        {wmType === "text" && (
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 6 }}>颜色</div>
            <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <input
                type="color"
                value={color}
                onChange={(e) => setColor(e.target.value)}
                style={{ width: 36, height: 28, border: "1px solid #374151", borderRadius: 4, cursor: "pointer", background: "transparent" }}
              />
              <input
                className="input"
                type="text"
                value={color}
                onChange={(e) => setColor(e.target.value)}
                style={{ flex: 1, fontSize: 12, fontFamily: "monospace" }}
              />
            </div>
          </div>
        )}

        {/* 水印样式 */}
        <div style={{ marginBottom: 12 }}>
          <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 6 }}>水印样式</div>
          <div style={{ display: "flex", background: "rgba(30,30,50,0.5)", borderRadius: 6, padding: 3 }}>
            {(["single", "tile", "diagonal"] as WMStyle[]).map((s) => (
              <button
                key={s}
                onClick={() => setStyle(s)}
                style={{
                  flex: 1, padding: "6px", border: "none", borderRadius: 4,
                  background: style === s ? "#3b82f6" : "transparent",
                  color: style === s ? "#fff" : "#9ca3af", fontSize: 12, fontWeight: 500, cursor: "pointer",
                }}
              >
                {s === "single" ? "单个" : s === "tile" ? "平铺" : "对角线"}
              </button>
            ))}
          </div>
        </div>

        {/* 快速定位 3x3 网格 */}
        {style === "single" && (
          <div style={{ marginBottom: 12 }}>
            <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 6 }}>快速定位</div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 4, aspectRatio: "1" }}>
              {Array.from({ length: 9 }).map((_, i) => (
                <button
                  key={i}
                  onClick={() => handleGridClick(i as PosIdx)}
                  style={{
                    background: posIdx === i ? "#3b82f6" : "rgba(30,30,50,0.5)",
                    border: posIdx === i ? "none" : "1px solid #374151",
                    borderRadius: 4, cursor: "pointer",
                    display: "flex", alignItems: "center", justifyContent: "center",
                  }}
                >
                  <div style={{ width: 4, height: 4, borderRadius: "50%", background: posIdx === i ? "#fff" : "#6b7280" }} />
                </button>
              ))}
            </div>
          </div>
        )}

        {/* 水平/垂直位置滑块 */}
        {style === "single" && (
          <>
            <div style={{ marginBottom: 8 }}>
              <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 4, display: "flex", justifyContent: "space-between" }}>
                <span>水平位置</span>
                <span style={{ color: "#3b82f6" }}>{posX}%</span>
              </div>
              <input type="range" min={0} max={100} value={posX} onChange={(e) => { setPosX(Number(e.target.value)); setPosIdx(-1 as PosIdx); }} style={{ width: "100%" }} />
            </div>
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 4, display: "flex", justifyContent: "space-between" }}>
                <span>垂直位置</span>
                <span style={{ color: "#3b82f6" }}>{posY}%</span>
              </div>
              <input type="range" min={0} max={100} value={posY} onChange={(e) => { setPosY(Number(e.target.value)); setPosIdx(-1 as PosIdx); }} style={{ width: "100%" }} />
            </div>
            <div style={{ fontSize: 11, color: "#6b7280", textAlign: "center", marginBottom: 12 }}>
              也可以直接在预览图上拖动水印
            </div>
          </>
        )}

        {/* 透明度 */}
        <div style={{ marginBottom: 16 }}>
          <div style={{ fontSize: 12, color: "#9ca3af", marginBottom: 6, display: "flex", justifyContent: "space-between" }}>
            <span>透明度</span>
            <span style={{ color: "#3b82f6" }}>{opacity}%</span>
          </div>
          <input type="range" min={5} max={100} value={opacity} onChange={(e) => setOpacity(Number(e.target.value))} style={{ width: "100%" }} />
        </div>

        {/* 应用按钮 */}
        <button
          onClick={handleApply}
          disabled={processing || !inputPath || !outputDir}
          style={{
            width: "100%",
            padding: "12px",
            border: "none",
            borderRadius: 8,
            background: (processing || !inputPath || !outputDir) ? "rgba(59,130,246,0.3)" : "linear-gradient(135deg, #3b82f6, #6366f1)",
            color: "#fff",
            fontSize: 14,
            fontWeight: 500,
            cursor: (processing || !inputPath || !outputDir) ? "not-allowed" : "pointer",
          }}
        >
          {processing ? "处理中..." : "💧 应用水印并导出"}
        </button>

        {result && (
          <div style={{ marginTop: 12, padding: 10, background: "rgba(74,222,128,0.1)", borderRadius: 6, fontSize: 11, color: "#4ade80", wordBreak: "break-all" }}>
            ✅ {result.split(/[/\\]/).pop()}
          </div>
        )}
      </div>
    </div>
  );
}
