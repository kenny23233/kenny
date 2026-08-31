use crate::config::ConfigState;
use crate::db::Database;
use crate::types::{ProgressEvent, VideoInfo};
use crate::ytdlp::{self, runner};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::process::Child;
use tokio::sync::Mutex as AsyncMutex;

/// 正在运行的下载任务 (id -> child handle)
type ActiveDownloads = Arc<AsyncMutex<HashMap<String, Child>>>;

#[tauri::command]
pub async fn probe_url(
    url: String,
    config: State<'_, Arc<ConfigState>>,
) -> Result<VideoInfo, String> {
    if url.trim().is_empty() {
        return Err("URL 不能为空".to_string());
    }
    tracing::info!("probe_url: {}", url);

    // 启动前先校验关键二进制
    if let Err(e) = ytdlp::check_binaries() {
        return Err(e);
    }

    let snapshot = config.snapshot().map_err(|e| format!("读取配置失败: {}", e))?;
    let cookies_dir = snapshot.cookies_dir();
    let proxy = snapshot.proxy.as_deref();
    match runner::probe(&url, &cookies_dir, proxy).await {
        Ok(info) => Ok(info),
        Err(e) => {
            // 友好化错误信息
            let raw = e.to_string();
            let hint = if raw.contains("Sign in") || raw.contains("login") || raw.contains("confirm") {
                " (可能需要登录 - 请在 Cookies 页面导入对应域名的 cookies 后重试)"
            } else if raw.contains("Unable to extract") || raw.contains("Unsupported URL") {
                " (该网站可能不被支持,或 URL 格式不正确)"
            } else if raw.contains("HTTP Error 403") || raw.contains("HTTP Error 429") {
                " (请求被拒绝,可能触发地区限制或反爬 - 尝试配置代理)"
            } else if raw.contains("Could not connect") || raw.contains("getaddrinfo") || raw.contains("network") {
                " (网络连接失败,请检查网络或代理设置)"
            } else if raw.contains("Private video") || raw.contains("Video unavailable") {
                " (视频不可用 - 私有视频、已删除或地区限制)"
            } else {
                ""
            };
            Err(format!("获取视频信息失败: {}{}", raw, hint))
        }
    }
}

#[tauri::command]
pub async fn start_download(
    app: AppHandle,
    url: String,
    format_id: String,
    save_dir: Option<String>,
    title: Option<String>,
    config: State<'_, Arc<ConfigState>>,
    db: State<'_, Arc<Database>>,
) -> Result<String, String> {
    if url.trim().is_empty() {
        return Err("URL 不能为空".to_string());
    }
    if format_id.trim().is_empty() {
        return Err("未选择下载格式".to_string());
    }
    tracing::info!("download started: url={} format={}", url, format_id);

    // 启动前先校验关键二进制
    if let Err(e) = ytdlp::check_binaries() {
        return Err(e);
    }

    let download_id = uuid::Uuid::new_v4().to_string();
    let id_for_task = download_id.clone();
    let url_for_task = url.clone();
    // 真实 title 优先 (前端在 probe 后传入); 兜底用 URL
    let title_for_task = title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| url.clone());

    let snapshot = config.snapshot().map_err(|e| format!("读取配置失败: {}", e))?;
    let save_path = match save_dir {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => snapshot.default_save_dir.clone(),
    };
    let cookies_dir = snapshot.cookies_dir();
    let proxy = snapshot.proxy.clone();
    if let Err(e) = std::fs::create_dir_all(&save_path) {
        return Err(format!("无法创建下载目录 {}: {}", save_path.display(), e));
    }

    // 启动下载
    let child = runner::start_download(
        app.clone(),
        download_id.clone(),
        url.clone(),
        format_id,
        &save_path,
        &cookies_dir,
        proxy.as_deref(),
    )
    .await
    .map_err(|e| format!("启动下载失败: {}", e))?;

    // 保存 child handle 用于取消
    let active = app.state::<ActiveDownloads>();
    active.inner().lock().await.insert(download_id.clone(), child);

    // 等下载完成后写历史
    let db_clone = db.inner().clone();
    let save_path_clone = save_path.clone();
    let app_for_task = app.clone();
    let id_for_cleanup = download_id.clone();
    let id_for_log = download_id.clone();

    tauri::async_runtime::spawn(async move {
        // 等 child 退出
        let result = {
            let mut active = app_for_task.state::<ActiveDownloads>().inner().lock().await;
            if let Some(mut child) = active.remove(&id_for_task) {
                child.wait().await
            } else {
                tracing::info!("download canceled before start: id={}", id_for_task);
                return;
            }
        };

        match result {
            Ok(status) if status.success() => {
                tracing::info!("download finished: id={}", id_for_log);
                if let Err(e) = db_clone.add_history(
                    &url_for_task,
                    &title_for_task,
                    &save_path_clone.to_string_lossy(),
                    None,
                ) {
                    tracing::error!("写历史失败: {}", e);
                }
                let _ = app_for_task.emit(
                    &format!("download://{id_for_cleanup}"),
                    ProgressEvent {
                        id: id_for_cleanup.clone(),
                        percent: 100.0,
                        speed: None,
                        eta: Some(0),
                        downloaded_bytes: 0,
                        total_bytes: None,
                        status: "finished".to_string(),
                        message: None,
                    },
                );
            }
            Ok(status) => {
                tracing::error!("download failed: id={} exit_code={:?}", id_for_log, status.code());
                let _ = app_for_task.emit(
                    &format!("download://{id_for_cleanup}"),
                    ProgressEvent {
                        id: id_for_cleanup.clone(),
                        percent: 0.0,
                        speed: None,
                        eta: None,
                        downloaded_bytes: 0,
                        total_bytes: None,
                        status: "error".to_string(),
                        message: Some(format!("yt-dlp 退出码: {}", status.code().unwrap_or(-1))),
                    },
                );
            }
            Err(e) => {
                tracing::error!("download error: id={} err={}", id_for_log, e);
                let _ = app_for_task.emit(
                    &format!("download://{id_for_cleanup}"),
                    ProgressEvent {
                        id: id_for_cleanup.clone(),
                        percent: 0.0,
                        speed: None,
                        eta: None,
                        downloaded_bytes: 0,
                        total_bytes: None,
                        status: "error".to_string(),
                        message: Some(e.to_string()),
                    },
                );
            }
        }
    });

    Ok(download_id)
}

#[tauri::command]
pub async fn cancel_download(
    download_id: String,
    app: AppHandle,
) -> Result<(), String> {
    tracing::info!("cancel_download: {}", download_id);
    let active = app.state::<ActiveDownloads>();
    let mut map = active.inner().lock().await;
    if let Some(mut child) = map.remove(&download_id) {
        let _ = child.kill().await;
        Ok(())
    } else {
        Err("下载任务不存在或已完成".to_string())
    }
}

/// 注册全局 state (在 setup 里调)
pub fn register_state(app: &AppHandle) {
    app.manage(ActiveDownloads::default());
}
