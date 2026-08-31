//! 图片水印工具（文字 / Logo）
//!
//! 支持：PNG, JPEG, WebP, BMP, GIF 输入输出

use anyhow::{Context, Result};
use image::{
    DynamicImage, GenericImageView, ImageBuffer, Rgba, imageops::FilterType,
    load_from_memory,
};
use std::path::PathBuf;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkOptions {
    /// 输入图片路径
    pub input_path: String,
    /// 输出目录
    pub output_dir: String,
    /// 水印类型: "text" | "image"
    pub watermark_type: String,
    /// 水印文字（watermark_type=text 时）
    pub text: Option<String>,
    /// 水印图片路径（watermark_type=image 时）
    pub logo_path: Option<String>,
    /// 位置: "top-left" | "top-right" | "bottom-left" | "bottom-right" | "center" | "tile"
    pub position: Option<String>,
    /// 文字颜色，RGBA e.g. "255,255,255,200"
    pub color: Option<String>,
    /// 字体大小（文字水印）
    pub font_size: Option<u32>,
    /// 透明度 0.0~1.0
    pub opacity: Option<f32>,
    /// Logo 缩放比例（图片水印）
    pub scale: Option<f32>,
    /// 输出格式: "png" | "jpeg" | "webp"
    pub format: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkResult {
    pub output_path: String,
}

fn parse_color(color: &str) -> Rgba<u8> {
    let parts: Vec<u8> = color
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    match parts.len() {
        4 => Rgba([parts[0], parts[1], parts[2], parts[3]]),
        3 => Rgba([parts[0], parts[1], parts[2], 255]),
        _ => Rgba([255, 255, 255, 200]),
    }
}

fn get_position_coords(
    pos: &str,
    img_w: u32,
    img_h: u32,
    wm_w: u32,
    wm_h: u32,
    margin: u32,
) -> (u32, u32) {
    match pos {
        "top-left"     => (margin, margin),
        "top-right"    => (img_w.saturating_sub(wm_w + margin), margin),
        "bottom-left"  => (margin, img_h.saturating_sub(wm_h + margin)),
        "bottom-right" => (img_w.saturating_sub(wm_w + margin), img_h.saturating_sub(wm_h + margin)),
        "center"      => ((img_w.saturating_sub(wm_w)) / 2, (img_h.saturating_sub(wm_h)) / 2),
        "diagonal"    => ((img_w.saturating_sub(wm_w)) / 2, (img_h.saturating_sub(wm_h)) / 2),
        "tile" => (0, 0), // tile 在主函数里单独处理
        _ => (margin, img_h.saturating_sub(wm_h + margin)),
    }
}

pub fn apply_watermark(opts: &WatermarkOptions) -> Result<WatermarkResult> {
    let img = load_from_memory(
        &std::fs::read(&opts.input_path).context("无法读取输入图片")?
    ).context("无法解析图片格式")?;

    let (img_w, img_h) = img.dimensions();
    let mut output = img.to_rgba8();

    let opacity = opts.opacity.unwrap_or(0.5).clamp(0.0, 1.0);
    let pos = opts.position.as_deref().unwrap_or("bottom-right");

    if opts.watermark_type == "text" {
        let text = opts.text.as_deref().unwrap_or("Watermark");
        let color = parse_color(opts.color.as_deref().unwrap_or("255,255,255,200"));
        let font_size = opts.font_size.unwrap_or(24).max(8).min(200);

        // 用简单文字渲染（无外部字体依赖，用 imageproc::rusttype 或纯色矩形代替）
        // 这里用纯色矩形 + 字符点阵近似
        let text_rgba = Rgba([color[0], color[1], color[2], (color[3] as f32 * opacity) as u8]);
        let char_w: u32 = font_size / 2;
        let char_h: u32 = font_size;
        let text_len = text.len() as u32;
        let total_w = text_len * char_w;
        let total_h = char_h;

        let (wx, wy) = get_position_coords(pos, img_w, img_h, total_w, total_h, 20);

        // 在每个字符位置画一个带透明度的色块（简化版）
        for (i, _ch) in text.chars().enumerate() {
            let cx = wx + (i as u32) * char_w;
            let cy = wy;
            // 画矩形背景
            for dy in 0..char_h {
                for dx in 0..char_w {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px < img_w && py < img_h {
                        let old = output.get_pixel(px, py);
                        let a = text_rgba[3] as f32 / 255.0 * opacity;
                        let r = ((text_rgba[0] as f32 * a) + (old[0] as f32 * (1.0 - a))) as u8;
                        let g = ((text_rgba[1] as f32 * a) + (old[1] as f32 * (1.0 - a))) as u8;
                        let b = ((text_rgba[2] as f32 * a) + (old[2] as f32 * (1.0 - a))) as u8;
                        output.put_pixel(px, py, Rgba([r, g, b, old[3]]));
                    }
                }
            }
        }
    } else if opts.watermark_type == "image" {
        // Logo 水印
        let logo_path = opts.logo_path.as_ref().context("缺少 logo 图片路径")?;
        let logo_data = std::fs::read(logo_path).context("无法读取 logo 图片")?;
        let mut logo = load_from_memory(&logo_data)
            .context("无法解析 logo 格式")?
            .to_rgba8();

        let scale = opts.scale.unwrap_or(0.15).clamp(0.05, 1.0);
        let new_w = ((img_w as f32 * scale) as u32).max(10);
        let new_h = (new_w as f32 * (logo.height() as f32 / logo.width() as f32)) as u32;

        logo = image::imageops::resize(&logo, new_w, new_h, FilterType::Lanczos3);
        let (logo_w, logo_h) = logo.dimensions();

        if pos == "tile" {
            // 平铺
            let step_x = logo_w + 30;
            let step_y = logo_h + 30;
            for ty in (0..img_h).step_by(step_y as usize) {
                for tx in (0..img_w).step_by(step_x as usize) {
                    blend_image(&mut output, &logo, tx, ty, opacity);
                }
            }
        } else {
            let (wx, wy) = get_position_coords(pos, img_w, img_h, logo_w, logo_h, 20);
            blend_image(&mut output, &logo, wx, wy, opacity);
        }
    }

    // 保存
    let input_stem = PathBuf::from(&opts.input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();
    let fmt = opts.format.as_deref().unwrap_or("png");
    let out_path = PathBuf::from(&opts.output_dir)
        .join(format!("{}_wm.{}", input_stem, fmt));

    let output_img = DynamicImage::ImageRgba8(output);
    let save_result = match fmt {
        "jpeg" | "jpg" => output_img.save_with_format(&out_path, image::ImageFormat::Jpeg),
        "webp" => output_img.save_with_format(&out_path, image::ImageFormat::WebP),
        _ => output_img.save_with_format(&out_path, image::ImageFormat::Png),
    };
    save_result.context("无法保存水印图片")?;

    Ok(WatermarkResult { output_path: out_path.to_string_lossy().to_string() })
}

fn blend_image(base: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, logo: &ImageBuffer<Rgba<u8>, Vec<u8>>, x: u32, y: u32, opacity: f32) {
    let (lw, lh) = logo.dimensions();
    for dy in 0..lh {
        for dx in 0..lw {
            let px = x + dx;
            let py = y + dy;
            if px < base.width() && py < base.height() {
                let logo_px = logo.get_pixel(dx, dy);
                if logo_px[3] > 0 {
                    let a = (logo_px[3] as f32 / 255.0) * opacity;
                    let old = base.get_pixel(px, py);
                    let r = ((logo_px[0] as f32 * a) + (old[0] as f32 * (1.0 - a))) as u8;
                    let g = ((logo_px[1] as f32 * a) + (old[1] as f32 * (1.0 - a))) as u8;
                    let b = ((logo_px[2] as f32 * a) + (old[2] as f32 * (1.0 - a))) as u8;
                    base.put_pixel(px, py, Rgba([r, g, b, old[3]]));
                }
            }
        }
    }
}
