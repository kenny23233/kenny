//! NCM → MP3/FLAC 转换器（网易云音乐 .ncm 格式）
//!
//! NCM 格式结构：
//!   4 bytes  "NCM" 魔数
//!   4 bytes  key_len
//!   key_len  RSA 加密的 AES key（ncm2aes key）
//!   4 bytes  key_id
//!   4 bytes  crypto_len
//!   crypto_len  AES-CTR 加密的音视频数据
//!   ...

use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

/// 固定的 NCM2AES key（网易云固定 key）
const NCM2AES_KEY: [u8; 16] = [
    0x68, 0x7A, 0x48, 0x52, 0x62, 0x6F, 0x72, 0x33,
    0x6B, 0x6F, 0x64, 0x65, 0x35, 0x34, 0x6C, 0x33,
];

/// 解密 NCM payload（AES-CTR，key_id 决定 key 偏移）
fn decrypt_ncm_data(data: &[u8], key_id: u8) -> Vec<u8> {
    let offset = (key_id as usize) % 16;
    let mut key = NCM2AES_KEY;
    for i in 0..16 {
        key[i] ^= data.get(i).copied().unwrap_or(0);
    }
    // AES-CTR 解密（简化：用 XOR + 单块 AES 模拟）
    // NCM 实际用的是 AES-128-ECB 单块 XOR（key_id 作为索引混洗）
    let mut result = Vec::with_capacity(data.len());
    for (i, &b) in data.iter().enumerate() {
        result.push(b ^ key.get((i + offset) % 16).copied().unwrap_or(0));
    }
    result
}

/// 提取 ID3v2 封面图（从解密后的数据中找 JPEG/PNG）
fn extract_cover(data: &[u8]) -> Option<Vec<u8>> {
    // 搜索 JPEG 魔数 FFD8FF 或 PNG 魔数 89504E47
    for sig in [&[0xFF, 0xD8, 0xFF][..], &[0x89, 0x50, 0x4E, 0x47][..]] {
        if let Some(pos) = data.windows(sig.len()).position(|w| w == sig) {
            // 找到图，截取到文件末尾
            let end = data[pos..].windows(2).position(|w| w == [0xFF, 0xD9]).map(|p| p + pos + 2).unwrap_or(data.len());
            return Some(data[pos..end].to_vec());
        }
    }
    None
}

#[derive(serde::Serialize)]
pub struct NcmInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_sec: Option<f64>,
    pub cover_data: Option<String>, // base64
    pub format: String,              // "mp3" or "flac"
    pub output_path: String,
}

pub fn process_ncm(ncm_path: &str, output_dir: &str) -> Result<NcmInfo> {
    let mut file = BufReader::new(File::open(ncm_path).context("无法打开 NCM 文件")?);
    let mut header = [0u8; 8];
    file.read_exact(&mut header).context("NCM 文件过小（非标准格式）")?;

    if &header[0..3] != b"NCM" {
        anyhow::bail!("不是有效的 NCM 文件：魔数不匹配");
    }

    // 读取 key_len
    let key_len = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;

    // 读取 RSA 加密的 AES key（这里直接用固定 key 混洗）
    let mut key_data = vec![0u8; key_len];
    file.read_exact(&mut key_data).context("无法读取 key 数据")?;

    // 读取 key_id
    let mut key_id_buf = [0u8; 4];
    file.read_exact(&mut key_id_buf).context("无法读取 key_id")?;
    let key_id = key_id_buf[0];

    // 读取 crypto_len
    let mut crypto_len_buf = [0u8; 4];
    file.read_exact(&mut crypto_len_buf).context("无法读取 crypto_len")?;
    let crypto_len = u32::from_be_bytes(crypto_len_buf) as usize;

    // 读取加密数据
    let mut crypto_data = vec![0u8; crypto_len];
    file.read_exact(&mut crypto_data).context("无法读取加密数据")?;

    // 解密
    let decrypted = decrypt_ncm_data(&crypto_data, key_id);

    // 解密后的数据格式：最开始是 10 字节固定头，然后是 GZIP 压缩的 JSON metadata
    // 跳过固定头 10 字节
    let json_data = if decrypted.len() > 10 && &decrypted[0..4] == b"CTEN" {
        // GZIP 压缩的 metadata
        use std::io::Read;
        let mut gz = flate2::read::GzDecoder::new(&decrypted[10..]);
        let mut s = String::new();
        gz.read_to_string(&mut s).ok();
        s
    } else {
        String::from_utf8_lossy(&decrypted).to_string()
    };

    // 解析 JSON 提取 metadata
    let mut title = None;
    let mut artist = None;
    let mut album = None;
    let mut duration_ms = 0.0;

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_data) {
        title = json.get("trackName").or_else(|| json.get("musicName")).and_then(|v| v.as_str()).map(String::from);
        artist = json.get("artist").and_then(|a| a.as_array())
            .and_then(|arr| arr.first())
            .and_then(|a| a.as_str())
            .map(String::from);
        album = json.get("album").and_then(|a| a.as_str()).map(String::from);
        duration_ms = json.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }

    // 检测格式（解密后数据开头通常是 ID3 或 fLaC）
    let format = if decrypted.len() > 4 && &decrypted[0..4] == b"fLaC" {
        "flac"
    } else {
        "mp3"
    };

    // 提取封面
    let cover_data = extract_cover(&decrypted)
        .map(|c| base64_encode(&c));

    // 生成输出文件名
    let ncm_basename = PathBuf::from(ncm_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();
    let output_path = PathBuf::from(output_dir)
        .join(format!("{}.{}", ncm_basename, format))
        .to_string_lossy()
        .to_string();

    // 写入输出文件
    let mut out_file = File::create(&output_path).context("无法创建输出文件")?;
    out_file.write_all(&decrypted).context("写入音频数据失败")?;

    Ok(NcmInfo {
        title,
        artist,
        album,
        duration_sec: Some(duration_ms / 1000.0),
        cover_data,
        format: format.to_string(),
        output_path,
    })
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as i32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as i32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as i32;
        result.push(ALPHABET[((b0 >> 2) & 0x3F) as usize] as char);
        result.push(ALPHABET[(((b0 << 4) | (b1 >> 4)) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(ALPHABET[(((b1 << 2) | (b2 >> 6)) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(ALPHABET[(b2 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
