use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum KVBits {
    #[default]
    #[serde(rename = "4bit")]
    Bits4,
    #[serde(rename = "8bit")]
    Bits8,
    #[serde(rename = "fp16")]
    FP16,
}

impl std::fmt::Display for KVBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KVBits::Bits4 => write!(f, "4bit"),
            KVBits::Bits8 => write!(f, "8bit"),
            KVBits::FP16 => write!(f, "fp16"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_host: String,
    pub api_port: u16,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_devices_refresh_interval")]
    pub devices_refresh_interval: u64,
    #[serde(default)]
    pub kv_bits: KVBits,
    #[serde(default = "default_max_batch_exp")]
    pub max_batch_exp: u8,
    #[serde(default = "default_seq_len")]
    pub seq_len: u32,
}

#[inline(always)]
#[rustfmt::skip]
fn default_max_tokens() -> u32  { 2000 }
#[inline(always)]
#[rustfmt::skip]
fn default_temperature() -> f32 { 0.7 }
#[inline(always)]
#[rustfmt::skip]
fn default_devices_refresh_interval() -> u64 { 1 }
#[inline(always)]
#[rustfmt::skip]
fn default_max_batch_exp() -> u8 { 2 }
#[inline(always)]
#[rustfmt::skip]
fn default_seq_len() -> u32 { 512 }

impl Default for Config {
    fn default() -> Self {
        Self {
            api_host: "127.0.0.1".to_string(),
            api_port: 8080,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            devices_refresh_interval: default_devices_refresh_interval(),
            kv_bits: KVBits::default(),
            max_batch_exp: default_max_batch_exp(),
            seq_len: default_seq_len(),
        }
    }
}

impl Config {
    /// Load config from either current directory or `~/.dria/dnet/` directory
    pub fn load() -> color_eyre::Result<Self> {
        // try current directory first
        let local_path = PathBuf::from("dnet.json");
        if local_path.exists() {
            let content = fs::read_to_string(&local_path)?;
            let config: Config = serde_json::from_str(&content)?;
            return Ok(config);
        }

        // try ~/.dria/dnet/ directory
        let dria_path = Self::dria_config_path();
        if dria_path.exists() {
            let content = fs::read_to_string(&dria_path)?;
            let config: Config = serde_json::from_str(&content)?;
            return Ok(config);
        }

        // if neither exists, create default config in current directory
        let config = Self::default();
        let content = serde_json::to_string_pretty(&config)?;
        fs::write(&local_path, content)?;
        Ok(config)
    }

    /// Save config to `~/.dria/dnet/` directory
    pub fn save_to_dria(&self) -> color_eyre::Result<()> {
        let config_path = Self::dria_config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get the path to `$HOME/.dria/dnet/dnet.json`
    ///
    /// FIXME: this is not cross-platform
    fn dria_config_path() -> PathBuf {
        let mut path = match std::env::var("HOME") {
            Ok(home) => PathBuf::from(home),
            Err(_) => PathBuf::from("."),
        };
        path.push(".dria");
        path.push("dnet");
        path.push("dnet.json");
        path
    }

    /// Get the current config location (for display purposes)
    pub fn current_location() -> String {
        let local_path = PathBuf::from("dnet.json");
        if local_path.exists() {
            return "./dnet.json".to_string();
        }

        let dria_path = Self::dria_config_path();
        if dria_path.exists() {
            return dria_path.to_string_lossy().to_string();
        }

        "./dnet.json (not found)".to_string()
    }

    /// Get the full API URL, `http://{host}:{port}` format
    pub fn api_url(&self) -> String {
        format!("http://{}:{}", self.api_host, self.api_port)
    }
}
