# Manual Test Checklist — Video Toolbox MVP

> Phase 5 出货前必跑。**逐项勾选**, 不通过的不发版。
> 跑法: 安装 `target/release/bundle/msi/*.msi` 或 `target/release/bundle/nsis/*.exe`,
> 启动应用, 按以下步骤逐项验证, 记录结果。

- 测试机: Windows 10 22H2 / Windows 11 23H2, x64
- 测试日期: __________
- 测试人员: __________
- App 版本: v0.1.0
- yt-dlp.exe: `src-tauri/bin/yt-dlp.exe` (已就位)
- ffmpeg.exe: `src-tauri/bin/ffmpeg.exe` (已就位)

---

## 0. 安装与启动

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| INST-01 | 双击 MSI / NSIS 安装 | 进度条走完, 桌面出现 Video Toolbox 快捷方式 | ☐ |
| INST-02 | 启动应用 (从开始菜单 / 桌面) | 窗口出现, 标题 "Video Toolbox" | ☐ |
| INST-03 | 窗口尺寸 ≈ 1100x720 | 符合 tauri.conf.json 配置 | ☐ |
| INST-04 | 关闭应用 | 进程退出, 无残留 | ☐ |
| INST-05 | 再次启动 | 数据目录 (`%APPDATA%\com.video-toolbox.app\`) 保留 | ☐ |

---

## 1. URL 解析

| ID | 步骤 | 输入 | 预期 | 通过 |
| --- | --- | --- | --- | --- |
| URL-01 | B 站公开视频 | `https://www.bilibili.com/video/BV1xx411c7mD` (替换为真实 ID) | 5s 内显示标题 / UP 主 / 时长, 格式表填充 | ☐ |
| URL-02 | 短链 | `https://b23.tv/XXXX` | 解析成功 | ☐ |
| URL-03 | 无效 URL | `not a url` | 错误提示, 不崩溃 | ☐ |
| URL-04 | 空白 URL | (不输入) | "解析" 按钮 disabled | ☐ |
| URL-05 | YouTube 需登录视频 (有 cookie) | `https://www.youtube.com/watch?v=jNQXAC9IVRw` | 解析成功 (前提: 已导入 cookie) | ☐ |
| URL-06 | YouTube 无 cookie | 同上 | 报错 "Sign in to confirm you're not a bot" 或类似 | ☐ |

---

## 2. 选格式下载

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| FMT-01 | 解析后, 格式表显示 ≥ 1 行 | 列出多种分辨率 / 编码 / 大小 | ☐ |
| FMT-02 | 点击某行选中 | 行高亮 + 出现 "✓" | ☐ |
| FMT-03 | 选 480p / 720p 短视频, 点下载 | 进度条动, 速度 / ETA 显示 | ☐ |
| FMT-04 | 下载完成 | 状态变 "下载完成", 文件出现在保存目录 | ☐ |
| FMT-05 | 文件可播放 | 用系统播放器打开, 音视频正常 | ☐ |
| FMT-06 | 文件名格式 | 类似 `视频标题 [BV1xx411c7mD].mp4` (yt-dlp template) | ☐ |
| FMT-07 | 文件大小 | 接近格式表中显示的 size (允许 ±5%) | ☐ |

---

## 3. 取消下载

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| CAN-01 | 选一个大文件 (≥ 50MB) 开始下载 | 进度条动 | ☐ |
| CAN-02 | 进度到 10-30% 时点取消 | 进度条消失, 状态变 error / 取消 | ☐ |
| CAN-03 | 进程退出 | 任务管理器无残留 yt-dlp.exe 子进程 | ☐ |
| CAN-04 | 保存目录无半成品 `.part` / `.ytdl` 文件 | (或允许残留, 后续覆盖) | ☐ |
| CAN-05 | 立即再次点下载 | 不报错, 正常启动 | ☐ |

---

## 4. 导入 Cookies

> 准备: `yt_cookies.txt` (Netscape 格式, 从浏览器扩展导出)

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| CK-01 | 设置页点 "导入 Cookies" | 弹文件选择对话框 | ☐ |
| CK-02 | 选择 `yt_cookies.txt` | 提示 "Cookies 导入成功" | ☐ |
| CK-03 | Cookie 列表显示 youtube.com | 列表新增一行, 显示 domain / size / mtime | ☐ |
| CK-04 | 用导入的 cookie 解析 YouTube (URL-05) | 解析成功 | ☐ |
| CK-05 | 重复导入同一文件 | 静默覆盖 (不报错, mtime 更新) | ☐ |
| CK-06 | 删除 cookie | 行消失 | ☐ |
| CK-07 | 选非 .txt 文件 | 拒绝 (扩展名过滤) | ☐ |
| CK-08 | 选不存在的文件路径 | 报错 "文件不存在" | ☐ |
| CK-09 | 文件名不识别 (cookies 内容无法解析域名) | 报错 "无法识别域名" | ☐ |

---

## 5. 历史记录

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| HIS-01 | 完成 1 个下载, 历史列表自动新增 | 显示标题 / URL / 时间 | ☐ |
| HIS-02 | 时间显示 | ISO8601 / 本地时间, 与系统时间一致 | ☐ |
| HIS-03 | 按下载时间 DESC | 最新在上 | ☐ |
| HIS-04 | 点 "重下" (如前端实现了) | 重新填入 URL | ☐ |
| HIS-05 | 点删除 | 二次确认 → 列表移除 | ☐ |
| HIS-06 | 删除所有历史 | 列表清空 | ☐ |
| HIS-07 | 关闭并重启应用, 历史仍存在 | 持久化生效 | ☐ |
| HIS-08 | 搜索框 (如前端实现了) 输入关键字 | 过滤结果 | ☐ |

---

## 6. 设置修改

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| SET-01 | 打开设置页 | 显示当前 default_save_dir / format / proxy / cookies | ☐ |
| SET-02 | 改 default_save_dir 为 `D:\Downloads\videos` | 写入 config.json / SQLite | ☐ |
| SET-03 | 重启应用, 设置保留 | 持久化生效 | ☐ |
| SET-04 | 新下载使用新目录 | 文件出现在 D:\Downloads\videos | ☐ |
| SET-05 | 改 default_format_preference | 下次解析时该格式被默认选中 | ☐ |
| SET-06 | 设置 HTTP 代理 (如 `http://127.0.0.1:7890`) | yt-dlp 通过代理下载 | ☐ |
| SET-07 | 清空代理 | 恢复直连 | ☐ |
| SET-08 | 设置非法路径 (如不存在的盘符) | 下载时报错, 不崩溃 | ☐ |

---

## 7. 文件菜单 / "在文件夹中显示"

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| FS-01 | 完成 1 个下载, 历史行右键 / 按钮 → "在文件夹中显示" | Explorer 打开, 高亮目标文件 | ☐ |
| FS-02 | 点击保存目录旁的 "打开" 按钮 | Explorer 打开保存目录 | ☐ |
| FS-03 | 保存目录不存在 | 报错, 不崩溃 | ☐ |

---

## 8. 错误路径 / 边界

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| ERR-01 | 解析时拔网线 | 5-30s 后报错, 提示网络问题 | ☐ |
| ERR-02 | 视频已被删除 / 私有 | 报错, 提示 "video unavailable" 之类 | ☐ |
| ERR-03 | 磁盘空间不足 | 报错, 不留下半成品 | ☐ |
| ERR-04 | 同时下载 3 个 | UI 至少不卡死, 进度独立更新 | ☐ |
| ERR-05 | 解析 100 次同一 URL | 不内存泄漏, 不报 "too many open files" | ☐ |
| ERR-06 | kill 应用 (任务管理器) | 启动后不残留, 数据不损坏 | ☐ |
| ERR-07 | 修改 `%APPDATA%\com.video-toolbox.app\history.db` 删字段 | 下次启动重建或报错, 不无限循环 | ☐ |

---

## 9. 性能粗略冒烟

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| PERF-01 | 应用启动到窗口可见 | < 3s (含 WebView 冷启) | ☐ |
| PERF-02 | 解析响应 | < 5s (国内 B 站) | ☐ |
| PERF-03 | 进度更新延迟 | < 500ms (用户感) | ☐ |
| PERF-04 | 内存占用 (空闲) | < 200MB (含 WebView2) | ☐ |

---

## 10. 打包产物

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| PKG-01 | `target/release/bundle/msi/*.msi` 存在 | 文件存在, 大小 ≥ 30MB | ☐ |
| PKG-02 | `target/release/bundle/nsis/*.exe` 存在 | 文件存在, 大小 ≥ 30MB | ☐ |
| PKG-03 | MSI 内含 yt-dlp.exe + ffmpeg.exe | 用 7-zip / msiexec 提取检查 | ☐ |
| PKG-04 | 双击启动安装后的 EXE | 自动找同目录 yt-dlp.exe (不需要 PATH) | ☐ |
| PKG-05 | 卸载 (控制面板 → 卸载) | 数据目录保留 (或询问删除, 行为按设计) | ☐ |

---

## 11. E2E 冒烟脚本 (CI 集成)

| ID | 步骤 | 预期 | 通过 |
| --- | --- | --- | --- |
| CI-01 | `pwsh scripts/smoke-test-e2e.ps1` (用 powershell.exe 替代) | 输出 "OK", ExitCode 0 | ☐ |
| CI-02 | 故意删 release exe 再跑 | "FAIL: release binary not found", ExitCode 1 | ☐ |
| CI-03 | 故意删 bundle/msi + bundle/nsis | "FAIL: Neither MSI nor NSIS artifact found", ExitCode 2 | ☐ |

---

## 通过标准

- 所有 **P0** 项 (高亮或标 ⭐) 全通过 → 发版
- **P1** 项 允许 ≤ 2 项不通过, 需在 release notes 注明
- **P2** 项 不阻塞, 但需 backlog 记录

### P0 项清单

- INST-01, INST-02, INST-04
- URL-01, URL-03
- FMT-01, FMT-03, FMT-04, FMT-05
- CAN-01, CAN-02, CAN-03
- CK-01, CK-02, CK-03
- HIS-01, HIS-03, HIS-05, HIS-07
- SET-02, SET-03, SET-04
- FS-01
- PKG-01, PKG-02, PKG-04
- CI-01

---

## 已知限制 (MVP 不要求)

- ❌ macOS / Linux 打包
- ❌ 浏览器扩展自动抓 cookies (需手动导出)
- ❌ 下载队列并发管理 (UI 可能不显示多任务切换)
- ❌ yt-dlp 自动更新
- ❌ 主题切换 (只有亮色)
- ❌ 国际化 (只有中文)

---

## 报告模板

完成所有 P0 后, 填写:

- [ ] 所有 P0 PASS, 总计 X/37 通过
- [ ] 已知 P1 失败: <列出>
- [ ] 新发现 bug: <列出, 含严重等级>
- [ ] 截图: <粘贴 1-2 张关键 UI 截图>
- [ ] 测试人员签字: __________ 日期: __________
