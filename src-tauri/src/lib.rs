// 禁用 console 窗口 (Windows release 模式)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
pub mod db;
pub mod types;
pub mod updater;
pub mod ytdlp;
mod tools;

use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 解析 %APPDATA%\video-toolbox\
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("无法获取 app data 目录");
            std::fs::create_dir_all(&data_dir).ok();
            std::fs::create_dir_all(data_dir.join("cookies")).ok();
            std::fs::create_dir_all(data_dir.join("logs")).ok();

            // 初始化文件日志: logs/app.YYYY-MM-DD
            let log_dir = data_dir.join("logs");
            let file_appender = tracing_appender::rolling::daily(&log_dir, "app.log");
            let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);
            // guard 需要 'static 生命周期,放进 Box
            let _guard_static: Box<dyn Send + Sync + 'static> = Box::new(_guard);

            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::INFO)
                .with_target(false)
                .with_ansi(false)
                .with_writer(file_writer)
                .init();

            tracing::info!("App started, data_dir={:?}", data_dir);

            // 初始化配置
            let config = Arc::new(config::ConfigState::new(
                config::Config::load_or_init(&data_dir)?,
            ));
            app.manage(config.clone());

            // 尝试设置 resource_dir (Tauri bundle.resources)
            // - 写到 Config (持久化到 config.json, 启动时也能看)
            // - 写到 ytdlp 模块全局 (路径查找 fallback 用)
            if let Ok(resource_dir) = app.path().resource_dir() {
                let _ = config.set_resource_dir(resource_dir.clone());
                ytdlp::set_resource_dir(resource_dir);
            }

            // 初始化数据库
            let db = Arc::new(db::Database::open(&data_dir.join("history.db"))?);
            app.manage(db);

            // 注册 download 模块的 state
            commands::download::register_state(&app.handle());

            // 启动时校验关键二进制
            if let Err(e) = ytdlp::check_binaries() {
                tracing::warn!("启动时二进制检查未通过: {}", e);
            }

            // 启动 bgutil POT server (用于 YouTube 反爬绕过)
            // 失败不致命 — 降级到旧 yt-dlp.exe 也能工作, 只是 YouTube 反爬会更严
            let bgutil_result = tauri::async_runtime::block_on(async {
                ytdlp::bgutil::ensure_bgutil_running().await
            });
            match bgutil_result {
                Ok(msg) => tracing::info!("bgutil: {}", msg),
                Err(e) => tracing::warn!("bgutil server 启动失败 (YouTube 反爬可能绕过不了): {}", e),
            }

            // bgutil server 跟随 app 生命周期: 它是用 kill_on_drop(true) 启动的,
            // process 句柄在 child_slot 里, app 退出时 Rust 会 drop, 自动 kill。
            // 这里无需显式 close hook。

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::download::probe_url,
            commands::download::start_download,
            commands::download::cancel_download,
            commands::download::open_in_browser,
            commands::download::open_parser_window,
            commands::cookies::import_cookies,
            commands::cookies::import_cookies_for_domain,
            commands::cookies::list_cookies,
            commands::cookies::delete_cookies,
            commands::history::list_history,
            commands::history::delete_history,
            commands::history::clear_history,
            commands::history::get_history_count,
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::set_default_save_dir,
            commands::settings::set_proxy,
            commands::settings::set_default_format_preference,
            commands::settings::open_save_dir,
            commands::settings::reveal_in_folder,
            commands::settings::check_binaries,
            commands::settings::get_bgutil_status,
            updater::check_update,
            updater::check_update_with_manifest,
            updater::open_msi_folder,
            updater::get_app_data_dir,
            updater::download_and_install,
            updater::auto_install_and_restart,
            commands::tools::extract_psd_layers,
            commands::tools::convert_ncm,
            commands::tools::apply_image_watermark,
            commands::tools::read_image_as_data_url,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Video Toolbox 失败");
}
