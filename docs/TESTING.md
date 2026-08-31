# 测试计划 — Video Toolbox (Phase 4 MVP)

> Owner: qa-engineer  
> 日期: 2026-08-31  
> 范围: MVP 验证 (Phase 1-5)

---

## 1. 测试目标

验证 video-toolbox MVP 的核心可用性, 即用户能:

1. 粘贴视频 URL, 正确解析出元数据和格式列表
2. 选定格式后能成功下载到本地
3. 下载过程中能取消
4. 能导入 cookies, 后续下载可带 cookie
5. 历史记录能正确写入、列出、删除
6. 设置能正确读写 (默认保存目录 / 格式偏好 / 代理)
7. 编译产物 (MSI / NSIS) 能正常启动

**不验证**(MVP 之外): UI 美观、下载速度、自动更新、多语言、订阅、并发性能、macOS / Linux 兼容。

---

## 2. 测试范围

### ✅ 在范围内

| 模块 | 关键路径 |
| --- | --- |
| 后端命令 | `probe_url` / `start_download` / `cancel_download` / `import_cookies` / `list_cookies` / `list_history` / `delete_history` / `get_settings` / `set_setting` |
| 后端工具 | `ytdlp::ytdlp_path` / `ytdlp::ffmpeg_path` / `ytdlp::extract_domain` / `ytdlp::runner::parse_progress_line` |
| 数据层 | `Database::open` / `add_history` / `list_history` / `delete_history` / `get_setting` / `set_setting` |
| 前端类型 | `VideoInfo` / `FormatInfo` / `ProgressEvent` (TS 类型契约) |
| 打包 | `tauri build` 产物 (MSI / NSIS / .exe) |
| 启动 | release binary 启动后进程稳定 |

### ❌ 不在范围

- yt-dlp 自身 bug 行为 (只测我们的封装, 不测 yt-dlp 全部)
- 网络失败时 yt-dlp 的重试逻辑
- UI 视觉回归
- 跨平台 (仅 Windows 10/11 x64)
- 性能基准 (只做粗略冒烟)
- 浏览器扩展导入 cookies (MVP 不做)

---

## 3. 测试类型

| 类型 | 占比 | 谁来写 | 何时跑 |
| --- | --- | --- | --- |
| 单元测试 | 30% | backend-pro (src-tauri 内部 `#[cfg(test)]`) | `cargo test` |
| 集成测试 | 25% | qa-engineer (`src-tauri/tests/integration_test.rs`) | `cargo test --test integration_test` |
| 前端类型测试 | 10% | qa-engineer (`src/types/tauri.test.ts`) | `node --test --experimental-strip-types` |
| E2E 冒烟 | 10% | qa-engineer (`scripts/smoke-test-e2e.ps1`) | release 产物后手动 / CI |
| 手动测试 | 25% | qa-engineer + 用户 (`docs/MANUAL-TEST-CHECKLIST.md`) | Phase 5 收尾 |

---

## 4. 测试环境

### 4.1 硬件 / OS

- **目标**: Windows 10 22H2 / Windows 11 23H2, x64
- **测试机**: 任意主流笔记本或台式机, ≥ 8GB RAM
- **网络**: 国内 (国内 bilibili 视频; 国外 YouTube "Me at the zoo" jNQXAC9IVRw 需要 cookie)

### 4.2 软件依赖

- Node.js 24 LTS
- Rust 1.77+ (1.98 已装, MSVC toolchain)
- yt-dlp.exe (项目内 `src-tauri/bin/yt-dlp.exe`)
- ffmpeg.exe (项目内 `src-tauri/bin/ffmpeg.exe`)
- WiX Toolset 3.x (MSI 打包, Tauri 自动调用)

### 4.3 无头 vs GUI

- **无头 (CI / 无人值守)**: cargo test, node --test, smoke-test-e2e.ps1
- **GUI (手动)**: 必须有显示器, 用于启动 .exe 验证窗口、UI 操作

### 4.4 测试数据源

| 用途 | 源 | 可访问性 |
| --- | --- | --- |
| 解析 + 下载成功路径 | `https://www.bilibili.com` 公开视频 | ✅ 国内 |
| 文件下载 (无 UI) | `https://www.learningcontainer.com/wp-content/uploads/2020/05/sample-mp4-file.mp4` (10MB) | ✅ |
| 需要登录的源 | `https://www.youtube.com/watch?v=jNQXAC9IVRw` ("Me at the zoo") | ⚠️ 需 cookie |
| 弱网 / 国外源 | archive.org / GCS | ❌ 国内访问不稳, 跳过 |

---

## 5. 测试用例列表

### 5.1 后端集成测试 (qa-engineer)

> 文件: `src-tauri/tests/integration_test.rs`  
> 跑法: `cargo test --test integration_test -- --nocapture`

| ID | 描述 | 预期 | 优先级 |
| --- | --- | --- | --- |
| IT-DOM-01 | `extract_domain("https://www.youtube.com/watch?v=xxx")` | `Some("youtube.com")` | 高 |
| IT-DOM-02 | `extract_domain("https://example.com:8080/path")` 带端口 | `Some("example.com")` | 高 |
| IT-DOM-03 | `extract_domain("https://example.com/?q=1&b=2")` 带 query | `Some("example.com")` | 高 |
| IT-DOM-04 | `extract_domain("https://example.com/#anchor")` 带 fragment | `Some("example.com")` | 高 |
| IT-DOM-05 | `extract_domain("https://user:pass@example.com/")` 含 user:pass | 文档化当前行为 (`Some("user")`), **已知限制** | 中 |
| IT-DOM-06 | `extract_domain("not a url")` 无 scheme | `None` | 高 |
| IT-DOM-07 | `extract_domain("https://bilibili.com/video/BV1xx")` 不带 www | `Some("bilibili.com")` | 中 |
| IT-PATH-01 | `ytdlp_path()` 在 dev 模式 (有 `CARGO_MANIFEST_DIR`) 解析到 `bin/yt-dlp.exe` | 路径以 `yt-dlp.exe` 结尾, exists | 高 |
| IT-PATH-02 | `ffmpeg_path()` 在 dev 模式解析到 `bin/ffmpeg.exe` | `Some(p)`, p 以 `ffmpeg.exe` 结尾, exists | 高 |
| IT-DB-01 | `add_history` + `list_history` 往返 | 列表含新条目, 字段一致 | 高 |
| IT-DB-02 | `list_history` 按 id DESC 排序 | 最新一条在最前 | 高 |
| IT-DB-03 | `delete_history` 删除指定 id | 该 id 从列表消失 | 高 |
| IT-DB-04 | `delete_history` 不存在的 id | 静默成功, 列表无变化 | 中 |
| IT-DB-05 | `set_setting` + `get_setting` 往返 | 读出与写入一致 | 高 |
| IT-DB-06 | `set_setting` 覆写已存在的 key | 第二次写覆盖第一次 | 中 |
| IT-DB-07 | `get_setting` 不存在的 key | `None` | 中 |
| IT-DB-08 | 临时 db 路径, 多 Database 实例共享同一文件 | 第二次 open 看到第一次写入的数据 (跨连接) | 中 |

### 5.2 前端类型测试 (qa-engineer)

> 文件: `src/types/tauri.test.ts`  
> 跑法: `node --test --experimental-strip-types src/types/tauri.test.ts`

| ID | 描述 | 预期 | 优先级 |
| --- | --- | --- | --- |
| FT-01 | `ProgressEvent.status` 联合类型只能赋值三个字符串 | `"downloading" \| "finished" \| "error"` 都通过编译时检查 | 高 |
| FT-02 | `VideoInfo` 必有字段齐 | id / title / uploader / duration / thumbnail / formats | 中 |
| FT-03 | `FormatInfo.filesize` 可为 null | 编译时允许 `null` | 中 |
| FT-04 | 类型 JSON 序列化契约 (形状) | JSON 字段名与后端 `serde` 输出一致 (snake_case) | 高 |

### 5.3 E2E 冒烟 (qa-engineer)

> 文件: `scripts/smoke-test-e2e.ps1`  
> 跑法: `pwsh scripts/smoke-test-e2e.ps1`

| ID | 描述 | 预期 | 优先级 |
| --- | --- | --- | --- |
| E2E-01 | `target/release/bundle/msi/*.msi` 存在 | 文件存在 | 高 |
| E2E-02 | `target/release/bundle/nsis/*.exe` 存在 (二选一即可) | 文件存在 | 高 |
| E2E-03 | `target/release/video-toolbox.exe` 存在 | 文件存在 | 高 |
| E2E-04 | 启动 binary 后 5 秒内进程仍在 | `Get-Process -Name video-toolbox` 命中 | 高 |
| E2E-05 | 进程可以安全 kill | `Stop-Process` 成功, 退出码可忽略 | 中 |

### 5.4 手动测试 (qa-engineer + 用户)

> 文件: `docs/MANUAL-TEST-CHECKLIST.md`  
> 跑法: 启动 .msi / .exe, 逐项勾选

| ID | 描述 | 优先级 |
| --- | --- | --- |
| MT-01 | 粘贴 B 站公开视频 URL, 解析成功 | 高 |
| MT-02 | 选中 480p 或 720p 格式, 完整下载完 (文件大小 > 0, 可播放) | 高 |
| MT-03 | 下载进行中, 点击取消, 进度条消失, 进程退出 | 高 |
| MT-04 | 导入 `yt_cookies.txt`, 列表中显示 | 高 |
| MT-05 | 历史列表显示已完成条目 | 高 |
| MT-06 | 删除历史条目 | 高 |
| MT-07 | 修改默认保存目录, 重启后保留 | 中 |
| MT-08 | "在文件夹中显示" 打开 explorer | 中 |
| MT-09 | 错误路径: 粘贴无效 URL, 提示合理 | 中 |
| MT-10 | 错误路径: 启动下载时保存目录被删, 报错不崩溃 | 中 |

---

## 6. 准入 / 准出标准

### 准入 (Phase 4 进入)
- [x] backend-pro 完成 `db.rs` / `ytdlp/mod.rs` / `commands/*` 主体实现
- [x] frontend-pro 完成 `App.tsx` 主流程
- [x] yt-dlp.exe + ffmpeg.exe 已就位
- [x] Node 24 / Rust 1.98 / cargo 在 PATH

### 准出 (Phase 5 出货)
- [ ] 所有 P0 集成测试 PASS
- [ ] P0 手动测试用例全通过
- [ ] E2E 冒烟脚本 PASS
- [ ] 已知限制 (IT-DOM-05) 已记录, 用户可接受
- [ ] README 反映 "MVP 限制" 一节

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| 国内访问 YouTube 不稳 | MT-04 测不了 | 用 B 站 / learningcontainer 做主路径, YouTube 仅作可选 |
| Tauri 2 在 Windows 沙箱下文件路径权限 | DB 写入失败 | 集成测试用 `std::env::temp_dir()` 临时文件验证 |
| `extract_domain` 解析 user:pass 错误 | 误把 user 当 domain | 单元测试固定现状 + 文档化 + 建议 backend-pro 后续升级 url crate |
| 集成测试 `current_exe` 在 test binary 路径 | 测不到 exe 同级分支 | 至少验证 `CARGO_MANIFEST_DIR` 兜底, 标"开发模式" |
| 临时 db 残留 | 磁盘脏 | 测试结尾清理 (用 unique name + `fs::remove_file`) |

---

## 8. 自动化 / CI 接入

- 集成测试: 加入 `pre-merge` 检查 (`cargo test --test integration_test`)
- 前端类型测试: 加入 `pre-merge` (`node --test --experimental-strip-types src/types/tauri.test.ts`)
- E2E 冒烟: `tauri build` 完成后, CI 调 `pwsh scripts/smoke-test-e2e.ps1`
- 手动测试清单: 留给 QA / 用户每发版前跑一次

---

## 9. 报告与移交

qa-engineer 在 Phase 5 出货前出一份《测试报告》, 含:

- 各测试 pass/fail 数
- 已知 bug / 限制 (含严重等级)
- 自动化集成情况
- 手动测试通过项
- 后续建议 (P1 / P2)
