//! PSD/PSB 图层提取 → PNG
//!
//! 使用 `image` crate 内置 PSD 支持读取合并图像，
//! 图层信息通过解析 PSD 二进制结构提取。

use anyhow::{Context, Result};
use image::GenericImageView;
use std::fs;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PsdLayerInfo {
    pub index: usize,
    pub name: String,
    pub top: u32,
    pub left: u32,
    pub width: u32,
    pub height: u32,
    pub visible: bool,
    pub output_path: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PsdResult {
    pub composite_path: String,
    pub layers: Vec<PsdLayerInfo>,
    pub layer_count: usize,
}

fn read_u32<R: Read>(r: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn read_u16<R: Read>(r: &mut R) -> Result<u16> {
    let mut buf = [0u8; 2];
    r.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn read_tag<R: Read>(r: &mut R) -> Result<String> {
    let mut buf = vec![0u8; 4];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn read_pascal_string<R: Read>(r: &mut R) -> Result<String> {
    let len = r.bytes().next().ok_or_else(|| anyhow::anyhow!("EOF"))?? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    if len % 2 == 0 {
        let mut pad = [0u8; 1];
        r.read_exact(&mut pad)?;
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}

pub fn extract_psd(psd_path: &str, output_dir: &str) -> Result<PsdResult> {
    let data = fs::read(psd_path).context("无法读取 PSD/PSB 文件")?;

    // 尝试用 image crate 解析 PSD（支持复合图）
    let img = match image::load_from_memory(&data) {
        Ok(i) => i,
        Err(e) => anyhow::bail!("无法解析 PSD 文件（{}），可能是PSB大文件或不支持的格式: {}", psd_path, e),
    };

    let (_width, _height) = img.dimensions();

    // 输出复合图
    let psd_basename = PathBuf::from(psd_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();
    let composite_path = PathBuf::from(output_dir)
        .join(format!("{}_composite.png", psd_basename))
        .to_string_lossy()
        .to_string();

    img.save_with_format(&composite_path, image::ImageFormat::Png)
        .context("无法保存复合图")?;

    // 解析图层信息（从二进制结构）
    let mut cursor = Cursor::new(&data);
    let mut layers = Vec::new();

    // 定位到 layer info start
    // PSD header: 26 + color mode data + image resources + layer mask
    // 简化：跳过头部 26 字节
    cursor.seek(SeekFrom::Start(26))?;

    // 读取并跳过 color mode data
    let cmd_len = read_u32(&mut cursor)? as u64;
    cursor.seek(SeekFrom::Current(cmd_len as i64))?;

    // 读取并跳过 image resources
    let ird_len = read_u32(&mut cursor)? as u64;
    cursor.seek(SeekFrom::Current(ird_len as i64))?;

    // Layer and mask data
    let lmd_len = read_u32(&mut cursor)? as u64;
    if lmd_len > 4 {
        cursor.seek(SeekFrom::Current(4))?; // sig + len
        let layer_count = read_u16(&mut cursor)? as usize;

        for i in 0..layer_count {
            let top = read_u32(&mut cursor)?;
            let left = read_u32(&mut cursor)?;
            let bottom = read_u32(&mut cursor)?;
            let right = read_u32(&mut cursor)?;
            let h = bottom - top;
            let w = right - left;

            // 跳过 channel info (6 * 2 bytes per channel)
            let channel_count = read_u16(&mut cursor)?;
            for _c in 0..channel_count {
                cursor.seek(SeekFrom::Current(6))?;
            }

            // 读取图层名称（Pascal string, 4 bytes length prefix）
            let name_len = cursor.by_ref().bytes().next().ok_or_else(|| anyhow::anyhow!("EOF"))?? as usize;
            let mut name_buf = vec![0u8; name_len];
            cursor.read_exact(&mut name_buf)?;
            let layer_name = String::from_utf8_lossy(&name_buf).to_string();

            // 跳过 blend flags + extra data
            cursor.seek(SeekFrom::Current(12))?;

            layers.push(PsdLayerInfo {
                index: i,
                name: layer_name,
                top,
                left,
                width: w,
                height: h,
                visible: true,
                output_path: None,
            });
        }
    }

    Ok(PsdResult {
        composite_path,
        layer_count: layers.len(),
        layers,
    })
}
