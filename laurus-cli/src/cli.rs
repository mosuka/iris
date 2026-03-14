//! CLI argument definitions for the laurus command-line tool.
//!
//! This module defines the top-level [`Cli`] struct and all subcommand
//! structures parsed by [`clap`]. Each subcommand maps to a specific
//! operation such as creating an index, querying documents, or starting
//! a gRPC server.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::output::OutputFormat;

/// Laurus - Unified search engine CLI
#[derive(Parser)]
#[command(name = "laurus", version, about)]
pub struct Cli {
    /// Path to the data directory.
    #[arg(long, env = "LAURUS_DATA_DIR", default_value = "./laurus_data")]
    pub data_dir: PathBuf,

    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a resource.
    Create(CreateCommand),
    /// Get a resource.
    Get(GetCommand),
    /// Add a resource.
    Add(AddCommand),
    /// Delete a resource.
    Delete(DeleteCommand),
    /// Commit pending changes.
    Commit,
    /// Execute a search query.
    Search(SearchCommand),
    /// Start an interactive REPL session.
    Repl,
    /// Start the gRPC server.
    Serve(ServeCommand),
}

// --- Create ---

/// CLI arguments for the `create` subcommand.
///
/// Holds the target resource to create (e.g. an index or a schema file).
#[derive(Parser)]
pub struct CreateCommand {
    #[command(subcommand)]
    pub resource: CreateResource,
}

#[derive(Subcommand)]
pub enum CreateResource {
    /// Create a new index from a schema TOML file.
    Index {
        /// Path to the schema TOML file.
        #[arg(long)]
        schema: PathBuf,
    },
    /// Interactively generate a schema TOML file.
    Schema {
        /// Output file path for the generated schema TOML.
        #[arg(long, default_value = "schema.toml")]
        output: PathBuf,
    },
}

// --- Get ---

/// CLI arguments for the `get` subcommand.
///
/// Holds the target resource to retrieve (e.g. index stats or a document).
#[derive(Parser)]
pub struct GetCommand {
    #[command(subcommand)]
    pub resource: GetResource,
}

#[derive(Subcommand)]
pub enum GetResource {
    /// Show index statistics.
    Index,
    /// Get a document by ID.
    Doc {
        /// External document ID.
        #[arg(long)]
        id: String,
    },
}

// --- Add ---

/// CLI arguments for the `add` subcommand.
///
/// Holds the target resource to add (e.g. a document).
#[derive(Parser)]
pub struct AddCommand {
    #[command(subcommand)]
    pub resource: AddResource,
}

#[derive(Subcommand)]
pub enum AddResource {
    /// Add a document to the index.
    Doc {
        /// External document ID.
        #[arg(long)]
        id: String,
        /// Document data as a JSON string.
        #[arg(long)]
        data: String,
    },
}

// --- Delete ---

/// CLI arguments for the `delete` subcommand.
///
/// Holds the target resource to delete (e.g. a document by ID).
#[derive(Parser)]
pub struct DeleteCommand {
    #[command(subcommand)]
    pub resource: DeleteResource,
}

#[derive(Subcommand)]
pub enum DeleteResource {
    /// Delete a document by ID.
    Doc {
        /// External document ID.
        #[arg(long)]
        id: String,
    },
}

// --- Serve ---

/// CLI arguments for the `serve` subcommand.
///
/// Configures the gRPC server (and optional HTTP gateway) including
/// listen address, ports, and an optional TOML configuration file.
/// Values can be supplied via CLI flags or environment variables.
/// Use the `RUST_LOG` environment variable to control log verbosity.
#[derive(Parser)]
pub struct ServeCommand {
    /// Path to the configuration file (TOML).
    #[arg(short = 'c', long = "config", env = "LAURUS_CONFIG")]
    pub config: Option<PathBuf>,

    /// Listen address.
    #[arg(short = 'H', long = "host", env = "LAURUS_HOST")]
    pub host: Option<String>,

    /// Listen port.
    #[arg(short = 'p', long = "port", env = "LAURUS_PORT")]
    pub port: Option<u16>,

    /// HTTP Gateway port. If set, starts an HTTP gateway alongside the gRPC server.
    #[arg(long = "http-port", env = "LAURUS_HTTP_PORT")]
    pub http_port: Option<u16>,
}

// --- Search ---

/// CLI arguments for the `search` subcommand.
///
/// Accepts a query string written in the Laurus query DSL along with
/// pagination parameters (`limit` and `offset`).
#[derive(Parser)]
pub struct SearchCommand {
    /// Search query string (Laurus query DSL).
    pub query: String,

    /// Maximum number of results.
    #[arg(long, default_value_t = 10)]
    pub limit: usize,

    /// Number of results to skip.
    #[arg(long, default_value_t = 0)]
    pub offset: usize,
}
