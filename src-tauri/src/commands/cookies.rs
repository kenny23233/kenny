use crate::config::{Config, ConfigState};
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CookieInfo {
    pub domain: String,
    pub path: String,
    pub size_bytes: u64,
    pub last_modified: String, // ISO8601
}

/// 公共的扫描函数 - 给 commands::settings::get_settings 复用,避免递归命令调用
pub fn list_cookies_impl(config: &Config) -> Result<Vec<CookieInfo>, String> {
    let cookies_dir = config.cookies_dir();
    if !cookies_dir.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    let entries = std::fs::read_dir(&cookies_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("txt") {
            continue;
        }
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                chrono::DateTime::<chrono::Utc>::from_timestamp(d.as_secs() as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let domain = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        out.push(CookieInfo {
            domain,
            path: p.to_string_lossy().to_string(),
            size_bytes: meta.len(),
            last_modified: modified,
        });
    }
    out.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(out)
}

/// 校验/规范化 domain 文件名,只允许字母数字、点、横线
fn sanitize_domain(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("域名不能为空".to_string());
    }
    let valid = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    if !valid {
        return Err(format!("域名包含非法字符: {}", raw));
    }
    Ok(trimmed.to_ascii_lowercase())
}

#[tauri::command]
pub async fn import_cookies(
    file_path: String,
    config: State<'_, Arc<ConfigState>>,
) -> Result<String, String> {
    let src = PathBuf::from(&file_path);
    if !src.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    // 从文件内容中识别域名
    let content = std::fs::read_to_string(&src).map_err(|e| e.to_string())?;
    let domain_raw = detect_domain_from_cookies(&content, &file_path)
        .ok_or_else(|| "无法从 cookies 文件识别域名,请尝试使用 import_cookies_for_domain 手动指定".to_string())?;
    let domain = sanitize_domain(&domain_raw)?;

    let snapshot = config.snapshot().map_err(|e| format!("读取配置失败: {}", e))?;
    let cookies_dir = snapshot.cookies_dir();
    std::fs::create_dir_all(&cookies_dir).map_err(|e| e.to_string())?;

    let dest = cookies_dir.join(format!("{}.txt", domain));
    std::fs::copy(&src, &dest).map_err(|e| format!("复制 cookies 文件失败: {}", e))?;

    tracing::info!("imported cookies: {} -> {}", file_path, dest.display());
    Ok(domain)
}

/// 用户手动指定域名导入 (绕过自动识别)
#[tauri::command]
pub async fn import_cookies_for_domain(
    domain: String,
    file_path: String,
    config: State<'_, Arc<ConfigState>>,
) -> Result<(), String> {
    let domain = sanitize_domain(&domain)?;
    let src = PathBuf::from(&file_path);
    if !src.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }

    let snapshot = config.snapshot().map_err(|e| format!("读取配置失败: {}", e))?;
    let cookies_dir = snapshot.cookies_dir();
    std::fs::create_dir_all(&cookies_dir).map_err(|e| e.to_string())?;

    let dest = cookies_dir.join(format!("{}.txt", domain));
    std::fs::copy(&src, &dest).map_err(|e| format!("复制 cookies 文件失败: {}", e))?;

    tracing::info!("imported cookies for domain {}: {} -> {}", domain, file_path, dest.display());
    Ok(())
}

#[tauri::command]
pub async fn list_cookies(
    config: State<'_, Arc<ConfigState>>,
) -> Result<Vec<CookieInfo>, String> {
    let snapshot = config.snapshot().map_err(|e| format!("读取配置失败: {}", e))?;
    list_cookies_impl(&snapshot)
}

/// 删除指定域名的 cookies
#[tauri::command]
pub async fn delete_cookies(
    domain: String,
    config: State<'_, Arc<ConfigState>>,
) -> Result<(), String> {
    let domain = sanitize_domain(&domain)?;
    let snapshot = config.snapshot().map_err(|e| format!("读取配置失败: {}", e))?;
    let cookies_dir = snapshot.cookies_dir();
    let target = cookies_dir.join(format!("{}.txt", domain));
    if !target.exists() {
        return Err(format!("该域名的 cookies 不存在: {}", domain));
    }
    std::fs::remove_file(&target).map_err(|e| format!("删除 cookies 文件失败: {}", e))?;
    tracing::info!("deleted cookies: {}", target.display());
    Ok(())
}

/// 从 cookies 文件内容中识别域名
/// 支持:
///   1. 文件名提示 (例如 youtube.txt) - 优先
///   2. Netscape 格式 (第一行是 "# Netscape HTTP Cookie File" 或 "# HTTP Cookie File")
///   3. JSON 数组格式 (含 "domain" 字段)
///   4. 通用兜底: 正则匹配 .example.com
pub(crate) fn detect_domain_from_cookies(content: &str, file_path: &str) -> Option<String> {
    // 1. 先看文件路径里有没有提示
    if let Some(name) = Path::new(file_path).file_stem().and_then(|s| s.to_str()) {
        if !name.is_empty() && name.chars().any(|c| c == '.') {
            return Some(name.to_string());
        }
    }

    // 2. 检查是否是 Netscape 格式
    let is_netscape = content.lines().take(3).any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("netscape http cookie file")
            || lower.contains("http cookie file")
            || lower.starts_with("#httponly_")
    });

    if is_netscape {
        // Netscape 格式行: domain | flag | path | secure | expiration | name | value
        // domain 列以 "." 开头 (例如 .example.com),或以 "#HttpOnly_." 开头
        // 也可能首列就是 "example.com" (无前导点)
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') && !trimmed.to_ascii_lowercase().starts_with("#httponly_") {
                continue;
            }
            // 去掉 #HttpOnly_ 前缀
            let cleaned = if let Some(stripped) = trimmed.strip_prefix("#HttpOnly_") {
                stripped
            } else if let Some(stripped) = trimmed.strip_prefix("#httponly_") {
                stripped
            } else {
                trimmed
            };
            let first_col = cleaned.split('\t').next()?.trim();
            if first_col.is_empty() {
                continue;
            }
            // 去掉前导点, 取剩下部分作为域
            let candidate = first_col.trim_start_matches('.');
            if is_plausible_domain(candidate) {
                return Some(candidate.to_string());
            }
        }
    }

    // 3. 尝试 JSON 格式
    if content.trim_start().starts_with('[') || content.trim_start().starts_with('{') {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(d) = extract_domain_from_json(&value) {
                return Some(d);
            }
        }
    }

    // 4. 兜底: 正则匹配 .example.com
    let re = Regex::new(r"\.([a-zA-Z0-9-]+\.[a-zA-Z]{2,})").ok()?;
    if let Some(c) = re.captures(content) {
        return c.get(1).map(|m| m.as_str().to_string());
    }
    None
}

fn extract_domain_from_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(extract_domain_from_json)
            .next(),
        serde_json::Value::Object(map) => {
            // 优先取 "domain" 字段
            if let Some(d) = map.get("domain").and_then(|v| v.as_str()) {
                if is_plausible_domain(d.trim_start_matches('.')) {
                    return Some(d.trim_start_matches('.').to_string());
                }
            }
            // 递归找
            for v in map.values() {
                if let Some(d) = extract_domain_from_json(v) {
                    return Some(d);
                }
            }
            None
        }
        _ => None,
    }
}

/// 简单校验: 至少一个点, 长度 >= 3, 字母数字横线点
fn is_plausible_domain(s: &str) -> bool {
    let s = s.trim();
    if s.len() < 3 {
        return false;
    }
    if !s.contains('.') {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_domain_from_netscape_format() {
        let content = "# Netscape HTTP Cookie File\n\
                       # https://example.com/\n\
                       .example.com\tTRUE\t/\tFALSE\t0\tname\tvalue\n";
        assert_eq!(
            detect_domain_from_cookies(content, "cookies.txt"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn test_detect_domain_from_netscape_httponly() {
        let content = "# Netscape HTTP Cookie File\n\
                       #HttpOnly_.youtube.com\tTRUE\t/\tFALSE\t0\tSID\tabc123\n";
        assert_eq!(
            detect_domain_from_cookies(content, "cookies.txt"),
            Some("youtube.com".to_string())
        );
    }

    #[test]
    fn test_detect_domain_from_filename() {
        // 文件名含点 (即已经是 youtube.com.txt),优先用文件名
        let content = "garbage content, no recognizable domain here";
        assert_eq!(
            detect_domain_from_cookies(content, "C:/some/path/bilibili.com.txt"),
            Some("bilibili.com".to_string())
        );
    }

    #[test]
    fn test_detect_domain_from_json() {
        let content = r#"[{"name":"k","value":"v","domain":".twitter.com","path":"/"}]"#;
        assert_eq!(
            detect_domain_from_cookies(content, "cookies.txt"),
            Some("twitter.com".to_string())
        );
    }

    #[test]
    fn test_detect_domain_fallback_regex() {
        // 非 Netscape / JSON, 兜底正则
        let content = "some random text with .example.org embedded";
        assert_eq!(
            detect_domain_from_cookies(content, "cookies.txt"),
            Some("example.org".to_string())
        );
    }

    #[test]
    fn test_detect_domain_nothing() {
        let content = "no domain at all here, just garbage 12345";
        assert_eq!(detect_domain_from_cookies(content, "cookies.txt"), None);
    }

    #[test]
    fn test_sanitize_domain() {
        assert_eq!(sanitize_domain("YouTube.COM").unwrap(), "youtube.com");
        assert_eq!(sanitize_domain("example.com").unwrap(), "example.com");
        assert!(sanitize_domain("bad/name").is_err());
        assert!(sanitize_domain("").is_err());
        assert!(sanitize_domain("   ").is_err());
    }
}
