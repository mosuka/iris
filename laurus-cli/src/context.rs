//! Index lifecycle helpers for the CLI.
//!
//! Provides convenience functions for creating a new index from a schema TOML
//! file and for opening an existing index from a index directory. These are
//! used by the various CLI subcommands to obtain an [`Engine`] instance.

use std::path::Path;

use anyhow::{Context, Result, bail};
use laurus::storage::file::FileStorageConfig;
use laurus::{Engine, Schema, StorageConfig, StorageFactory};

/// File name used to persist the schema inside the index directory.
const SCHEMA_FILE: &str = "schema.toml";

/// Subdirectory name used for the storage backend within the index directory.
const STORE_DIR: &str = "store";

/// Create a new index in the given index directory from a schema TOML file.
///
/// Reads the schema from `schema_path`, creates the index directory (if it
/// does not already exist), persists the schema as `schema.toml` inside
/// `index_dir`, and initialises the underlying storage and engine.
///
/// # Arguments
///
/// * `index_dir` - Path to the index directory where the index will be stored.
/// * `schema_path` - Path to the source schema TOML file that defines fields
///   and their options.
///
/// # Returns
///
/// Returns `Ok(())` on success.
///
/// # Errors
///
/// Returns an error if:
/// - An index already exists in `index_dir` (i.e. `schema.toml` is present).
/// - The schema file cannot be read or parsed.
/// - The index directory cannot be created.
/// - The engine or storage initialisation fails.
pub async fn create_index(index_dir: &Path, schema_path: &Path) -> Result<()> {
    if index_dir.join(SCHEMA_FILE).exists() {
        bail!(
            "Index already exists at {}. Delete the directory first to recreate.",
            index_dir.display()
        );
    }

    // Read and parse the schema file.
    let schema_content =
        std::fs::read_to_string(schema_path).context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_content).context("Failed to parse schema TOML")?;

    // Create the index directory.
    std::fs::create_dir_all(index_dir).context("Failed to create index directory")?;

    // Save schema to the index directory as TOML.
    let schema_toml =
        toml::to_string_pretty(&schema).context("Failed to serialize schema to TOML")?;
    let schema_dest = index_dir.join(SCHEMA_FILE);
    std::fs::write(&schema_dest, &schema_toml).context("Failed to write schema file")?;

    // Create the storage and engine to initialize the index structure.
    let store_path = index_dir.join(STORE_DIR);
    let storage_config = StorageConfig::File(FileStorageConfig::new(&store_path));
    let storage = StorageFactory::create(storage_config)?;
    let _engine = Engine::new(storage, schema).await?;

    Ok(())
}

/// Open an existing index from the given index directory.
///
/// Reads the persisted `schema.toml`, opens the file-based storage backend,
/// and constructs an [`Engine`] instance ready for querying and mutation.
///
/// # Arguments
///
/// * `index_dir` - Path to the index directory that contains an existing index
///   (must have a `schema.toml` file and a `store/` subdirectory).
///
/// # Returns
///
/// Returns the opened [`Engine`] on success.
///
/// # Errors
///
/// Returns an error if:
/// - No `schema.toml` file is found in `index_dir`.
/// - The schema file cannot be read or parsed.
/// - The storage backend cannot be opened or the engine cannot be initialised.
pub async fn open_index(index_dir: &Path) -> Result<Engine> {
    let schema_path = index_dir.join(SCHEMA_FILE);
    if !schema_path.exists() {
        bail!(
            "No index found at {}. Run 'index create' first.",
            index_dir.display()
        );
    }

    // Read the schema.
    let schema_toml =
        std::fs::read_to_string(&schema_path).context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_toml).context("Failed to parse schema TOML")?;

    // Open storage and recover the engine.
    let store_path = index_dir.join(STORE_DIR);
    let storage_config = StorageConfig::File(FileStorageConfig::new(&store_path));
    let storage = StorageFactory::open(storage_config)?;
    let engine = Engine::new(storage, schema).await?;

    Ok(engine)
}

/// Persist the current schema back to the index directory.
///
/// Serializes the given schema as TOML and writes it to `schema.toml`
/// inside `index_dir`, overwriting the existing file.
///
/// # Arguments
///
/// * `index_dir` - Path to the index directory containing the index.
/// * `schema` - The schema to persist.
///
/// # Errors
///
/// Returns an error if serialization or file write fails.
/// Read the schema from the index directory.
///
/// # Arguments
///
/// * `index_dir` - The index directory containing `schema.toml`.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn read_schema(index_dir: &Path) -> Result<Schema> {
    let schema_path = index_dir.join(SCHEMA_FILE);
    let schema_toml =
        std::fs::read_to_string(&schema_path).context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_toml).context("Failed to parse schema TOML")?;
    Ok(schema)
}

/// Persist the schema to the index directory.
///
/// # Arguments
///
/// * `index_dir` - The index directory in which to write `schema.toml`.
/// * `schema` - The schema to persist.
///
/// # Errors
///
/// Returns an error if serialization or file write fails.
pub fn save_schema(index_dir: &Path, schema: &Schema) -> Result<()> {
    let schema_toml =
        toml::to_string_pretty(schema).context("Failed to serialize schema to TOML")?;
    let schema_dest = index_dir.join(SCHEMA_FILE);
    std::fs::write(&schema_dest, &schema_toml).context("Failed to write schema file")?;
    Ok(())
}
