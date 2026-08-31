# 路线图

## Phase 0：环境准备 ✅ 进行中
- [x] Python 3.12（已完成，前面流程）
- [x] Git 2.55
- [x] Node.js 24 LTS
- [x] Rust 1.98 (rustup)
- [ ] VS Build Tools 2022 + C++ workload（装中）

## Phase 1：MVP 后端（1.5 天）
- [ ] Tauri 项目初始化（`npm create tauri-app`）
- [ ] 项目骨架配置（tauri.conf.json, Cargo.toml, package.json）
- [ ] Rust commands: `probe_url` (用 yt-dlp --dump-json 解析元数据)
- [ ] Rust commands: `start_download` (spawn yt-dlp，emit 进度)
- [ ] Rust commands: `cancel_download` (kill child process)
- [ ] 错误处理 (yt-dlp 退出码、stderr 解析)

## Phase 2：MVP 前端（1.5 天）
- [ ] React + Vite 基础布局
- [ ] `DownloadForm` 组件：URL 输入 + 解析按钮
- [ ] 调 `probe_url` 显示元数据
- [ ] `FormatSelector` 组件：表格列出可选格式
- [ ] 调 `start_download`，订阅 `download://progress` 事件
- [ ] `ProgressBar` 组件：实时进度 + 速度 + ETA
- [ ] 错误提示 toast

## Phase 3：Cookies + 历史（0.5 天）
- [ ] `CookiesPanel` 组件：导入按钮 + 当前 cookies 列表
- [ ] `import_cookies` Rust command：文件对话框 → 读 .txt → 复制到配置目录
- [ ] SQLite schema: `history` 表 (id, url, title, path, size, downloaded_at)
- [ ] `add_history` / `list_history` / `delete_history` commands
- [ ] `HistoryList` 组件：表格 + 重下按钮

## Phase 4：设置 + 收尾（0.5 天）
- [ ] `SettingsPanel` 组件：下载目录、默认格式、代理
- [ ] SQLite settings KV 表
- [ ] 关于页（版本、作者、致谢、license）
- [ ] 应用图标
- [ ] README 完善

## Phase 5：打包（0.5 天）
- [ ] `tauri build` 生成 MSI / NSIS
- [ ] 内置 yt-dlp.exe + ffmpeg.exe（已复制到 src-tauri/bin/）
- [ ] 体积优化
- [ ] 冒烟测试（安装、启动、下载、卸载）

## 总时间
**5 个工作日左右**（如果没遇到大坑）

## 不做的（MVP 不含）
- ❌ macOS / Linux 支持
- ❌ 浏览器扩展自动抓 cookies
- ❌ 云同步
- ❌ 订阅/批量下载
- ❌ 自动更新 yt-dlp
- ❌ 多语言（先中文）
- ❌ 主题切换（先亮色）
- ❌ 下载加速（多线程）

## 后续可加（Post-MVP）
- 拖拽 URL 到窗口下载
- 剪贴板自动识别
- 后台静默下载 + 系统通知
- 下载队列管理
- 内置 yt-dlp 自动更新
- 简易视频预览
