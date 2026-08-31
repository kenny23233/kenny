//! 热更新模块：读取 manifest，检查版本，下载 MSI，触发安装。
//!
//! manifest 文件放在 %APPDATA%\video-toolbox\update-manifest.json

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Emitter, Manager};

/// manifest 结构
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateManifest {
    pub version: String,
    pub date: String,
    pub release_notes: String,
    /// MSI 安装包：本地路径 / UNC 路径 / HTTP URL
    pub msi_path: String,
    pub msi_size_bytes: Option<u64>,
}

/// 前端需要的版本对比结果
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub latest_version: String,
    pub current_version: String,
    pub update_available: bool,
    pub message: String,
    pub manifest: Option<UpdateManifest>,
}

/// 下载进度事件（通过 Tauri emit 推给前端）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    /// 0.0 ~ 100.0
    pub percent: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    /// "downloading" | "finished" | "error"
    pub status: String,
    pub message: String,
}

fn emit_progress(app: &tauri::AppHandle, p: DownloadProgress) {
    let _ = app.emit("updater://download-progress", p);
}

/// 从 manifest 文件解析
fn parse_manifest(path: &PathBuf) -> Result<UpdateManifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取 manifest 失败: {} (路径: {})", e, path.display()))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("manifest JSON 解析失败: {}", e))
}

/// 简单版本比较：a > b → true
fn version_gt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<(u32, String)> {
        let v = v.strip_prefix('v').unwrap_or(v);
        let parts: Vec<&str> = v.split('-').collect();
        let (base, pre) = if parts.len() >= 2 {
            (parts[0], parts[1..].join("-"))
        } else {
            (parts[0], String::new())
        };
        let nums: Vec<u32> = base.split('.').filter_map(|s| s.parse().ok()).collect();
        let pre_num = if pre.is_empty() { u32::MAX } else { 0 };
        let mut result: Vec<(u32, String)> = nums.into_iter().map(|n| (n, String::new())).collect();
        result.push((pre_num, pre));
        result
    };
    parse(a) > parse(b)
}

fn do_check_update(data_dir: &PathBuf, manifest_path: Option<&PathBuf>) -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let manifest_path = match manifest_path {
        Some(p) => p.clone(),
        None => {
            let p = data_dir.join("update-manifest.json");
            if !p.exists() {
                return Ok(UpdateCheckResult {
                    latest_version: String::new(),
                    current_version,
                    update_available: false,
                    message: format!("未找到 update-manifest.json，请放入：{}\\", data_dir.to_string_lossy()),
                    manifest: None,
                });
            }
            p
        }
    };

    let manifest = match parse_manifest(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            return Ok(UpdateCheckResult {
                latest_version: String::new(),
                current_version,
                update_available: false,
                message: format!("manifest 解析失败: {}", e),
                manifest: None,
            });
        }
    };

    let update_available = version_gt(&manifest.version, &current_version);
    let message = if update_available {
        format!(
            "发现新版本 v{}（当前 v{}），点击\u{201C}安装新版本\u{201D}下载并更新",
            manifest.version, current_version
        )
    } else if manifest.version == current_version {
        format!("已是最新版本 v{}", current_version)
    } else {
        format!("当前 v{} 领先于 manifest v{}", current_version, manifest.version)
    };

    let manifest_clone = manifest.clone();
    Ok(UpdateCheckResult {
        latest_version: manifest.version,
        current_version,
        update_available,
        message,
        manifest: Some(manifest_clone),
    })
}

/// 检查更新
#[tauri::command]
pub async fn check_update(app: tauri::AppHandle) -> Result<UpdateCheckResult, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| format!("获取 app data 目录失败: {}", e))?;
    do_check_update(&data_dir, None)
}

/// 手动指定 manifest 路径
#[tauri::command]
pub async fn check_update_with_manifest(app: tauri::AppHandle, manifest_path: String) -> Result<UpdateCheckResult, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| format!("获取 app data 目录失败: {}", e))?;
    do_check_update(&data_dir, Some(&PathBuf::from(manifest_path)))
}

/// 获取 app data 目录
#[tauri::command]
pub async fn get_app_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| format!("获取 app data 目录失败: {}", e))?;
    Ok(data_dir.to_string_lossy().to_string())
}

/// 判断 msi_path 是否为 HTTP/HTTPS URL
fn is_http_url(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

/// 下载文件到目标路径（带进度回调）
async fn download_file(
    url: &str,
    dest: &PathBuf,
    total_bytes: Option<u64>,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    // reqwest client，禁用代理让内网 CDN 直连
    let client = reqwest::Client::builder()
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("下载返回 HTTP {}: {}", response.status().as_u16(), url));
    }

    let content_length: Option<u64> = response.content_length().or(total_bytes);

    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("创建临时文件失败: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();
    let total = content_length.unwrap_or(0);

    use tokio::io::AsyncWriteExt;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("下载流读取失败: {}", e))?;
        file.write_all(&chunk).await.map_err(|e| format!("写入文件失败: {}", e))?;
        downloaded += chunk.len() as u64;
        let percent = if total > 0 { (downloaded as f64 / total as f64) * 100.0 } else { 0.0 };
        emit_progress(app, DownloadProgress {
            percent: (percent * 10.0).round() / 10.0,
            downloaded_bytes: downloaded,
            total_bytes: if total > 0 { Some(total) } else { None },
            status: "downloading".to_string(),
            message: format!("已下载 {:.1} MB / {:.1} MB", downloaded as f64 / 1_048_576.0, total as f64 / 1_048_576.0),
        });
    }

    file.flush().await.map_err(|e| format!("刷新文件失败: {}", e))?;
    Ok(())
}

/// 启动 MSI 安装程序
fn launch_msi(msi_path: &str) -> Result<(), String> {
    std::process::Command::new("msiexec")
        .args(["/i", msi_path])
        .spawn()
        .map_err(|e| format!("启动 MSI 安装器失败: {}", e))?;
    Ok(())
}

/// 打开 MSI 文件（本地路径用 explorer 高亮，UNC/URL 直接启动）
#[tauri::command]
pub async fn open_msi_folder(_app: tauri::AppHandle, msi_path: String) -> Result<(), String> {
    if is_http_url(&msi_path) {
        // HTTP URL 无法直接打开，告知用户
        return Err("MSI 路径是 URL，请点击\u{201C}安装新版本\u{201D}下载后自动安装。".to_string());
    }
    let p = PathBuf::from(&msi_path);
    if p.exists() {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &msi_path])
            .spawn()
            .map_err(|e| format!("启动 MSI 安装器失败: {}", e))?;
    } else {
        let parent = p.parent().unwrap_or(&p);
        std::process::Command::new("explorer")
            .arg(parent)
            .spawn()
            .map_err(|e| format!("打开文件夹失败: {}", e))?;
    }
    Ok(())
}

/// 下载 MSI 并启动安装程序
///
/// - HTTP/HTTPS URL：下载到 %TEMP%\video-toolbox-updates\ 后启动 msiexec
/// - 本地 / UNC 路径：直接启动 msiexec
/// 前端通过监听 "updater://download-progress" 事件获取下载进度
#[tauri::command]
pub async fn download_and_install(app: tauri::AppHandle, msi_path: String) -> Result<(), String> {
    emit_progress(&app, DownloadProgress {
        percent: 0.0,
        downloaded_bytes: 0,
        total_bytes: None,
        status: "downloading".to_string(),
        message: "正在准备...".to_string(),
    });

    let final_path: String;

    if is_http_url(&msi_path) {
        // 从 URL 提取文件名
        let parsed_url = url::Url::parse(&msi_path)
            .map_err(|e| format!("解析 URL 失败: {}", e))?;
        let url_path = parsed_url.path();
        let filename = std::path::Path::new(url_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("VideoToolbox.msi");

        let temp_dir = std::env::temp_dir().join("video-toolbox-updates");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("创建临时目录失败: {}", e))?;
        let dest = temp_dir.join(filename);
        final_path = dest.to_string_lossy().to_string();

        download_file(&msi_path, &dest, None, &app).await?;

        emit_progress(&app, DownloadProgress {
            percent: 100.0,
            downloaded_bytes: std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0),
            total_bytes: std::fs::metadata(&dest).ok().map(|m| m.len()),
            status: "finished".to_string(),
            message: "下载完成，正在启动安装程序...".to_string(),
        });
    } else {
        // 本地/UNC 路径：直接使用
        final_path = msi_path;
        let p = PathBuf::from(&final_path);
        if !p.exists() {
            return Err(format!("MSI 文件不存在: {}", final_path));
        }
    }

    // 启动 MSI 安装程序（app 窗口随后由前端主动退出）
    launch_msi(&final_path)?;

    emit_progress(&app, DownloadProgress {
        percent: 100.0,
        downloaded_bytes: 0,
        total_bytes: None,
        status: "finished".to_string(),
        message: "安装程序已启动，请等待完成后重新打开应用".to_string(),
    });

    Ok(())
}
