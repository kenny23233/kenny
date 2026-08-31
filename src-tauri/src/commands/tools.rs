//! 工具箱命令：PSD 提取 / NCM 转换 / 水印

use crate::tools::{self, NcmInfo, PsdResult, WatermarkOptions, WatermarkResult};
use base64::Engine;
use tauri::command;

#[command]
pub async fn extract_psd_layers(
    psd_path: String,
    output_dir: String,
) -> Result<PsdResult, String> {
    tools::extract_psd(&psd_path, &output_dir).map_err(|e| e.to_string())
}

#[command]
pub async fn convert_ncm(
    ncm_path: String,
    output_dir: String,
) -> Result<NcmInfo, String> {
    tools::process_ncm(&ncm_path, &output_dir).map_err(|e| e.to_string())
}

#[command]
pub async fn apply_image_watermark(
    options: WatermarkOptions,
) -> Result<WatermarkResult, String> {
    tools::apply_watermark(&options).map_err(|e| e.to_string())
}

/// 读取图片文件并返回 data URL（base64）供前端直接显示
/// 避免 Tauri 2 asset 协议在某些场景下加载本地图片失败的问题
#[command]
pub async fn read_image_as_data_url(path: String) -> Result<String, String> {
    let data = std::fs::read(&path).map_err(|e| format!("读取图片失败: {}", e))?;
    let ext = std::path::Path::new(&path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "gif" => "image/gif",
        _ => "application/octet-stream",
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
    Ok(format!("data:{};base64,{}", mime, b64))
}
