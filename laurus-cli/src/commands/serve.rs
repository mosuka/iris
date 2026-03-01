use std::path::Path;

use anyhow::Result;

use crate::cli::ServeCommand;

/// Start the gRPC server.
pub async fn run(cmd: ServeCommand, data_dir: &Path) -> Result<()> {
    let mut config = match &cmd.config {
        Some(path) => laurus_server::config::Config::from_file(path)?,
        None => laurus_server::config::Config::default(),
    };

    // Override with CLI arguments / environment variables when provided.
    if let Some(host) = cmd.host {
        config.server.host = host;
    }
    if let Some(port) = cmd.port {
        config.server.port = port;
    }
    if cmd.http_port.is_some() {
        config.server.http_port = cmd.http_port;
    }
    if let Some(log_level) = cmd.log_level {
        config.log.level = log_level;
    }
    config.index.data_dir = data_dir.to_path_buf();

    laurus_server::server::run(&config).await
}
