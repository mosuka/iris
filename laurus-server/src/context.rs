use std::path::Path;

use anyhow::{Context, bail};
use laurus::storage::file::FileStorageConfig;
use laurus::{Engine, Schema, StorageConfig, StorageFactory};

const SCHEMA_FILE: &str = "schema.toml";
const STORE_DIR: &str = "store";

/// Create a new index at the given data directory with the provided schema.
pub async fn create_index(data_dir: &Path, schema: &Schema) -> anyhow::Result<Engine> {
    let schema_path = data_dir.join(SCHEMA_FILE);
    if schema_path.exists() {
        bail!(
            "Index already exists at {}. Delete the directory first.",
            data_dir.display()
        );
    }

    // Ensure the data directory exists.
    std::fs::create_dir_all(data_dir)
        .with_context(|| format!("Failed to create data directory: {}", data_dir.display()))?;

    // Serialize the schema to TOML and persist it.
    let schema_toml =
        toml::to_string_pretty(schema).context("Failed to serialize schema to TOML")?;
    std::fs::write(&schema_path, &schema_toml).context("Failed to write schema file")?;

    // Initialize storage and create the engine.
    let store_path = data_dir.join(STORE_DIR);
    let storage_config = StorageConfig::File(FileStorageConfig::new(&store_path));
    let storage = StorageFactory::create(storage_config)?;
    let engine = Engine::new(storage, schema.clone()).await?;

    Ok(engine)
}

/// Open an existing index from the given data directory.
pub async fn open_index(data_dir: &Path) -> anyhow::Result<Engine> {
    let schema_path = data_dir.join(SCHEMA_FILE);
    if !schema_path.exists() {
        bail!(
            "No index found at {}. Create one first via the CreateIndex RPC.",
            data_dir.display()
        );
    }

    let schema_toml =
        std::fs::read_to_string(&schema_path).context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_toml).context("Failed to parse schema TOML")?;

    let store_path = data_dir.join(STORE_DIR);
    let storage_config = StorageConfig::File(FileStorageConfig::new(&store_path));
    let storage = StorageFactory::open(storage_config)?;
    let engine = Engine::new(storage, schema).await?;

    Ok(engine)
}

/// Read the schema from the data directory without opening the full engine.
pub fn read_schema(data_dir: &Path) -> anyhow::Result<Schema> {
    let schema_path = data_dir.join(SCHEMA_FILE);
    let schema_toml =
        std::fs::read_to_string(&schema_path).context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_toml).context("Failed to parse schema TOML")?;
    Ok(schema)
}
