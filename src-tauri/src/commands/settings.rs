use crate::commands::cookies::{list_cookies_impl, CookieInfo};
use crate::config::ConfigState;
use crate::db::Database;
use crate::ytdlp;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub default_save_dir: String,
    pub default_format_preference: String,
    pub proxy: Option<String>,
    pub cookies: Vec<CookieInfo>,
}

#[tauri::command]
pub async fn get_settings(
    config: State<'_, Arc<ConfigState>>,
) -> Result<Settings, String> {
    // 直接调公共实现,避免命令互相调用时的 State 借用问题
    let snapshot = config.snapshot().map_err(|e| format!("读取配置失败: {}", e))?;
    let cookies = list_cookies_impl(&snapshot)?;
    Ok(Settings {
        default_save_dir: snapshot.default_save_dir.to_string_lossy().to_string(),
        default_format_preference: snapshot.default_format_preference.clone(),
        proxy: snapshot.proxy.clone(),
        cookies,
    })
}

/// 通用 key/value 设置, 写 SQLite (audit)
/// 已知字段的持久化请用专门的 set_default_save_dir / set_proxy / set_default_format_preference
#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    db: State<'_, Arc<Database>>,
) -> Result<(), String> {
    db.set_setting(&key, &value)
        .map_err(|e| format!("保存设置失败: {}", e))
}

#[tauri::command]
pub async fn set_default_save_dir(
    path: String,
    config: State<'_, Arc<ConfigState>>,
) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("目录不存在: {}", path));
    }
    if !p.is_dir() {
        return Err(format!("不是一个目录: {}", path));
    }
    config
        .set("default_save_dir", &p.to_string_lossy())
        .map_err(|e| format!("保存配置失败: {}", e))?;
    tracing::info!("set_default_save_dir: {}", path);
    Ok(())
}

#[tauri::command]
pub async fn set_proxy(
    proxy: Option<String>,
    config: State<'_, Arc<ConfigState>>,
) -> Result<(), String> {
    // proxy 允许 None (清空), 也允许空字符串 (等价于 None)
    let normalized: Option<String> = proxy.and_then(|p| {
        let t = p.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    });
    // 简单校验: http://, https://, socks5:// 开头
    if let Some(ref p) = normalized {
        if !(p.starts_with("http://") || p.starts_with("https://") || p.starts_with("socks5://")) {
            return Err(format!("代理地址格式不合法,需以 http:// / https:// / socks5:// 开头: {}", p));
        }
    }
    let value = normalized.as_deref().unwrap_or("");
    config
        .set("proxy", value)
        .map_err(|e| format!("保存配置失败: {}", e))?;
    tracing::info!("set_proxy: {:?}", normalized);
    Ok(())
}

#[tauri::command]
pub async fn set_default_format_preference(
    pref: String,
    config: State<'_, Arc<ConfigState>>,
) -> Result<(), String> {
    let trimmed = pref.trim();
    if trimmed.is_empty() {
        return Err("格式偏好不能为空".to_string());
    }
    config
        .set("default_format_preference", trimmed)
        .map_err(|e| format!("保存配置失败: {}", e))?;
    tracing::info!("set_default_format_preference: {}", trimmed);
    Ok(())
}

/// 用系统文件管理器打开默认下载目录
#[tauri::command]
pub async fn open_save_dir(
    config: State<'_, Arc<ConfigState>>,
) -> Result<(), String> {
    let snapshot = config.snapshot().map_err(|e| format!("读取配置失败: {}", e))?;
    let path = snapshot.default_save_dir.clone();
    open_in_explorer(&path)
}

/// 在文件管理器中打开并高亮某文件
#[tauri::command]
pub async fn reveal_in_folder(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    reveal_path(&p)
}

fn open_in_explorer(path: &std::path::Path) -> Result<(), String> {
    use std::process::Command;
    if !path.exists() {
        std::fs::create_dir_all(path).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("启动文件管理器失败: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("启动 Finder 失败: {}", e))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("启动文件管理器失败: {}", e))?;
    }
    Ok(())
}

fn reveal_path(path: &std::path::Path) -> Result<(), String> {
    use std::process::Command;
    #[cfg(target_os = "windows")]
    {
        // explorer /select,"<path>" 语法: 单个参数,引号嵌在字符串里
        let arg = format!(r#"/select,"{}""#, path.to_string_lossy());
        Command::new("explorer")
            .arg(&arg)
            .spawn()
            .map_err(|e| format!("启动文件管理器失败: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-R", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("启动 Finder 失败: {}", e))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // Linux 上 dbus-send 或 xdg-mime, 简单兜底: 打开父目录
        let parent = path.parent().unwrap_or(path);
        Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("启动文件管理器失败: {}", e))?;
    }
    Ok(())
}

/// 校验关键二进制可用性
#[tauri::command]
pub async fn check_binaries() -> Result<(), String> {
    ytdlp::check_binaries()
}

/// bgutil POT server 状态 (前端 Settings 页面显示)
#[tauri::command]
pub async fn get_bgutil_status() -> crate::ytdlp::bgutil::BgutilStatus {
    crate::ytdlp::bgutil::get_status()
}
