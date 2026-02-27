use std::path::Path;

use anyhow::{Context, Result, bail};
use laurus::storage::file::FileStorageConfig;
use laurus::{Engine, Schema, StorageConfig, StorageFactory};

const SCHEMA_FILE: &str = "schema.toml";
const STORE_DIR: &str = "store";

/// Create a new index in the given data directory from a schema TOML file.
pub async fn create_index(data_dir: &Path, schema_path: &Path) -> Result<()> {
    if data_dir.join(SCHEMA_FILE).exists() {
        bail!(
            "Index already exists at {}. Delete the directory first to recreate.",
            data_dir.display()
        );
    }

    // Read and parse the schema file.
    let schema_content =
        std::fs::read_to_string(schema_path).context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_content).context("Failed to parse schema TOML")?;

    // Create the data directory.
    std::fs::create_dir_all(data_dir).context("Failed to create data directory")?;

    // Save schema to the data directory as TOML.
    let schema_toml =
        toml::to_string_pretty(&schema).context("Failed to serialize schema to TOML")?;
    let schema_dest = data_dir.join(SCHEMA_FILE);
    std::fs::write(&schema_dest, &schema_toml).context("Failed to write schema file")?;

    // Create the storage and engine to initialize the index structure.
    let store_path = data_dir.join(STORE_DIR);
    let storage_config = StorageConfig::File(FileStorageConfig::new(&store_path));
    let storage = StorageFactory::create(storage_config)?;
    let _engine = Engine::new(storage, schema).await?;

    Ok(())
}

/// Open an existing index from the given data directory.
pub async fn open_index(data_dir: &Path) -> Result<Engine> {
    let schema_path = data_dir.join(SCHEMA_FILE);
    if !schema_path.exists() {
        bail!(
            "No index found at {}. Run 'index create' first.",
            data_dir.display()
        );
    }

    // Read the schema.
    let schema_toml =
        std::fs::read_to_string(&schema_path).context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_toml).context("Failed to parse schema TOML")?;

    // Open storage and recover the engine.
    let store_path = data_dir.join(STORE_DIR);
    let storage_config = StorageConfig::File(FileStorageConfig::new(&store_path));
    let storage = StorageFactory::open(storage_config)?;
    let engine = Engine::new(storage, schema).await?;

    Ok(engine)
}
