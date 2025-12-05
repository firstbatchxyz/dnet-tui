use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::settings::SettingsField;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum KVBits {
    #[serde(rename = "4bit")]
    Bits4,
    #[default]
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

impl FromStr for KVBits {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "4bit" => Ok(KVBits::Bits4),
            "8bit" => Ok(KVBits::Bits8),
            "fp16" => Ok(KVBits::FP16),
            _ => Err(color_eyre::eyre::eyre!("Invalid KV Bits value: {}", s)),
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

impl Config {
    pub fn read_setting(&self, selection: SettingsField) -> String {
        match selection {
            SettingsField::Host => self.api_host.clone(),
            SettingsField::Port => self.api_port.to_string(),
            SettingsField::MaxTokens => self.max_tokens.to_string(),
            SettingsField::Temperature => format!("{:.2}", self.temperature),
            SettingsField::DevicesRefreshInterval => self.devices_refresh_interval.to_string(),
            SettingsField::KVBits => self.kv_bits.to_string(),
            SettingsField::MaxBatchExp => self.max_batch_exp.to_string(),
            SettingsField::SeqLen => self.seq_len.to_string(),
        }
    }

    pub fn write_setting(
        &mut self,
        selection: SettingsField,
        value: &str,
    ) -> color_eyre::Result<()> {
        match selection {
            SettingsField::Host => self.api_host = value.to_string(),
            SettingsField::Port => self.api_port = value.parse()?,
            SettingsField::MaxTokens => {
                self.max_tokens = value.parse().map(|t: u32| t.clamp(1, 100000))?
            }
            SettingsField::Temperature => {
                self.temperature = value.parse().map(|t: f32| t.clamp(0.0, 2.0))?
            }
            SettingsField::DevicesRefreshInterval => {
                self.devices_refresh_interval = value.parse().map(|t: u64| t.clamp(1, 3600))?;
            }
            SettingsField::KVBits => self.kv_bits = value.parse()?,
            SettingsField::MaxBatchExp => {
                self.max_batch_exp = value.parse().map(|t: u8| t.clamp(1, 8))?
            }
            SettingsField::SeqLen => {
                self.seq_len = value.parse().map(|t: u32| t.clamp(0, 999_999))?
            }
        }

        Ok(())
    }
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
fn default_seq_len() -> u32 { 4096 }

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
    pub const FILE_NAME: &'static str = "dnet.json";
    /// Load config from either current directory or `~/.dria/dnet/` directory
    pub fn load() -> color_eyre::Result<Self> {
        // try current directory first
        let local_path = PathBuf::from(Self::FILE_NAME);
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
        path.extend([".dria", "dnet", Self::FILE_NAME]);
        path
    }

    /// Get the current config location (for display purposes)
    pub fn current_location() -> String {
        let local_path = PathBuf::from(Self::FILE_NAME);
        if local_path.exists() {
            return format!("./{}", Self::FILE_NAME);
        }

        let dria_path = Self::dria_config_path();
        if dria_path.exists() {
            return dria_path.to_string_lossy().to_string();
        }

        format!("./{} (not found)", Self::FILE_NAME)
    }

    /// Get the full API URL, `http://{host}:{port}` format
    pub fn api_url(&self) -> String {
        format!("http://{}:{}", self.api_host, self.api_port)
    }
}
