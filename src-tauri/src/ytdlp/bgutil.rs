//! bgutil-ytdlp-pot-provider HTTP server 管理
//!
//! bgutil server 是 Node.js 应用, 默认监听 127.0.0.1:4416,
//! 提供 /ping (健康检查) 和 /get_pot (生成 PO Token) 两个端点。
//!
//! 本模块负责:
//! - 启动/停止 server 子进程
//! - 探测端口避免重复启动
//! - 在 Tauri app setup 阶段被调用一次
//!
//! 资源查找顺序 (与 ytdlp_path/ffmpeg_path 保持一致):
//! 1. Tauri resource_dir 下的 server 子目录
//! 2. exe 同目录
//! 3. CARGO_MANIFEST_DIR/bin/bgutil/

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tokio::process::{Child, Command};

/// bgutil HTTP server 默认地址 (与 plugin 默认 base_url 一致)
pub const BGUTIL_DEFAULT_URL: &str = "http://127.0.0.1:4416";
const BGUTIL_DEFAULT_PORT: u16 = 4416;

/// server 子进程句柄 (启动后保存, 退出时 kill)
static BGUTIL_CHILD: OnceLock<Mutex<Option<Child>>> = OnceLock::new();

/// server 资源目录 (由 set_bgutil_resource_dir 在 setup 时设置)
static BGUTIL_RESOURCE_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn child_slot() -> &'static Mutex<Option<Child>> {
    BGUTIL_CHILD.get_or_init(|| Mutex::new(None))
}

fn resource_dir_slot() -> &'static Mutex<Option<PathBuf>> {
    BGUTIL_RESOURCE_DIR.get_or_init(|| Mutex::new(None))
}

/// 设置 bgutil 资源目录 (Tauri app setup 时调用)
pub fn set_bgutil_resource_dir(dir: PathBuf) {
    if let Ok(mut g) = resource_dir_slot().lock() {
        *g = Some(dir);
    }
}

fn bgutil_resource_dir() -> Option<PathBuf> {
    resource_dir_slot().lock().ok().and_then(|g| g.clone())
}

/// 在指定目录下寻找 bgutil 部署 (含 build/main.js)
fn find_bgutil_dir() -> Option<PathBuf> {
    let candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Some(rd) = bgutil_resource_dir() {
            v.push(rd.join("bgutil"));
            v.push(rd.join("bgutil-pot-server"));
            if let Some(p) = rd.parent() {
                v.push(p.join("bgutil"));
                v.push(p.join("bgutil-pot-server"));
                v.push(p.join("resources").join("bgutil"));
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                v.push(dir.join("bgutil"));
                v.push(dir.join("bgutil-pot-server"));
            }
        }
        if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            let m = PathBuf::from(manifest);
            v.push(m.join("bin").join("bgutil"));
            v.push(m.join("bin").join("bgutil-pot-server"));
        }
        v
    };

    for c in candidates {
        if c.join("build").join("main.js").exists() {
            return Some(c);
        }
    }
    None
}

/// 在指定目录下寻找 node.exe
fn find_node() -> Option<PathBuf> {
    let candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Some(rd) = bgutil_resource_dir() {
            v.push(rd.join("node.exe"));
            if let Some(p) = rd.parent() {
                v.push(p.join("node.exe"));
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                v.push(dir.join("node.exe"));
            }
        }
        if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            let m = PathBuf::from(manifest);
            v.push(m.join("bin").join("node.exe"));
        }
        // PATH 兜底
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                v.push(dir.join("node.exe"));
            }
        }
        v
    };

    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

/// 在指定目录下寻找 deno.exe
fn find_deno() -> Option<PathBuf> {
    let candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Some(rd) = bgutil_resource_dir() {
            v.push(rd.join("deno.exe"));
            if let Some(p) = rd.parent() {
                v.push(p.join("deno.exe"));
            }
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                v.push(dir.join("deno.exe"));
            }
        }
        if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            let m = PathBuf::from(manifest);
            v.push(m.join("bin").join("deno.exe"));
        }
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                v.push(dir.join("deno.exe"));
            }
        }
        v
    };

    for c in candidates {
        if c.exists() {
            return Some(c);
        }
    }
    None
}

/// 检查 4416 端口是否已在监听 (避免重复启动)
pub fn is_bgutil_already_running() -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", BGUTIL_DEFAULT_PORT).parse().unwrap(),
        Duration::from_millis(300),
    )
    .is_ok()
}

/// 启动 bgutil HTTP server, 返回 (success, message)
///
/// 如果端口已被占用 (例如用户手动跑了一个), 直接返回成功。
pub async fn ensure_bgutil_running() -> Result<String, String> {
    if is_bgutil_already_running() {
        tracing::info!("[bgutil] server 已在端口 {} 监听, 跳过启动", BGUTIL_DEFAULT_PORT);
        return Ok(format!("already-running:{}", BGUTIL_DEFAULT_URL));
    }

    let bgutil_dir = find_bgutil_dir().ok_or_else(|| {
        "未找到 bgutil POT server (期望路径: <resource_dir>/bgutil/build/main.js)".to_string()
    })?;
    let node = find_node().ok_or_else(|| "未找到 node.exe (用于运行 bgutil server)".to_string())?;
    let main_js = bgutil_dir.join("build").join("main.js");

    tracing::info!("[bgutil] 启动 server: node {} {}", node.display(), main_js.display());

    // 启动后日志写入 %APPDATA%\video-toolbox\logs\bgutil.log
    let log_path = std::env::var("APPDATA")
        .ok()
        .map(|p| PathBuf::from(p).join("video-toolbox").join("logs").join("bgutil.log"));

    let mut cmd = Command::new(&node);
    cmd.arg(&main_js)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(log) = &log_path {
        if let Some(parent) = log.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // 重定向到文件
        if let Ok(f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
        {
            cmd.stdout(Stdio::from(f));
        }
        if let Ok(f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log)
        {
            cmd.stderr(Stdio::from(f));
        }
    }

    cmd.kill_on_drop(true);
    let child = cmd
        .spawn()
        .map_err(|e| format!("启动 bgutil server 失败: {}", e))?;

    if let Ok(mut g) = child_slot().lock() {
        *g = Some(child);
    }

    // 等待 server 起来 (最多 5s)
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if is_bgutil_already_running() {
            tracing::info!("[bgutil] server 启动成功: {}", BGUTIL_DEFAULT_URL);
            return Ok(format!("started:{}", BGUTIL_DEFAULT_URL));
        }
    }

    Err("bgutil server 启动超时 (5s 内未监听 4416)".to_string())
}

/// 停止 bgutil server (在 app 退出时调用)
pub async fn stop_bgutil_server() {
    if let Ok(mut g) = child_slot().lock() {
        if let Some(mut child) = g.take() {
            let _ = child.kill().await;
            tracing::info!("[bgutil] server 已停止");
        }
    }
}

/// 返回 Deno 路径 (供 yt-dlp --js-runtimes 用), 若没找到返回 None
pub fn deno_path() -> Option<PathBuf> {
    find_deno()
}

/// bgutil 当前状态 (供前端显示)
#[derive(Debug, Clone, serde::Serialize)]
pub struct BgutilStatus {
    pub available: bool,
    pub url: String,
    pub node_found: bool,
    pub deno_found: bool,
    pub server_dir_found: bool,
    pub port_in_use: bool,
}

pub fn get_status() -> BgutilStatus {
    BgutilStatus {
        available: is_bgutil_already_running(),
        url: BGUTIL_DEFAULT_URL.to_string(),
        node_found: find_node().is_some(),
        deno_found: find_deno().is_some(),
        server_dir_found: find_bgutil_dir().is_some(),
        port_in_use: is_bgutil_already_running(),
    }
}

/// 单元测试: 路径查找
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgutil_default_url() {
        assert_eq!(BGUTIL_DEFAULT_URL, "http://127.0.0.1:4416");
    }

    #[test]
    fn test_status_default() {
        let s = get_status();
        assert_eq!(s.url, BGUTIL_DEFAULT_URL);
    }
}
