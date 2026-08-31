import { useState } from "react";
// 直接 import docs/COOKIES.md 作为字符串 (Vite ?raw query)
// tauri-plugin-shell 没启用, 没法 shell.openPath, 所以把教程内嵌渲染
import cookiesDoc from "../../docs/COOKIES.md?raw";

export function AboutPanel() {
  const [showCookiesDoc, setShowCookiesDoc] = useState(false);

  return (
    <div>
      <div className="card">
        <h3 className="card-title">Video Toolbox</h3>
        <table className="table table-fixed info-table">
          <tbody>
            <tr>
              <td className="muted col-actions-sm">版本</td>
              <td>v0.1.0</td>
            </tr>
            <tr>
              <td className="muted col-actions-sm">作者</td>
              <td>you</td>
            </tr>
            <tr>
              <td className="muted col-actions-sm">协议</td>
              <td>MIT</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div className="card">
        <h3 className="card-title">致谢</h3>
        <ul className="info-list">
          <li>
            <a
              href="https://github.com/yt-dlp/yt-dlp"
              target="_blank"
              rel="noreferrer"
            >
              yt-dlp
            </a>{" "}
            — 视频解析与下载核心
          </li>
          <li>
            <a
              href="https://ffmpeg.org/"
              target="_blank"
              rel="noreferrer"
            >
              FFmpeg
            </a>{" "}
            — 视频转码与合并
          </li>
          <li>
            <a
              href="https://tauri.app/"
              target="_blank"
              rel="noreferrer"
            >
              Tauri
            </a>{" "}
            — 桌面应用框架
          </li>
        </ul>
      </div>

      <div className="card">
        <h3 className="card-title">资源</h3>
        <div className="row-wrap">
          <a
            className="btn btn-secondary"
            href="https://github.com/yourname/video-toolbox"
            target="_blank"
            rel="noreferrer"
          >
            🔗 项目主页
          </a>
          <button
            className="btn"
            onClick={() => setShowCookiesDoc((v) => !v)}
          >
            {showCookiesDoc ? "收起教程" : "查看 cookies 导出教程"}
          </button>
        </div>

        {showCookiesDoc && (
          <div className="doc-wrap">
            <pre className="doc-pre">{cookiesDoc}</pre>
          </div>
        )}
      </div>
    </div>
  );
}
