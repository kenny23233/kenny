import { useCallback, useEffect, useMemo, useState } from "react";
import { DownloadPanel } from "./components/DownloadPanel";
import { CookiesPanel } from "./components/CookiesPanel";
import { HistoryList } from "./components/HistoryList";
import { SettingsPanel } from "./components/SettingsPanel";
import { AboutPanel } from "./components/AboutPanel";
import { ErrorToast } from "./components/ErrorToast";
import { ErrorContext, type ErrorApi } from "./components/ErrorContext";
import { PsdPanel } from "./components/tools/PsdPanel";
import { NcmPanel } from "./components/tools/NcmPanel";
import { WmPanel } from "./components/tools/WmPanel";

type TabId = "home" | "download" | "psd" | "ncm" | "wm" | "cookies" | "history" | "settings" | "about";
type Theme = "light" | "dark";

const TABS: { id: TabId; label: string; icon: string }[] = [
  { id: "home", label: "主页", icon: "🏠" },
  { id: "psd", label: "PSD 转 PNG", icon: "🖼️" },
  { id: "ncm", label: "NCM 解密", icon: "🎵" },
  { id: "wm", label: "图片水印", icon: "💧" },
  { id: "download", label: "视频下载", icon: "📥" },
];

const THEME_KEY = "video-toolbox-theme";

export default function App() {
  const [active, setActive] = useState<TabId>("wm");
  const [urlSeed, setUrlSeed] = useState<string | null>(null);
  const [defaultSaveDir, setDefaultSaveDir] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const [theme, setTheme] = useState<Theme>("dark");
  const [tabTransitionKey, setTabTransitionKey] = useState(0);

  // 加载主题
  useEffect(() => {
    const saved = localStorage.getItem(THEME_KEY) as Theme | null;
    const initial: Theme = saved ?? "dark";
    setTheme(initial);
    document.documentElement.setAttribute("data-theme", initial);
  }, []);

  // 切换主题
  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem(THEME_KEY, theme);
  }, [theme]);

  const toggleTheme = useCallback(() => {
    setTheme((t) => (t === "dark" ? "light" : "dark"));
  }, []);

  // 切换 tab 时给内容区加个 key 触发重渲染动画
  const handleTabChange = useCallback((id: TabId) => {
    setActive(id);
    setTabTransitionKey((k) => k + 1);
  }, []);

  const errorApi = useMemo<ErrorApi>(
    () => ({
      error,
      show: (msg) => setError(msg),
      clear: () => setError(null),
    }),
    [error],
  );

  const handleRedownload = useCallback((url: string) => {
    setUrlSeed(url);
    setActive("download");
  }, []);

  return (
    <ErrorContext.Provider value={errorApi}>
      <div className="app">
        {/* 顶栏 */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            padding: "12px 24px",
            background: "var(--bg-overlay)",
            backdropFilter: "blur(20px)",
            WebkitBackdropFilter: "blur(20px)",
            borderBottom: "1px solid var(--border)",
            gap: 16,
            position: "relative",
            zIndex: 10,
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
            <div
              className="hover-scale"
              style={{
                width: 40, height: 40,
                borderRadius: 10,
                background: "var(--gradient-primary)",
                display: "flex", alignItems: "center", justifyContent: "center",
                fontSize: 20,
                color: "var(--accent-on)",
                boxShadow: "var(--shadow-md)",
                cursor: "pointer",
              }}
            >
              🎵
            </div>
            <div>
              <div style={{ fontSize: 17, fontWeight: 600, color: "var(--text)", background: "var(--gradient-primary)", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent", backgroundClip: "text" }}>
                本地工具箱
              </div>
              <div style={{ fontSize: 11, color: "var(--text-dim)", marginTop: 2 }}>
                PSD / PSB 转 PNG · NCM 解密 · 图片水印 · 视频下载 —— 一个页面全搞定
              </div>
            </div>
          </div>

          <div style={{ flex: 1 }} />

          <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
            {TABS.map((t) => {
              const isActive = active === t.id;
              return (
                <button
                  key={t.id}
                  onClick={() => handleTabChange(t.id)}
                  className="hover-lift"
                  style={{
                    padding: "8px 14px",
                    borderRadius: 8,
                    border: "none",
                    background: isActive ? "var(--accent-light)" : "transparent",
                    color: isActive ? "var(--accent)" : "var(--text-dim)",
                    cursor: "pointer",
                    fontSize: 13,
                    fontWeight: isActive ? 600 : 400,
                    display: "flex",
                    alignItems: "center",
                    gap: 6,
                    transition: "all var(--duration-fast) var(--ease-out)",
                  }}
                >
                  <span style={{ fontSize: 14 }}>{t.icon}</span>
                  {t.label}
                </button>
              );
            })}

            <button
              onClick={toggleTheme}
              className="hover-scale"
              title="切换主题"
              style={{
                marginLeft: 8,
                width: 32, height: 32,
                borderRadius: 8,
                border: "1px solid var(--border)",
                background: "var(--bg-elev-2)",
                color: "var(--text-muted)",
                fontSize: 14,
                cursor: "pointer",
                display: "flex", alignItems: "center", justifyContent: "center",
                transition: "all var(--duration-base) var(--ease-spring)",
              }}
            >
              {theme === "dark" ? "☀️" : "🌙"}
            </button>

            <button
              onClick={() => handleTabChange("settings")}
              className="hover-scale"
              title="设置"
              style={{
                width: 32, height: 32,
                borderRadius: 8,
                border: "1px solid var(--border)",
                background: active === "settings" ? "var(--accent-light)" : "var(--bg-elev-2)",
                color: active === "settings" ? "var(--accent)" : "var(--text-muted)",
                fontSize: 14,
                cursor: "pointer",
                display: "flex", alignItems: "center", justifyContent: "center",
                transition: "all var(--duration-base) var(--ease-spring)",
              }}
            >
              ⚙️
            </button>

            <div
              style={{
                marginLeft: 4,
                padding: "6px 12px",
                borderRadius: 8,
                border: "1px solid rgba(16, 185, 129, 0.3)",
                background: "rgba(16, 185, 129, 0.08)",
                color: "var(--success)",
                fontSize: 12,
                fontWeight: 500,
                display: "flex", alignItems: "center", gap: 4,
              }}
            >
              🔒 纯本地运行
            </div>
          </div>
        </div>

        <ErrorToast />

        <div
          className="content"
          key={tabTransitionKey}
          style={{
            padding: 0,
            animation: "fadeIn 0.3s var(--ease-out)",
          }}
        >
          {active === "home" && <HomePanel onJump={handleTabChange} />}
          {active === "download" && (
            <DownloadPanel urlSeed={urlSeed} defaultSaveDir={defaultSaveDir} />
          )}
          {active === "psd" && <PsdPanel />}
          {active === "ncm" && <NcmPanel />}
          {active === "wm" && <WmPanel />}
          {active === "cookies" && <CookiesPanel />}
          {active === "history" && (
            <HistoryList onRedownload={handleRedownload} />
          )}
          {active === "settings" && (
            <SettingsPanel
              onSettingsLoaded={(dir) => setDefaultSaveDir(dir)}
            />
          )}
          {active === "about" && <AboutPanel />}
        </div>
      </div>
    </ErrorContext.Provider>
  );
}

function HomePanel({ onJump }: { onJump: (t: TabId) => void }) {
  const tools = [
    { id: "psd" as TabId, icon: "🖼️", title: "PSD / PSB 转 PNG", desc: "提取设计稿图层，批量处理", color: "#f59e0b" },
    { id: "ncm" as TabId, icon: "🎵", title: "NCM 解密", desc: "网易云音乐格式转 MP3 / FLAC", color: "#10b981" },
    { id: "wm" as TabId, icon: "💧", title: "图片水印", desc: "文字 / Logo 水印，实时预览", color: "#6366f1" },
    { id: "download" as TabId, icon: "📥", title: "视频下载", desc: "yt-dlp 内置，800+ 网站支持", color: "#ec4899" },
  ];
  return (
    <div style={{ padding: 32, display: "grid", gridTemplateColumns: "repeat(2, 1fr)", gap: 16, maxWidth: 800, margin: "0 auto" }}>
      {tools.map((t, i) => (
        <button
          key={t.id}
          onClick={() => onJump(t.id)}
          className="card card-interactive anim-slideUp"
          style={{
            padding: 24,
            background: "var(--bg-elev)",
            border: "1px solid var(--border)",
            borderRadius: 12,
            cursor: "pointer",
            textAlign: "left",
            color: "var(--text)",
            animationDelay: `${i * 60}ms`,
            animationFillMode: "backwards",
          }}
        >
          <div
            style={{
              fontSize: 32, marginBottom: 12,
              width: 56, height: 56,
              borderRadius: 12,
              background: `${t.color}20`,
              display: "flex", alignItems: "center", justifyContent: "center",
            }}
          >
            {t.icon}
          </div>
          <div style={{ fontSize: 16, fontWeight: 600, marginBottom: 4 }}>{t.title}</div>
          <div style={{ fontSize: 12, color: "var(--text-dim)" }}>{t.desc}</div>
        </button>
      ))}
    </div>
  );
}
