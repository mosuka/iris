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
}

// --- Create ---

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

// --- Search ---

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
