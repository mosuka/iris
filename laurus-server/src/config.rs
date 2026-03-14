//! Server configuration types deserialized from a TOML file.
//!
//! The top-level [`Config`] struct contains sections for the gRPC/HTTP server,
//! index storage, and logging. All sections have sensible defaults so that
//! a minimal (or even empty) TOML file produces a working configuration.

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Top-level configuration loaded from a TOML file.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// Network settings for the gRPC server and the optional HTTP gateway.
    #[serde(default)]
    pub server: ServerConfig,
    /// Index storage settings (e.g. data directory path).
    #[serde(default)]
    pub index: IndexConfig,
}

/// Server network configuration.
#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// Listen address for the gRPC server.
    #[serde(default = "default_host")]
    pub host: String,
    /// Listen port for the gRPC server.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Listen port for the HTTP Gateway. The Gateway is started only when this is set.
    #[serde(default)]
    pub http_port: Option<u16>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            http_port: None,
        }
    }
}

/// Index storage settings.
#[derive(Debug, Deserialize)]
pub struct IndexConfig {
    /// Filesystem path where the index data (schema and store) is persisted.
    /// Defaults to `"./laurus_data"`.
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

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    50051
}

fn default_data_dir() -> PathBuf {
    PathBuf::from("./laurus_data")
}

impl Config {
    /// Load configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Filesystem path to the TOML configuration file.
    ///
    /// # Returns
    ///
    /// A fully populated [`Config`] instance with defaults applied for any
    /// missing sections or fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if the TOML content
    /// cannot be deserialized into a [`Config`].
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
