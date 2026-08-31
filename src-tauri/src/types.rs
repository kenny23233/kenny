use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub uploader: String,
    pub duration: u64,
    pub thumbnail: String,
    pub formats: Vec<FormatInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormatInfo {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub fps: Option<f64>,
    pub vcodec: String,
    pub acodec: String,
    pub filesize: Option<u64>,
    pub filesize_approx: Option<u64>,
    pub tbr: Option<f64>,
    pub format_note: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgressEvent {
    pub id: String,
    pub percent: f64,
    /// bytes/s
    pub speed: Option<f64>,
    /// seconds
    pub eta: Option<u64>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    /// "downloading" | "finished" | "error"
    pub status: String,
    pub message: Option<String>,
}
