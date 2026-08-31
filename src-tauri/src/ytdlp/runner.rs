use crate::types::{FormatInfo, ProgressEvent, VideoInfo};
use crate::ytdlp::{extract_domain, ffmpeg_path, ytdlp_invocation};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Stdio;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// YouTube 域名 — 触发 bgutil POT + mweb client 配置
const YOUTUBE_DOMAINS: &[&str] = &["youtube.com", "youtu.be", "youtube-nocookie.com", "m.youtube.com"];

fn is_youtube_url(url: &str) -> bool {
    YOUTUBE_DOMAINS.iter().any(|d| {
        url.contains(&format!("://{}", d))
            || url.contains(&format!("://www.{}", d))
            || url.contains(&format!("://m.{}", d))
    })
}

/// 把通用 yt-dlp 参数构建出来 (含 bgutil / deno / youtube 调优)
fn build_base_args() -> Vec<String> {
    let mut args: Vec<String> = Vec::new();

    // 1. JS runtime (yt-dlp 2026.08+ 需要)
    if let Some(deno) = crate::ytdlp::bgutil::deno_path() {
        args.push("--js-runtimes".into());
        args.push(format!("deno:{}", deno.display()));
    }

    // 2. bgutil POT plugin (HTTP mode, 永远加上 — server 监听 4416, plugin 默认 base_url)
    if crate::ytdlp::bgutil::is_bgutil_already_running() {
        args.push("--extractor-args".into());
        args.push(format!("youtubepot-bgutilhttp:base_url={}", crate::ytdlp::bgutil::BGUTIL_DEFAULT_URL));
    }

    args
}

/// YouTube 专用调优参数 (player_client + fetch_pot 强制)
fn youtube_specific_args() -> Vec<String> {
    vec![
        "--extractor-args".to_string(),
        "youtube:player_client=mweb;fetch_pot=always".to_string(),
    ]
}

/// 把 proxy 字符串应用到 yt-dlp 命令
/// - 通过 --proxy 参数传递, yt-dlp 自己负责注入 HTTP_PROXY/HTTPS_PROXY 环境变量
/// - 顺便设 NO_PROXY 让本地 bgutil server (127.0.0.1) 直连不走代理
fn apply_proxy(cmd: &mut Command, proxy: Option<&str>) {
    let proxy = proxy.filter(|p| !p.trim().is_empty());
    if let Some(p) = proxy {
        cmd.arg("--proxy").arg(p);
    }
    // 始终让本地 bgutil server (127.0.0.1) 不走代理
    cmd.env("NO_PROXY", "localhost,127.0.0.1,::1");
    cmd.env("no_proxy", "localhost,127.0.0.1,::1");
}

#[derive(Deserialize, Debug)]
struct YtdlpFormatRaw {
    format_id: String,
    ext: String,
    resolution: Option<String>,
    fps: Option<f64>,
    vcodec: Option<String>,
    acodec: Option<String>,
    filesize: Option<u64>,
    filesize_approx: Option<u64>,
    tbr: Option<f64>,
    format_note: Option<String>,
    height: Option<u32>,
    width: Option<u32>,
}

#[derive(Deserialize, Debug)]
struct YtdlpInfoRaw {
    id: String,
    title: String,
    uploader: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    formats: Vec<YtdlpFormatRaw>,
}

/// 解析视频 (不下载)
pub async fn probe(url: &str, cookies_dir: &Path, proxy: Option<&str>) -> Result<VideoInfo> {
    let (program, prefix) = ytdlp_invocation();
    let mut cmd = Command::new(&program);
    for a in &prefix {
        cmd.arg(a);
    }
    cmd.args([
        "--dump-json",
        "--no-download",
        "--no-warnings",
        "--no-playlist",
        "--skip-download",
    ]);

    // 代理设置 (从 config.proxy 拿)
    apply_proxy(&mut cmd, proxy);

    // bgutil / deno / youtube 调优
    for a in build_base_args() {
        cmd.arg(a);
    }
    if is_youtube_url(url) {
        for a in youtube_specific_args() {
            cmd.arg(a);
        }
    }

    if let Some(ff) = ffmpeg_path() {
        cmd.env("FFMPEG_LOCATION", ff);
    }

    if let Some(domain) = extract_domain(url) {
        let cookie_file = cookies_dir.join(format!("{}.txt", domain));
        if cookie_file.exists() {
            cmd.arg("--cookies").arg(&cookie_file);
        }
    }

    cmd.arg(url);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd.output().await.map_err(|e| anyhow!("启动 yt-dlp 失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("yt-dlp 错误: {}", stderr));
    }

    let info: YtdlpInfoRaw = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow!("解析 yt-dlp 输出失败: {}", e))?;

    Ok(VideoInfo {
        id: info.id,
        title: info.title,
        uploader: info.uploader.unwrap_or_default(),
        duration: info.duration.map(|d| d as u64).unwrap_or(0),
        thumbnail: info.thumbnail.unwrap_or_default(),
        formats: info
            .formats
            .into_iter()
            .map(|f| FormatInfo {
                format_id: f.format_id,
                ext: f.ext,
                resolution: f.resolution.unwrap_or_else(|| {
                    if let (Some(w), Some(h)) = (f.width, f.height) {
                        format!("{}x{}", w, h)
                    } else {
                        String::new()
                    }
                }),
                fps: f.fps,
                vcodec: f.vcodec.unwrap_or_default(),
                acodec: f.acodec.unwrap_or_default(),
                filesize: f.filesize,
                filesize_approx: f.filesize_approx,
                tbr: f.tbr,
                format_note: f.format_note.unwrap_or_default(),
            })
            .collect(),
    })
}

/// 启动下载, 返回 child handle
pub async fn start_download(
    app: AppHandle,
    download_id: String,
    url: String,
    format_id: String,
    save_dir: &Path,
    cookies_dir: &Path,
    proxy: Option<&str>,
) -> Result<Child> {
    let (program, prefix) = ytdlp_invocation();
    let mut cmd = Command::new(&program);
    for a in &prefix {
        cmd.arg(a);
    }
    cmd.args([
        "--no-playlist",
        "--no-warnings",
        "--newline",
        "--no-part",
        "-f",
        &format_id,
        "-o",
        &format!(
            "{}/%(title).100B [%(id)s].%(ext)s",
            save_dir.to_string_lossy()
        ),
        "--progress-template",
        "download:%(progress._percent_str)s|speed:%(progress.speed)s|eta:%(progress.eta)s|downloaded:%(progress.downloaded_bytes)s|total:%(progress.total_bytes)s",
    ]);

    // 代理设置
    apply_proxy(&mut cmd, proxy);

    // bgutil / deno / youtube 调优
    for a in build_base_args() {
        cmd.arg(a);
    }
    if is_youtube_url(&url) {
        for a in youtube_specific_args() {
            cmd.arg(a);
        }
    }

    if let Some(ff) = ffmpeg_path() {
        cmd.env("FFMPEG_LOCATION", ff);
    }

    if let Some(domain) = extract_domain(&url) {
        let cookie_file = cookies_dir.join(format!("{}.txt", domain));
        if cookie_file.exists() {
            cmd.arg("--cookies").arg(&cookie_file);
        }
    }

    cmd.arg(&url);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| anyhow!("启动 yt-dlp 失败: {}", e))?;

    let stdout = child.stdout.take().expect("yt-dlp stdout 应可读");
    let stderr = child.stderr.take().expect("yt-dlp stderr 应可读");

    let app_for_stdout = app.clone();
    let dl_id_for_stdout = download_id.clone();

    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            if let Some(mut progress) = parse_progress_line(&line) {
                progress.id = dl_id_for_stdout.clone();
                let _ = app_for_stdout.emit(
                    &format!("download://{dl_id_for_stdout}"),
                    progress,
                );
            }
        }
    });

    let app_for_stderr = app.clone();
    let dl_id_for_stderr = download_id.clone();
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut err_buf = String::new();
        while let Ok(Some(line)) = reader.next_line().await {
            err_buf.push_str(&line);
            err_buf.push('\n');
            tracing::warn!("[yt-dlp stderr] {}", line);
        }
        if !err_buf.trim().is_empty() {
            let _ = app_for_stderr.emit(
                &format!("download://{dl_id_for_stderr}"),
                ProgressEvent {
                    id: dl_id_for_stderr.clone(),
                    percent: 0.0,
                    speed: None,
                    eta: None,
                    downloaded_bytes: 0,
                    total_bytes: None,
                    status: "error".to_string(),
                    message: Some(err_buf),
                },
            );
        }
    });

    Ok(child)
}

fn parse_progress_line(line: &str) -> Option<ProgressEvent> {
    if !line.starts_with("download:") {
        return None;
    }
    let mut percent = 0.0;
    let mut speed: Option<f64> = None;
    let mut eta: Option<u64> = None;
    let mut downloaded: u64 = 0;
    let mut total: Option<u64> = None;

    for part in line.split('|') {
        let (k, v) = part.split_once(':')?;
        match k {
            "download" => {
                let s = v.trim_end_matches('%');
                percent = s.parse().unwrap_or(0.0);
            }
            "speed" => speed = v.parse().ok(),
            "eta" => eta = v.parse().ok(),
            "downloaded" => {
                downloaded = v.parse().unwrap_or(0);
            }
            "total" => total = v.parse().ok(),
            _ => {}
        }
    }

    Some(ProgressEvent {
        id: String::new(),
        percent,
        speed,
        eta,
        downloaded_bytes: downloaded,
        total_bytes: total,
        status: "downloading".to_string(),
        message: None,
    })
}
