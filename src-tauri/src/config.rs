use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 应用配置 (持久化在 `config.json`)
///
/// - 启动时通过 `Config::load_or_init` 加载
/// - 运行时包成 `ConfigState` 共享: 任何修改都走 `set` / `set_resource_dir`
///   → 锁住 Mutex → 改 Config 字段 → 落 config.json, 保持内存/磁盘一致
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
    #[serde(default = "default_save_dir")]
    pub default_save_dir: PathBuf,
    #[serde(default = "default_format_pref")]
    pub default_format_preference: String,
    #[serde(default)]
    pub proxy: Option<String>,
    /// Tauri bundle.resources 解压位置
    /// - dev:  通常在 src-tauri/target/debug 下
    /// - prod: 安装目录下, 一般是 `<install>/resources/`
    /// 由 lib.rs 在 setup 阶段从 `app.path().resource_dir()` 写入
    #[serde(default)]
    pub resource_dir: Option<PathBuf>,
}

fn default_data_dir() -> PathBuf { PathBuf::new() }
fn default_save_dir() -> PathBuf {
    if let Ok(p) = std::env::var("USERPROFILE") {
        PathBuf::from(p).join("Downloads")
    } else {
        PathBuf::from(".")
    }
}
fn default_format_pref() -> String { "bestvideo+bestaudio/best".to_string() }

impl Config {
    fn config_path(data_dir: &Path) -> PathBuf {
        data_dir.join("config.json")
    }

    fn write_to_disk(path: &Path, cfg: &Config) -> Result<()> {
        let text = serde_json::to_string_pretty(cfg)?;
        std::fs::write(path, text).context("写 config.json 失败")
    }

    /// 从磁盘加载; 不存在则写默认值
    pub fn load_or_init(data_dir: &Path) -> Result<Self> {
        let cfg_path = Self::config_path(data_dir);
        if cfg_path.exists() {
            let text = std::fs::read_to_string(&cfg_path).context("读 config.json 失败")?;
            let mut cfg: Config = serde_json::from_str(&text).context("解析 config.json 失败")?;
            cfg.data_dir = data_dir.to_path_buf();
            Ok(cfg)
        } else {
            let default_save = default_save_dir();
            std::fs::create_dir_all(&default_save).ok();
            let cfg = Config {
                data_dir: data_dir.to_path_buf(),
                default_save_dir: default_save,
                default_format_preference: "bestvideo+bestaudio/best".to_string(),
                proxy: None,
                resource_dir: None,
            };
            Self::write_to_disk(&cfg_path, &cfg)?;
            Ok(cfg)
        }
    }

    /// 写回 config.json
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path(&self.data_dir);
        Self::write_to_disk(&path, self)
    }

    /// 只在内存里改一个字段 (不落盘, 通常配合 save() 一起用)
    /// key 取值: "default_save_dir" | "default_format_preference" | "proxy" | "resource_dir"
    pub fn update_field(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "default_save_dir" => self.default_save_dir = PathBuf::from(value),
            "default_format_preference" => self.default_format_preference = value.to_string(),
            "proxy" => {
                self.proxy = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            "resource_dir" => {
                self.resource_dir = if value.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(value))
                };
            }
            other => anyhow::bail!("未知配置项: {}", other),
        }
        Ok(())
    }

    pub fn cookies_dir(&self) -> PathBuf {
        self.data_dir.join("cookies")
    }
}

/// 可在 Tauri 中通过 `Arc<ConfigState>` 共享的 Config state
/// 所有写入路径都先锁住内部 Mutex, 改完同步落盘, 不存在"内存改了但磁盘没改"的窗口
pub struct ConfigState {
    inner: Mutex<Config>,
}

impl ConfigState {
    pub fn new(cfg: Config) -> Self {
        Self { inner: Mutex::new(cfg) }
    }

    /// 拿 Config 的克隆快照 (调用方可以安全持有)
    pub fn snapshot(&self) -> Result<Config> {
        let guard = self.inner.lock().map_err(|e| anyhow::anyhow!("config mutex poisoned: {}", e))?;
        Ok(guard.clone())
    }

    /// 通用 setter: 改字段 + 持久化
    pub fn set(&self, key: &str, value: &str) -> Result<()> {
        let mut guard = self.inner.lock().map_err(|e| anyhow::anyhow!("config mutex poisoned: {}", e))?;
        guard.update_field(key, value)?;
        guard.save()
    }

    /// 启动期写入 resource_dir, 后续 ytdlp 路径查找会用到
    pub fn set_resource_dir(&self, dir: PathBuf) -> Result<()> {
        let value = dir.to_string_lossy().to_string();
        self.set("resource_dir", &value)
    }
}
