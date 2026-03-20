//! Implementation for the `commit` subcommand.
//!
//! Commits all pending changes (additions and deletions) to the index,
//! making them visible to search.

use std::path::Path;

use anyhow::Result;

use crate::context;

/// Execute the `commit` command.
///
/// Opens the index at `index_dir` and commits all pending changes.
///
/// # Arguments
///
/// * `index_dir` - Path to the index directory holding the index.
///
/// # Errors
///
/// Returns an error if:
/// - The index cannot be opened.
/// - The commit operation fails.
pub async fn run(index_dir: &Path) -> Result<()> {
    let engine = context::open_index(index_dir).await?;
    engine.commit().await?;
    println!("Changes committed successfully.");
    Ok(())
}
