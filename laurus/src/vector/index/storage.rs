use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

use crate::error::{LaurusError, Result};
use crate::storage::StorageInput;
use crate::vector::core::vector::Vector;

/// Storage for vectors (in-memory or on-demand from disk).
///
/// # Thread Safety Notes
///
/// In the `OnDemand` variant:
/// - `offsets` is immutable after construction (`Arc<HashMap>`), so concurrent
///   reads are safe without locking.
/// - `input` uses `Mutex` because `StorageInput` requires `&mut self` for
///   seek + read. This serializes concurrent reads, which is inherent to
///   single file-handle I/O. For higher throughput, consider opening separate
///   file handles per thread.
#[derive(Debug, Clone)]
pub enum VectorStorage {
    Owned(Arc<HashMap<(u64, String), Vector>>),
    OnDemand {
        input: Arc<Mutex<Box<dyn StorageInput>>>,
        offsets: Arc<HashMap<(u64, String), u64>>,
    },
}

impl VectorStorage {
    pub fn keys(&self) -> Vec<(u64, String)> {
        match self {
            VectorStorage::Owned(map) => map.keys().cloned().collect(),
            VectorStorage::OnDemand { offsets, .. } => offsets.keys().cloned().collect(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            VectorStorage::Owned(map) => map.len(),
            VectorStorage::OnDemand { offsets, .. } => offsets.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains_key(&self, key: &(u64, String)) -> bool {
        match self {
            VectorStorage::Owned(map) => map.contains_key(key),
            VectorStorage::OnDemand { offsets, .. } => offsets.contains_key(key),
        }
    }

    pub fn get(&self, key: &(u64, String), dimension: usize) -> Result<Option<Vector>> {
        match self {
            VectorStorage::Owned(map) => Ok(map.get(key).cloned()),
            VectorStorage::OnDemand { input, offsets } => {
                if let Some(&offset) = offsets.get(key) {
                    let mut input = input
                        .lock()
                        .map_err(|_| LaurusError::internal("Mutex poisoned".to_string()))?;

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
