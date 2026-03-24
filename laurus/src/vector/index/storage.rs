use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

use crate::error::{LaurusError, Result};
use crate::storage::Storage;
use crate::vector::core::vector::Vector;

/// Storage for vectors (in-memory or on-demand from disk).
///
/// # Thread Safety
///
/// - The `Owned` variant holds an immutable `Arc<HashMap>` that is freely
///   shareable across threads.
/// - The `OnDemand` variant stores a reference to the underlying
///   [`Storage`] and the file name so that each call to [`get`](Self::get)
///   opens an independent file handle.  This eliminates the previous
///   `Mutex`-based serialization and allows fully concurrent reads.
#[derive(Debug, Clone)]
pub enum VectorStorage {
    /// All vectors are loaded into memory.
    Owned(Arc<HashMap<(u64, String), Vector>>),
    /// Vectors are read from disk on demand.
    ///
    /// Each [`get`](Self::get) call opens a fresh [`StorageInput`](crate::storage::StorageInput)
    /// via [`Storage::open_input`], performs a single seek + read, and closes
    /// the handle.  For mmap-backed storage this is essentially free; for
    /// file-backed storage the OS typically caches the file descriptor.
    OnDemand {
        /// Reference to the storage backend (e.g. file system, mmap).
        storage: Arc<dyn Storage>,
        /// Name of the vector index file within the storage.
        file_name: String,
        /// Pre-built mapping from `(doc_id, field_name)` to byte offset.
        offsets: Arc<HashMap<(u64, String), u64>>,
    },
}

impl VectorStorage {
    /// Returns all keys stored in this vector storage.
    pub fn keys(&self) -> Vec<(u64, String)> {
        match self {
            VectorStorage::Owned(map) => map.keys().cloned().collect(),
            VectorStorage::OnDemand { offsets, .. } => offsets.keys().cloned().collect(),
        }
    }

    /// Returns the number of vectors stored.
    pub fn len(&self) -> usize {
        match self {
            VectorStorage::Owned(map) => map.len(),
            VectorStorage::OnDemand { offsets, .. } => offsets.len(),
        }
    }

    /// Returns `true` if no vectors are stored.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if a vector with the given key exists.
    ///
    /// # Arguments
    ///
    /// * `key` - A `(doc_id, field_name)` tuple identifying the vector.
    pub fn contains_key(&self, key: &(u64, String)) -> bool {
        match self {
            VectorStorage::Owned(map) => map.contains_key(key),
            VectorStorage::OnDemand { offsets, .. } => offsets.contains_key(key),
        }
    }

    /// Retrieves a vector by its key.
    ///
    /// For the `Owned` variant the vector is cloned (O(1) due to `Arc`
    /// wrapping).  For the `OnDemand` variant a fresh file handle is opened,
    /// the reader seeks to the recorded offset, and the vector data is read
    /// directly.
    ///
    /// # Arguments
    ///
    /// * `key` - A `(doc_id, field_name)` tuple identifying the vector.
    /// * `dimension` - The expected number of dimensions (used to size the read buffer).
    ///
    /// # Returns
    ///
    /// `Ok(Some(vector))` if the key exists, `Ok(None)` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on I/O failure.
    pub fn get(&self, key: &(u64, String), dimension: usize) -> Result<Option<Vector>> {
        match self {
            VectorStorage::Owned(map) => Ok(map.get(key).cloned()),
            VectorStorage::OnDemand {
                storage,
                file_name,
                offsets,
            } => {
                if let Some(&offset) = offsets.get(key) {
                    let mut input = storage.open_input(file_name).map_err(|e| {
                        LaurusError::internal(format!("Failed to open vector file: {e}"))
                    })?;

                    input
                        .seek(SeekFrom::Start(offset))
                        .map_err(LaurusError::Io)?;

                    // Skip doc_id (8 bytes) + field_name (4 bytes length + variable)
                    let mut doc_id_buf = [0u8; 8];
                    input.read_exact(&mut doc_id_buf)?;

                    let mut field_name_len_buf = [0u8; 4];
                    input.read_exact(&mut field_name_len_buf)?;
                    let field_name_len = u32::from_le_bytes(field_name_len_buf) as usize;
                    let mut field_name_buf = vec![0u8; field_name_len];
                    input.read_exact(&mut field_name_buf)?;

                    // Read vector data
                    let mut values = vec![0.0f32; dimension];
                    for value in &mut values {
                        let mut value_buf = [0u8; 4];
                        input.read_exact(&mut value_buf)?;
                        *value = f32::from_le_bytes(value_buf);
                    }
                    Ok(Some(Vector::new(values)))
                } else {
                    Ok(None)
                }
            }
        }
    }
}
