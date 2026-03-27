//! OPFS persistence layer for laurus-wasm.
//!
//! This module provides functions to load and save index data between
//! a [`MemoryStorage`] instance and the browser's Origin Private File System (OPFS).
//!
//! The design uses `MemoryStorage` as the runtime backend (which satisfies the
//! `Storage` trait's `Send + Sync` requirement) and OPFS as a persistence layer.
//! Data is loaded from OPFS into memory on `open`, and persisted back on `commit`.

use std::io::{Read, Write};
use std::sync::Arc;

use laurus::storage::Storage;
use laurus::storage::memory::{MemoryStorage, MemoryStorageConfig};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// JS FFI: OPFS bridge functions
// ---------------------------------------------------------------------------

#[wasm_bindgen(module = "/js/opfs_bridge.js")]
extern "C" {
    /// Initialize an OPFS directory for an index.
    ///
    /// Returns a `FileSystemDirectoryHandle` as an opaque `JsValue`.
    #[wasm_bindgen(catch)]
    async fn opfs_init(name: &str) -> Result<JsValue, JsValue>;

    /// List all file names in an OPFS directory.
    #[wasm_bindgen(catch)]
    async fn opfs_list_files(dir: &JsValue) -> Result<JsValue, JsValue>;

    /// Check if a file exists in the OPFS directory.
    #[wasm_bindgen(catch)]
    async fn opfs_file_exists(dir: &JsValue, name: &str) -> Result<JsValue, JsValue>;

    /// Read a file's contents as a `Uint8Array`.
    #[wasm_bindgen(catch)]
    async fn opfs_read_file(dir: &JsValue, name: &str) -> Result<JsValue, JsValue>;

    /// Write data to a file in the OPFS directory.
    #[wasm_bindgen(catch)]
    async fn opfs_write_file(dir: &JsValue, name: &str, data: &[u8]) -> Result<JsValue, JsValue>;

    /// Delete a file from the OPFS directory.
    #[wasm_bindgen(catch)]
    async fn opfs_delete_file(dir: &JsValue, name: &str) -> Result<JsValue, JsValue>;

    /// Delete an entire index directory.
    #[wasm_bindgen(catch)]
    async fn opfs_delete_index(name: &str) -> Result<JsValue, JsValue>;
}

// ---------------------------------------------------------------------------
// OpfsPersistence
// ---------------------------------------------------------------------------

/// OPFS persistence handle for a single index.
///
/// Holds a reference to an OPFS directory (`FileSystemDirectoryHandle`)
/// and provides methods to load/save data to/from a [`MemoryStorage`].
pub struct OpfsPersistence {
    /// The OPFS directory handle (opaque JS object).
    dir: JsValue,
    /// The index name (used for error messages and deletion).
    name: String,
}

impl OpfsPersistence {
    /// Open or create an OPFS directory for the given index name.
    ///
    /// # Arguments
    ///
    /// * `name` - Index name used as the OPFS subdirectory name.
    ///
    /// # Returns
    ///
    /// A new `OpfsPersistence` handle.
    pub async fn open(name: &str) -> Result<Self, JsValue> {
        let dir = opfs_init(name).await?;
        Ok(Self {
            dir,
            name: name.to_string(),
        })
    }

    /// Load all files from OPFS into a new [`MemoryStorage`].
    ///
    /// Creates a new `MemoryStorage`, reads every file from the OPFS directory,
    /// and writes it into the in-memory store.
    ///
    /// # Returns
    ///
    /// An `Arc<MemoryStorage>` populated with all OPFS files, ready to be
    /// passed to `Engine::new()`.
    pub async fn load(&self) -> Result<Arc<MemoryStorage>, JsValue> {
        let storage = MemoryStorage::new(MemoryStorageConfig::default());

        let file_names_js = opfs_list_files(&self.dir).await?;
        let file_names: Vec<String> = serde_wasm_bindgen::from_value(file_names_js)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse file list: {e}")))?;

        for file_name in &file_names {
            let data_js = opfs_read_file(&self.dir, file_name).await?;
            let data: Vec<u8> = js_sys::Uint8Array::new(&data_js).to_vec();

            let mut output = storage.create_output(file_name).map_err(|e| {
                JsValue::from_str(&format!("Failed to create output '{file_name}': {e}"))
            })?;
            output
                .write_all(&data)
                .map_err(|e| JsValue::from_str(&format!("Failed to write '{file_name}': {e}")))?;
            output
                .close()
                .map_err(|e| JsValue::from_str(&format!("Failed to close '{file_name}': {e}")))?;
        }

        Ok(Arc::new(storage))
    }

    /// Save all files from a [`MemoryStorage`] to OPFS.
    ///
    /// Lists all files in the `MemoryStorage`, reads each one, and writes
    /// it to the OPFS directory. Files in OPFS that no longer exist in
    /// memory are deleted.
    ///
    /// # Arguments
    ///
    /// * `storage` - The in-memory storage to persist.
    pub async fn save(&self, storage: &dyn Storage) -> Result<(), JsValue> {
        let memory_files: Vec<String> = storage
            .list_files()
            .map_err(|e| JsValue::from_str(&format!("Failed to list memory files: {e}")))?;

        // Delete OPFS files that are no longer in memory
        let opfs_files_js = opfs_list_files(&self.dir).await?;
        let opfs_files: Vec<String> = serde_wasm_bindgen::from_value(opfs_files_js)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse OPFS file list: {e}")))?;

        for opfs_file in &opfs_files {
            if !memory_files.contains(opfs_file) {
                opfs_delete_file(&self.dir, opfs_file).await?;
            }
        }

        // Write all memory files to OPFS
        for file_name in &memory_files {
            let mut input = storage
                .open_input(file_name)
                .map_err(|e| JsValue::from_str(&format!("Failed to open '{file_name}': {e}")))?;
            let mut data = Vec::new();
            input
                .read_to_end(&mut data)
                .map_err(|e| JsValue::from_str(&format!("Failed to read '{file_name}': {e}")))?;
            opfs_write_file(&self.dir, file_name, &data).await?;
        }

        Ok(())
    }

    /// Delete the entire index from OPFS.
    pub async fn delete(&self) -> Result<(), JsValue> {
        opfs_delete_index(&self.name).await?;
        Ok(())
    }
}
