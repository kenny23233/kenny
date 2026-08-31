# 架构

## 总体

Tauri 2 应用，分两层：
- **前端**（React + TypeScript）：UI，运行在系统 WebView 中
- **后端**（Rust）：Tauri 进程，负责调 yt-dlp、读写 SQLite、管理 cookies

## 进程模型

```
┌─────────────────────────────────────┐
│  WebView (React UI)                 │
│  - 渲染界面                        │
│  - 通过 @tauri-apps/api 调 commands │
└──────────┬──────────────────────────┘
           │ IPC (JSON over WebView message bus)
┌──────────▼──────────────────────────┐
│  Rust 进程 (Tauri Main)             │
│  ┌─────────────────────────────┐   │
│  │ commands/                   │   │
│  │  - download (probe, fetch)   │   │
│  │  - cookies (import, status)  │   │
│  │  - history (list, delete)    │   │
│  │  - settings (get, set)       │   │
│  └──────────┬──────────────────┘   │
│  ┌──────────▼──────────────────┐   │
│  │ ytdlp/                       │   │
│  │  - Runner (调 yt-dlp.exe)    │   │
│  │  - Parser (parse dump-json)  │   │
│  └──────────┬──────────────────┘   │
│  ┌──────────▼──────────────────┐   │
│  │ db.rs (rusqlite)             │   │
│  │  - history 表                 │   │
│  │  - settings KV 表             │   │
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
           │
           │ std::process::Command
           ▼
   yt-dlp.exe (内置)
   ffmpeg.exe (内置)
```

## 数据流：下载

```
用户粘贴 URL
    ↓
[前端] DownloadForm
    ↓ invoke("probe_url", { url })
[后端] ytdlp::runner::probe
    → 调 yt-dlp.exe --dump-json --no-download URL
    → 解析 JSON 返回 { title, duration, formats: [...] }
    ↓
[前端] FormatSelector 显示可选格式
    ↓ 用户选格式，点下载
    ↓ invoke("start_download", { url, format_id, save_dir })
[后端] ytdlp::runner::start
    → spawn yt-dlp.exe -f FORMAT -o SAVE_DIR/%(title)s.%(ext)s
    → 解析 stdout 进度行
    → emit("download://progress", { id, percent, speed, eta })
    ↓
[前端] ProgressBar 实时显示
    ↓ 下载完成
[后端] 写 history 表 + emit("download://done")
[前端] 提示完成，刷新历史列表
```

## 数据存储

所有数据存在 `%APPDATA%\video-toolbox\`：

```
%APPDATA%\video-toolbox\
├── config.yaml          用户配置
├── history.db           SQLite (历史 + settings)
├── cookies/
│   ├── youtube.txt      YouTube cookies
│   ├── bilibili.txt     B 站 cookies
│   └── ...
└── logs/
    └── app.log
```

**为什么放 `%APPDATA%` 而不是工具箱目录？**
- 多电脑共享工具箱二进制时，配置/历史各电脑独立
- 升级时不覆盖用户数据
- U 盘版工具箱不会把数据写进 U 盘（提升寿命）

## 关键决策

- **手动 cookies 导入**：不写浏览器扩展，不读系统 cookie 文件，零安全风险
- **yt-dlp 内置**：每次启动用 `--update-to` 检查更新（可选）
- **数据库本地**：SQLite 存历史，不联网
- **不收集任何遥测**：完全离线
