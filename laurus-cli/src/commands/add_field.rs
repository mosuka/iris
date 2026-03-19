//! Dynamically add a new field to an existing index.
//!
//! Opens the index, deserializes the field option from a JSON string,
//! registers the field in the engine, and persists the updated schema
//! back to `schema.toml`.

use std::path::Path;

use anyhow::{Context, Result};
use laurus::FieldOption;

use crate::context;

/// Execute the `add field` command.
///
/// Opens the index at `data_dir`, parses the field option JSON, calls
/// [`Engine::add_field`], and writes the updated schema to disk.
///
/// # Arguments
///
/// * `name` - The name of the new field to add.
/// * `field_option_json` - A JSON string describing the field configuration
///   (e.g. `{"Text": {"indexed": true, "stored": true}}`).
/// * `data_dir` - Path to the data directory holding the index.
///
/// # Errors
///
/// Returns an error if:
/// - The index cannot be opened.
/// - The JSON string cannot be parsed into a [`FieldOption`].
/// - The engine rejects the field (e.g. duplicate name, unknown analyzer).
/// - The updated schema cannot be persisted to disk.
pub async fn run(name: &str, field_option_json: &str, data_dir: &Path) -> Result<()> {
    let engine = context::open_index(data_dir).await?;

    let field_option: FieldOption =
        serde_json::from_str(field_option_json).context("Failed to parse field option JSON")?;

    let updated_schema = engine
        .add_field(name, field_option)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    context::save_schema(data_dir, &updated_schema)?;

    println!("Field '{name}' added successfully.");
    Ok(())
}
