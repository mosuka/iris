use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level configuration loaded from a TOML file.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub log: LogConfig,
}

/// Server network settings.
#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

/// Index storage settings.
#[derive(Debug, Deserialize)]
pub struct IndexConfig {
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
        }
    }
}

/// Logging settings.
#[derive(Debug, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    50051
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("./laurus_data")
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    /// Load configuration from a TOML file.
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

}
