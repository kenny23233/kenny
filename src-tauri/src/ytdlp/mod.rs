pub mod bgutil;
pub mod runner;

use std::path::PathBuf;
use std::sync::OnceLock;

/// 启动期从 `app.path().resource_dir()` 写入的全局资源目录
/// - prod (NSIS/MSI):  `<install>/resources/`
/// - dev (tauri dev):  `src-tauri/target/debug/`
static RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 在 setup 阶段调用一次, 把 Tauri 的 resource_dir 缓存到 ytdlp 模块
/// 之后所有 ytdlp_path() / ffmpeg_path() 调用都会先查这个目录
pub fn set_resource_dir(dir: PathBuf) {
    let _ = RESOURCE_DIR.set(dir.clone());
    // 同时传给 bgutil 模块, 让它也用同一资源目录
    crate::ytdlp::bgutil::set_bgutil_resource_dir(dir);
}

fn resource_dir() -> Option<&'static PathBuf> {
    RESOURCE_DIR.get()
}

/// yt-dlp.exe 路径查找顺序:
/// 1. Tauri resource_dir/yt-dlp.exe  (bundle.resources 注入的位置)
/// 2. exe 同目录 yt-dlp.exe           (开发时放在 target/debug 下)
/// 3. CARGO_MANIFEST_DIR/bin/yt-dlp.exe (开发时 src-tauri/bin/)
/// 4. PATH 兜底: 直接 "yt-dlp.exe"     (假设 PATH 中能找到)
pub fn ytdlp_path() -> PathBuf {
    if let Some(rd) = resource_dir() {
        // Tauri 2 NSIS 把 resources 放进 <install>/resources/ 下,
        // 文件名以 bundle.resources 里的 key 为准
        let p = rd.join("yt-dlp.exe");
        if p.exists() {
            return p;
        }
        // 兼容: 一些 bundler 把 resources 散在 install 根
        if let Some(parent) = rd.parent() {
            let p = parent.join("yt-dlp.exe");
            if p.exists() {
                return p;
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("yt-dlp.exe");
            if p.exists() {
                return p;
            }
        }
    }

    // 开发模式 (cargo run / tauri dev)
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("bin").join("yt-dlp.exe");
        if p.exists() {
            return p;
        }
    }

    // 兜底: 假设 PATH 中有 yt-dlp
    PathBuf::from("yt-dlp.exe")
}

/// ffmpeg.exe 路径, 顺序同 ytdlp_path
pub fn ffmpeg_path() -> Option<PathBuf> {
    if let Some(rd) = resource_dir() {
        let p = rd.join("ffmpeg.exe");
        if p.exists() {
            return Some(p);
        }
        if let Some(parent) = rd.parent() {
            let p = parent.join("ffmpeg.exe");
            if p.exists() {
                return Some(p);
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("ffmpeg.exe");
            if p.exists() {
                return Some(p);
            }
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest).join("bin").join("ffmpeg.exe");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// 校验 yt-dlp / ffmpeg 是否可用
/// - yt-dlp 缺失: 返回错误
/// - ffmpeg 缺失: 返回警告信息(部分功能受限,但不致命)
pub fn check_binaries() -> Result<(), String> {
    // 优先检查 Python yt-dlp (支持 bgutil plugin)
    if python_ytdlp_available().is_some() {
        return Ok(());
    }
    // 回落到独立 exe
    let ytdlp = ytdlp_path();
    if ytdlp == PathBuf::from("yt-dlp.exe") {
        let found_in_path = std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths).any(|dir| {
                    let candidate = dir.join(if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" });
                    candidate.exists()
                })
            })
            .unwrap_or(false);
        if !found_in_path {
            return Err("未找到 yt-dlp。请将 yt-dlp.exe 放到程序同目录或 bin/ 下,或加入系统 PATH。\n如已安装 Python,可运行: pip install -U yt-dlp bgutil-ytdlp-pot-provider".to_string());
        }
    } else if !ytdlp.exists() {
        return Err(format!("yt-dlp 不存在: {}", ytdlp.display()));
    }

    if ffmpeg_path().is_none() {
        tracing::warn!("ffmpeg 未找到,部分视频合并/转码功能可能不可用");
    }

    Ok(())
}

/// Python 解释器路径查找 (用于 `python -m yt_dlp`)
pub fn python_path() -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "python.exe" } else { "python" };
    let exe_name3 = if cfg!(windows) { "python3.exe" } else { "python3" };

    let candidates: Vec<PathBuf> = {
        let mut v = Vec::new();
        // 1. resource_dir
        if let Some(rd) = resource_dir() {
            v.push(rd.join(exe_name));
            if let Some(p) = rd.parent() {
                v.push(p.join(exe_name));
            }
        }
        // 2. exe 同目录
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                v.push(dir.join(exe_name));
            }
        }
        // 3. CARGO_MANIFEST_DIR/bin (开发模式)
        if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            let m = PathBuf::from(manifest);
            v.push(m.join("bin").join(exe_name));
        }
        // 4. 已知 Windows 安装位置
        if cfg!(windows) {
            if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
                let programs = PathBuf::from(&appdata).join("Programs").join("Python");
                if let Ok(entries) = std::fs::read_dir(&programs) {
                    for entry in entries.flatten() {
                        v.push(entry.path().join(exe_name));
                    }
                }
            }
            if let Ok(appdata) = std::env::var("APPDATA") {
                let programs = PathBuf::from(&appdata).join("Local").join("Programs").join("Python");
                if let Ok(entries) = std::fs::read_dir(&programs) {
                    for entry in entries.flatten() {
                        v.push(entry.path().join(exe_name));
                    }
                }
            }
        }
        // 5. PATH 兜底
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                v.push(dir.join(exe_name));
                v.push(dir.join(exe_name3));
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

/// 检查 Python yt-dlp 是否可用 (返回 python 路径)
/// 验证方式: `python -m yt_dlp --version` 能成功
pub fn python_ytdlp_available() -> Option<PathBuf> {
    let py = python_path()?;
    // 同步检查 yt_dlp 模块是否可导入
    let output = std::process::Command::new(&py)
        .args(["-c", "import yt_dlp; print(yt_dlp.version.__version__)"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok()?;
    if output.status.success() {
        Some(py)
    } else {
        None
    }
}

/// 选择 yt-dlp 调起方式: 返回 (program, base_args)
/// - 优先 Python yt-dlp (支持 bgutil plugin)
/// - 回落到独立 exe
pub fn ytdlp_invocation() -> (PathBuf, Vec<&'static str>) {
    if let Some(py) = python_ytdlp_available() {
        (py, vec!["-m", "yt_dlp"])
    } else {
        (ytdlp_path(), vec![])
    }
}

/// 从 URL 提取主域名 (例如 https://www.youtube.com/watch?v=xxx -> youtube.com)
///
/// 委托给 `url` crate, 正确处理 user:pass@host、端口、IDN、fragment 等。
pub fn extract_domain(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    // 去掉 www. 前缀 (保留其他子域, 比如 v.douyin.com)
    let host = host.strip_prefix("www.").unwrap_or(host);
    if host.is_empty() {
        return None;
    }
    Some(host.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain_youtube() {
        assert_eq!(
            extract_domain("https://www.youtube.com/watch?v=abc"),
            Some("youtube.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_bilibili() {
        assert_eq!(
            extract_domain("https://www.bilibili.com/video/BV1xx"),
            Some("bilibili.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_douyin() {
        assert_eq!(
            extract_domain("https://v.douyin.com/iJ5n6Q7x/"),
            Some("v.douyin.com".to_string())
        );
    }

    #[test]
    fn test_extract_domain_no_scheme() {
        assert_eq!(extract_domain("not a url"), None);
    }

    #[test]
    fn test_extract_domain_with_port() {
        assert_eq!(
            extract_domain("http://localhost:8080/path"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn test_extract_domain_no_www() {
        assert_eq!(
            extract_domain("https://github.com/user/repo"),
            Some("github.com".to_string())
        );
    }
}
