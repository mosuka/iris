//! Implementations for the `get` subcommand.
//!
//! Handles retrieving information from an existing index:
//!
//! - [`run_stats`] - Show index statistics.
//! - [`run_schema`] - Show the current schema.
//! - [`run_doc`] - Get a document by ID.

use std::path::Path;

use anyhow::{Context, Result};

use crate::context;
use crate::output::{self, OutputFormat};

/// Execute the `get stats` command.
///
/// Opens the index at `index_dir` and prints index-level statistics
/// (document count, vector field stats, etc.).
///
/// # Arguments
///
/// * `index_dir` - Path to the index directory holding the index.
/// * `format` - The desired output format (table or JSON).
///
/// # Errors
///
/// Returns an error if:
/// - The index cannot be opened.
/// - Statistics cannot be retrieved.
pub async fn run_stats(index_dir: &Path, format: OutputFormat) -> Result<()> {
    let engine = context::open_index(index_dir).await?;
    let stats = engine.stats()?;
    output::print_stats(&stats, format);
    Ok(())
}

/// Execute the `get schema` command.
///
/// Reads the schema from `index_dir` and prints it as JSON.
///
/// # Arguments
///
/// * `index_dir` - Path to the index directory holding the index.
///
/// # Errors
///
/// Returns an error if:
/// - The schema file cannot be read or parsed.
/// - JSON serialization fails.
pub fn run_schema(index_dir: &Path) -> Result<()> {
    let schema = context::read_schema(index_dir)?;
    let json =
        serde_json::to_string_pretty(&schema).context("Failed to serialize schema to JSON")?;
    println!("{json}");
    Ok(())
}

/// Execute the `get doc` command.
///
/// Opens the index at `index_dir` and retrieves the document with the
/// given external ID.
///
/// # Arguments
///
/// * `id` - External document ID.
/// * `index_dir` - Path to the index directory holding the index.
/// * `format` - The desired output format (table or JSON).
///
/// # Errors
///
/// Returns an error if:
/// - The index cannot be opened.
/// - The document cannot be retrieved.
pub async fn run_doc(id: &str, index_dir: &Path, format: OutputFormat) -> Result<()> {
    let engine = context::open_index(index_dir).await?;
    let documents = engine.get_documents(id).await?;
    output::print_documents(id, &documents, format);
    Ok(())
}
