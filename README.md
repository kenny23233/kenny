# Video Toolbox

一个基于 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 的桌面视频下载工具。
Tauri 2 + React + TypeScript，Windows x64。

## 特点

- 粘贴 URL，解析后可选择格式下载
- 支持自定义 cookies（手动导入）
- 下载历史记录
- yt-dlp 和 ffmpeg 嵌入安装包，单 exe 自带运行依赖
- 闭源，仅本机/团队使用

## 目录

```
src/                  React 前端
src-tauri/            Rust 后端
  src/commands/       Tauri IPC commands
  src/ytdlp/          yt-dlp 包装
  bin/                内置 yt-dlp.exe / ffmpeg.exe (打包时塞进安装包)
  icons/              应用图标
  target/release/     构建产物
    bundle/msi/*.msi         Windows Installer
    bundle/nsis/*-setup.exe  NSIS 安装器
    video-toolbox.exe        主可执行文件
docs/                 文档
scripts/              辅助脚本 (build-release.ps1, smoke-test.ps1)
release/              打包产物
```

## 开发环境

需要：
- **Rust** 1.78+ (rustup)
- **Node.js** 20+ (npm)
- **Visual Studio 2022 Build Tools** (含 "C++ 桌面开发" 工作负载)
- **WebView2 Runtime** (Win11 自带，Win10 1803+ 也基本自带)

## 开发模式 (热重载)

```powershell
cd E:\projects\video-toolbox
npm install
npm run tauri dev
```

## 构建 release 安装包

### 一次性: 准备二进制

`src-tauri/bin/` 里必须有 `yt-dlp.exe` 和 `ffmpeg.exe`,否则安装包不带这些工具。
两者都是 GPL / Unlicense 的可独立分发的可执行文件,直接下载放到 bin/ 即可:

- yt-dlp: <https://github.com/yt-dlp/yt-dlp/releases> (下载 `yt-dlp.exe`)
- ffmpeg: <https://www.gyan.dev/ffmpeg/builds/> (下载 `ffmpeg-release-essentials.zip` 里的 ffmpeg.exe)

```
src-tauri/
  bin/
    yt-dlp.exe   (~18 MB)
    ffmpeg.exe   (~80-145 MB, 视构建版本)
```

### 一键打包

```powershell
cd E:\projects\video-toolbox
npm run build:release
```

这会跑 `scripts/build-release.ps1`:

1. 预检 `bin/` 和 `icons/`
2. 跑 `npm install` (按需)
3. 跑 `npm run tauri build` (首次 5-10 分钟, LTO 编译)
4. 打印所有产物路径

### 产物位置

| 类型 | 路径 |
|------|------|
| MSI (Windows Installer) | `src-tauri/target/release/bundle/msi/*.msi` |
| NSIS (Setup) | `src-tauri/target/release/bundle/nsis/*-setup.exe` |
| 主 exe (免安装) | `src-tauri/target/release/video-toolbox.exe` |

### 手动步骤 (如果想自己控制)

```powershell
# 1. 安装前端依赖
npm install

# 2. 构建前端 + 后端 + 打包安装程序
npm run tauri build

# 3. 看产物
dir src-tauri\target\release\bundle\
```

## 冒烟测试

构建完之后, 跑冒烟测试确认所有产物都在:

```powershell
npm run smoke-test
```

只做静态检查, **不实际跑安装** (那是用户的事)。

## 体积优化 (release)

`src-tauri/Cargo.toml` 已经设了:

- `panic = "abort"`            - 不展开 panic, 减小体积
- `lto = true`                 - thin LTO, 跨 crate 内联
- `opt-level = "s"`            - 优化体积
- `strip = true`               - 剥符号
- `codegen-units = 1`          - 单 codegen unit, 让 LTO 更彻底

预期 `video-toolbox.exe` 在 **8-12 MB** 左右 (不含 ffmpeg/yt-dlp)。

## 数据位置

应用数据 (config, 历史 db, cookies, logs) 在:

- Windows: `%APPDATA%\com.video-toolbox.app\`
  - `config.json`   - 应用配置 (默认保存目录 / 代理 / 格式偏好)
  - `history.db`    - SQLite, 下载历史 + 设置审计
  - `cookies/`      - 用户导入的 cookies 文件
  - `logs/`         - 按天轮转的应用日志

## 命令行

```powershell
# 开发
npm run dev              # 只跑前端
npm run tauri dev        # 跑前端 + 启动 Tauri 窗口

# 构建
npm run build            # 只 build 前端 (vite build)
npm run tauri build      # build 前端 + 后端 + 打包安装程序
npm run build:release    # 上面 + 预检 + 报告产物

# 测试
npm run smoke-test       # 验证 release 产物
```

## 致谢

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) — 核心下载能力
- [FFmpeg](https://ffmpeg.org/) — 视频处理
- [Tauri](https://tauri.app/) — 桌面框架
