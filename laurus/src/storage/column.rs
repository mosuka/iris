//! Column storage for fast field access.
//!
//! This module provides columnar storage capabilities for efficient
//! faceting, sorting, and aggregation operations. Data is organized by
//! field rather than by document, allowing rapid per-field lookups,
//! range queries, and frequency counting without full-document
//! deserialization.

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, RwLock};

use anyhow;
use byteorder::{BigEndian, ByteOrder};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::storage::Storage;

/// Column value types supported by the column storage.
///
/// Each variant wraps a single typed value that can be stored in a [`Column`].
/// The enum supports serialization to/from bytes for persistence, partial
/// ordering for range queries, and hashing for frequency counting.
/// Cross-type numeric comparisons (e.g. `I32` vs `I64`) are also allowed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnValue {
    /// UTF-8 string value.
    String(String),
    /// Signed 32-bit integer.
    I32(i32),
    /// Signed 64-bit integer.
    I64(i64),
    /// Unsigned 32-bit integer.
    U32(u32),
    /// Unsigned 64-bit integer.
    U64(u64),
    /// 32-bit IEEE 754 floating-point number.
    F32(f32),
    /// 64-bit IEEE 754 floating-point number.
    F64(f64),
    /// Boolean value.
    Bool(bool),
    /// Date-time represented as a Unix timestamp (seconds since epoch).
    DateTime(i64),
    /// Null / absent value.
    Null,
}

impl Eq for ColumnValue {}

impl std::hash::Hash for ColumnValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ColumnValue::String(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            ColumnValue::I32(v) => {
                1u8.hash(state);
                v.hash(state);
            }
            ColumnValue::I64(v) => {
                2u8.hash(state);
                v.hash(state);
            }
            ColumnValue::U32(v) => {
                3u8.hash(state);
                v.hash(state);
            }
            ColumnValue::U64(v) => {
                4u8.hash(state);
                v.hash(state);
            }
            ColumnValue::F32(v) => {
                5u8.hash(state);
                v.to_bits().hash(state);
            }
            ColumnValue::F64(v) => {
                6u8.hash(state);
                v.to_bits().hash(state);
            }
            ColumnValue::Bool(v) => {
                7u8.hash(state);
                v.hash(state);
            }
            ColumnValue::DateTime(v) => {
                8u8.hash(state);
                v.hash(state);
            }
            ColumnValue::Null => {
                255u8.hash(state);
            }
        }
    }
}

impl ColumnValue {
    /// Get the type name for this column value as a human-readable string.
    ///
    /// # Returns
    ///
    /// A static string slice identifying the variant (e.g. `"string"`,
    /// `"i32"`, `"null"`).
    pub fn type_name(&self) -> &'static str {
        match self {
            ColumnValue::String(_) => "string",
            ColumnValue::I32(_) => "i32",
            ColumnValue::I64(_) => "i64",
            ColumnValue::U32(_) => "u32",
            ColumnValue::U64(_) => "u64",
            ColumnValue::F32(_) => "f32",
            ColumnValue::F64(_) => "f64",
            ColumnValue::Bool(_) => "bool",
            ColumnValue::DateTime(_) => "datetime",
            ColumnValue::Null => "null",
        }
    }

    /// Check if this value can be compared with another value.
    ///
    /// Same-type pairs are always comparable. Cross-type numeric comparisons
    /// (e.g. `I32` vs `I64`, `F32` vs `F64`) are also allowed. `Null` is
    /// comparable with any other variant.
    ///
    /// # Arguments
    ///
    /// * `other` - The other value to check compatibility with.
    ///
    /// # Returns
    ///
    /// `true` if the two values can be meaningfully compared.
    pub fn is_comparable_with(&self, other: &ColumnValue) -> bool {
        match (self, other) {
            (ColumnValue::Null, _) | (_, ColumnValue::Null) => true,
            (ColumnValue::String(_), ColumnValue::String(_)) => true,
            (ColumnValue::I32(_), ColumnValue::I32(_)) => true,
            (ColumnValue::I64(_), ColumnValue::I64(_)) => true,
            (ColumnValue::U32(_), ColumnValue::U32(_)) => true,
            (ColumnValue::U64(_), ColumnValue::U64(_)) => true,
            (ColumnValue::F32(_), ColumnValue::F32(_)) => true,
            (ColumnValue::F64(_), ColumnValue::F64(_)) => true,
            (ColumnValue::Bool(_), ColumnValue::Bool(_)) => true,
            (ColumnValue::DateTime(_), ColumnValue::DateTime(_)) => true,
            // Allow numeric cross-comparisons
            (ColumnValue::I32(_), ColumnValue::I64(_))
            | (ColumnValue::I64(_), ColumnValue::I32(_))
            | (ColumnValue::U32(_), ColumnValue::U64(_))
            | (ColumnValue::U64(_), ColumnValue::U32(_))
            | (ColumnValue::F32(_), ColumnValue::F64(_))
            | (ColumnValue::F64(_), ColumnValue::F32(_)) => true,
            _ => false,
        }
    }

    /// Serialize this value to its binary byte representation.
    ///
    /// The format is a single type-marker byte followed by the value payload
    /// encoded in big-endian byte order. Strings are length-prefixed with a
    /// 4-byte big-endian `u32`.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the serialized bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails (should not happen in practice).
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();

        match self {
            ColumnValue::String(s) => {
                bytes.push(0); // Type marker
                let str_bytes = s.as_bytes();
                bytes.extend_from_slice(&(str_bytes.len() as u32).to_be_bytes());
                bytes.extend_from_slice(str_bytes);
            }
            ColumnValue::I32(v) => {
                bytes.push(1);
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            ColumnValue::I64(v) => {
                bytes.push(2);
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            ColumnValue::U32(v) => {
                bytes.push(3);
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            ColumnValue::U64(v) => {
                bytes.push(4);
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            ColumnValue::F32(v) => {
                bytes.push(5);
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            ColumnValue::F64(v) => {
                bytes.push(6);
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            ColumnValue::Bool(v) => {
                bytes.push(7);
                bytes.push(if *v { 1 } else { 0 });
            }
            ColumnValue::DateTime(v) => {
                bytes.push(8);
                bytes.extend_from_slice(&v.to_be_bytes());
            }
            ColumnValue::Null => {
                bytes.push(255); // Null marker
            }
        }

        Ok(bytes)
    }

    /// Deserialize a `ColumnValue` from its binary byte representation.
    ///
    /// The first byte is interpreted as a type marker that determines which
    /// variant to decode. An empty slice is treated as [`ColumnValue::Null`].
    /// The payload is expected in big-endian byte order, matching the format
    /// produced by [`to_bytes`](Self::to_bytes).
    ///
    /// # Arguments
    ///
    /// * `bytes` - The raw byte slice to deserialize from.
    ///
    /// # Returns
    ///
    /// The deserialized `ColumnValue`.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte slice is truncated, contains invalid
    /// UTF-8 for a string variant, or has an unrecognized type marker.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Ok(ColumnValue::Null);
        }

        let type_marker = bytes[0];
        match type_marker {
            0 => {
                // String
                if bytes.len() < 5 {
                    return Err(anyhow::anyhow!("Invalid string value bytes").into());
                }
                let len = BigEndian::read_u32(&bytes[1..5]) as usize;
                if bytes.len() < 5 + len {
                    return Err(anyhow::anyhow!("Truncated string value").into());
                }
                let s = String::from_utf8(bytes[5..5 + len].to_vec())
                    .map_err(|e| anyhow::anyhow!("UTF8 conversion error: {e}"))?;
                Ok(ColumnValue::String(s))
            }
            1 => {
                if bytes.len() < 5 {
                    return Err(anyhow::anyhow!("Invalid i32 value bytes").into());
                }
                let v = BigEndian::read_i32(&bytes[1..5]);
                Ok(ColumnValue::I32(v))
            }
            2 => {
                if bytes.len() < 9 {
                    return Err(anyhow::anyhow!("Invalid i64 value bytes").into());
                }
                let v = BigEndian::read_i64(&bytes[1..9]);
                Ok(ColumnValue::I64(v))
            }
            3 => {
                if bytes.len() < 5 {
                    return Err(anyhow::anyhow!("Invalid u32 value bytes").into());
                }
                let v = BigEndian::read_u32(&bytes[1..5]);
                Ok(ColumnValue::U32(v))
            }
            4 => {
                if bytes.len() < 9 {
                    return Err(anyhow::anyhow!("Invalid u64 value bytes").into());
                }
                let v = BigEndian::read_u64(&bytes[1..9]);
                Ok(ColumnValue::U64(v))
            }
            5 => {
                if bytes.len() < 5 {
                    return Err(anyhow::anyhow!("Invalid f32 value bytes").into());
                }
                let v = BigEndian::read_f32(&bytes[1..5]);
                Ok(ColumnValue::F32(v))
            }
            6 => {
                if bytes.len() < 9 {
                    return Err(anyhow::anyhow!("Invalid f64 value bytes").into());
                }
                let v = BigEndian::read_f64(&bytes[1..9]);
                Ok(ColumnValue::F64(v))
            }
            7 => {
                if bytes.len() < 2 {
                    return Err(anyhow::anyhow!("Invalid bool value bytes").into());
                }
                let v = bytes[1] != 0;
                Ok(ColumnValue::Bool(v))
            }
            8 => {
                if bytes.len() < 9 {
                    return Err(anyhow::anyhow!("Invalid datetime value bytes").into());
                }
                let v = BigEndian::read_i64(&bytes[1..9]);
                Ok(ColumnValue::DateTime(v))
            }
            255 => Ok(ColumnValue::Null),
            _ => Err(anyhow::anyhow!("Unknown column value type: {type_marker}").into()),
        }
    }
}

impl PartialOrd for ColumnValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;

        match (self, other) {
            (ColumnValue::Null, ColumnValue::Null) => Some(Ordering::Equal),
            (ColumnValue::Null, _) => Some(Ordering::Less),
            (_, ColumnValue::Null) => Some(Ordering::Greater),
            (ColumnValue::String(a), ColumnValue::String(b)) => a.partial_cmp(b),
            (ColumnValue::I32(a), ColumnValue::I32(b)) => a.partial_cmp(b),
            (ColumnValue::I64(a), ColumnValue::I64(b)) => a.partial_cmp(b),
            (ColumnValue::U32(a), ColumnValue::U32(b)) => a.partial_cmp(b),
            (ColumnValue::U64(a), ColumnValue::U64(b)) => a.partial_cmp(b),
            (ColumnValue::F32(a), ColumnValue::F32(b)) => a.partial_cmp(b),
            (ColumnValue::F64(a), ColumnValue::F64(b)) => a.partial_cmp(b),
            (ColumnValue::Bool(a), ColumnValue::Bool(b)) => a.partial_cmp(b),
            (ColumnValue::DateTime(a), ColumnValue::DateTime(b)) => a.partial_cmp(b),
            // Cross-type numeric comparisons
            (ColumnValue::I32(a), ColumnValue::I64(b)) => (*a as i64).partial_cmp(b),
            (ColumnValue::I64(a), ColumnValue::I32(b)) => a.partial_cmp(&(*b as i64)),
            (ColumnValue::U32(a), ColumnValue::U64(b)) => (*a as u64).partial_cmp(b),
            (ColumnValue::U64(a), ColumnValue::U32(b)) => a.partial_cmp(&(*b as u64)),
            (ColumnValue::F32(a), ColumnValue::F64(b)) => (*a as f64).partial_cmp(b),
            (ColumnValue::F64(a), ColumnValue::F32(b)) => a.partial_cmp(&(*b as f64)),
            _ => None,
        }
    }
}

/// Per-document column values for a single field.
///
/// A `Column` stores [`ColumnValue`] entries keyed by document ID for one
/// field, enabling efficient per-field filtering, range queries, faceting,
/// and aggregation without needing to deserialize full documents.
///
/// All public methods are safe to call from multiple threads because the
/// internal data structures are guarded by [`RwLock`].
#[derive(Debug)]
pub struct Column {
    /// The name of the field this column represents.
    field_name: String,
    /// Per-document values indexed by document ID.
    values: RwLock<HashMap<u32, ColumnValue>>,
    /// The next document ID to assign (one past the highest seen ID).
    next_doc_id: RwLock<u32>,
}

impl Column {
    /// Create a new, empty column for the given field.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The name of the field this column represents.
    ///
    /// # Returns
    ///
    /// A new `Column` with no stored values.
    pub fn new(field_name: String) -> Self {
        Column {
            field_name,
            values: RwLock::new(HashMap::new()),
            next_doc_id: RwLock::new(0),
        }
    }

    /// Get the field name for this column.
    ///
    /// # Returns
    ///
    /// A string slice referencing the field name.
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Add or replace a value for a document.
    ///
    /// If a value already exists for the given `doc_id`, it is overwritten.
    /// The internal `next_doc_id` counter is advanced past `doc_id` if needed.
    ///
    /// # Arguments
    ///
    /// * `doc_id` - The document identifier.
    /// * `value` - The column value to store.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    pub fn add_value(&self, doc_id: u32, value: ColumnValue) -> Result<()> {
        let mut values = self.values.write().unwrap();
        values.insert(doc_id, value);

        let mut next_id = self.next_doc_id.write().unwrap();
        if doc_id >= *next_id {
            *next_id = doc_id + 1;
        }

        Ok(())
    }

    /// Get the stored value for a document.
    ///
    /// # Arguments
    ///
    /// * `doc_id` - The document identifier.
    ///
    /// # Returns
    ///
    /// `Some(value)` if a value exists for the document, or `None` otherwise.
    pub fn get_value(&self, doc_id: u32) -> Option<ColumnValue> {
        let values = self.values.read().unwrap();
        values.get(&doc_id).cloned()
    }

    /// Get all stored values sorted by document ID in ascending order.
    ///
    /// # Returns
    ///
    /// A vector of `(doc_id, value)` pairs ordered by document ID.
    pub fn get_all_values(&self) -> Vec<(u32, ColumnValue)> {
        let values = self.values.read().unwrap();
        let mut result: Vec<_> = values
            .iter()
            .map(|(&id, value)| (id, value.clone()))
            .collect();
        result.sort_by_key(|(id, _)| *id);
        result
    }

    /// Get values for a contiguous range of document IDs (inclusive).
    ///
    /// Documents within the range that have no stored value are omitted from
    /// the result.
    ///
    /// # Arguments
    ///
    /// * `start_doc` - The first document ID in the range (inclusive).
    /// * `end_doc` - The last document ID in the range (inclusive).
    ///
    /// # Returns
    ///
    /// A vector of `(doc_id, value)` pairs for documents that have values
    /// within the specified range.
    pub fn get_values_in_range(&self, start_doc: u32, end_doc: u32) -> Vec<(u32, ColumnValue)> {
        let values = self.values.read().unwrap();
        let mut result = Vec::new();

        for doc_id in start_doc..=end_doc {
            if let Some(value) = values.get(&doc_id) {
                result.push((doc_id, value.clone()));
            }
        }

        result
    }

    /// Get the number of documents that have values stored in this column.
    ///
    /// # Returns
    ///
    /// The count of document entries in this column.
    pub fn doc_count(&self) -> u32 {
        let values = self.values.read().unwrap();
        values.len() as u32
    }

    /// Compute the frequency of each unique value in this column.
    ///
    /// This is useful for faceted search, aggregation, and cardinality
    /// estimation.
    ///
    /// # Returns
    ///
    /// A map from each distinct [`ColumnValue`] to the number of documents
    /// that contain it.
    pub fn get_value_frequencies(&self) -> HashMap<ColumnValue, u32> {
        let values = self.values.read().unwrap();
        let mut frequencies = HashMap::new();

        for value in values.values() {
            *frequencies.entry(value.clone()).or_insert(0) += 1;
        }

        frequencies
    }

    /// Find all document IDs whose stored value equals the given target.
    ///
    /// The returned IDs are sorted in ascending order.
    ///
    /// # Arguments
    ///
    /// * `target_value` - The value to search for.
    ///
    /// # Returns
    ///
    /// A sorted vector of document IDs that have the specified value.
    pub fn find_documents_with_value(&self, target_value: &ColumnValue) -> Vec<u32> {
        let values = self.values.read().unwrap();
        let mut result = Vec::new();

        for (&doc_id, value) in values.iter() {
            if value == target_value {
                result.push(doc_id);
            }
        }

        result.sort();
        result
    }

    /// Find all document IDs whose stored value falls within an inclusive range.
    ///
    /// Both bounds are inclusive (`min_value <= value <= max_value`).
    /// The returned IDs are sorted in ascending order.
    ///
    /// # Arguments
    ///
    /// * `min_value` - The lower bound of the range (inclusive).
    /// * `max_value` - The upper bound of the range (inclusive).
    ///
    /// # Returns
    ///
    /// A sorted vector of document IDs whose values lie within the range.
    pub fn find_documents_in_range(
        &self,
        min_value: &ColumnValue,
        max_value: &ColumnValue,
    ) -> Vec<u32> {
        let values = self.values.read().unwrap();
        let mut result = Vec::new();

        for (&doc_id, value) in values.iter() {
            if value >= min_value && value <= max_value {
                result.push(doc_id);
            }
        }

        result.sort();
        result
    }
}

/// Multi-column storage manager for fast per-field access.
///
/// `ColumnStorage` manages multiple [`Column`] instances, one per field name,
/// providing a unified API for adding, retrieving, and persisting columnar
/// data. It supports efficient faceting, sorting, and aggregation operations
/// by delegating to the appropriate per-field column.
///
/// Columns are created lazily on first access and are safe for concurrent
/// use from multiple threads.
#[derive(Debug)]
pub struct ColumnStorage {
    /// The underlying storage backend used for persistence.
    storage: Arc<dyn Storage>,
    /// Map from field name to the corresponding [`Column`].
    columns: RwLock<HashMap<String, Arc<Column>>>,
}

impl ColumnStorage {
    /// Create a new column storage backed by the given storage backend.
    ///
    /// # Arguments
    ///
    /// * `storage` - The underlying [`Storage`] used for persisting column data.
    ///
    /// # Returns
    ///
    /// A new `ColumnStorage` with no columns loaded.
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        ColumnStorage {
            storage,
            columns: RwLock::new(HashMap::new()),
        }
    }

    /// Get the column for a field, creating it if it does not already exist.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The name of the field whose column to retrieve.
    ///
    /// # Returns
    ///
    /// A shared reference-counted handle to the [`Column`].
    pub fn get_column(&self, field_name: &str) -> Arc<Column> {
        let mut columns = self.columns.write().unwrap();

        if let Some(column) = columns.get(field_name) {
            return Arc::clone(column);
        }

        let column = Arc::new(Column::new(field_name.to_string()));
        columns.insert(field_name.to_string(), Arc::clone(&column));
        column
    }

    /// Add a value to the column for a given field and document.
    ///
    /// The column is created automatically if it does not yet exist.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The field name identifying the column.
    /// * `doc_id` - The document identifier.
    /// * `value` - The value to store.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    pub fn add_value(&self, field_name: &str, doc_id: u32, value: ColumnValue) -> Result<()> {
        let column = self.get_column(field_name);
        column.add_value(doc_id, value)
    }

    /// Get a value from the column for a given field and document.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The field name identifying the column.
    /// * `doc_id` - The document identifier.
    ///
    /// # Returns
    ///
    /// `Some(value)` if the column exists and has a value for the document,
    /// or `None` otherwise.
    pub fn get_value(&self, field_name: &str, doc_id: u32) -> Option<ColumnValue> {
        let columns = self.columns.read().unwrap();
        if let Some(column) = columns.get(field_name) {
            column.get_value(doc_id)
        } else {
            None
        }
    }

    /// Get the names of all fields that have columns in this storage.
    ///
    /// # Returns
    ///
    /// A vector of field name strings (order is unspecified).
    pub fn get_field_names(&self) -> Vec<String> {
        let columns = self.columns.read().unwrap();
        columns.keys().cloned().collect()
    }

    /// Compute statistics for a column.
    ///
    /// # Arguments
    ///
    /// * `field_name` - The name of the field to compute statistics for.
    ///
    /// # Returns
    ///
    /// `Some(stats)` containing document count, unique value count, and
    /// value frequencies, or `None` if the column does not exist.
    pub fn get_column_stats(&self, field_name: &str) -> Option<ColumnStats> {
        let columns = self.columns.read().unwrap();
        if let Some(column) = columns.get(field_name) {
            let doc_count = column.doc_count();
            let value_frequencies = column.get_value_frequencies();
            let unique_values = value_frequencies.len() as u32;

            Some(ColumnStats {
                field_name: field_name.to_string(),
                doc_count,
                unique_values,
                value_frequencies,
            })
        } else {
            None
        }
    }

    /// Persist all columns to the underlying storage backend.
    ///
    /// Each column is serialized as JSON and written to
    /// `columns/<field_name>.json`.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or the underlying I/O fails.
    pub fn flush(&self) -> Result<()> {
        let columns = self.columns.read().unwrap();

        for (field_name, column) in columns.iter() {
            let values = column.get_all_values();
            let serialized = serde_json::to_vec(&values)?;

            let column_file = format!("columns/{field_name}.json");
            let mut output = self.storage.create_output(&column_file)?;
            output.write_all(&serialized)?;
            output.flush()?;
        }

        Ok(())
    }

    /// Load columns from the underlying storage backend.
    ///
    /// This is a placeholder for future implementation that will
    /// deserialize column data from the storage backend.
    /// Currently this method is a no-op and always returns `Ok(())`.
    pub fn load(&self) -> Result<()> {
        // Implementation would load column data from storage
        // This is a simplified version
        Ok(())
    }
}

/// Aggregated statistics for a single column.
///
/// `ColumnStats` holds summary information about a [`Column`], including the
/// total document count, the number of distinct values, and a frequency map
/// showing how often each value appears. This is useful for faceting and
/// query planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStats {
    /// The name of the field this statistics snapshot belongs to.
    pub field_name: String,
    /// The total number of documents that have a value in this column.
    pub doc_count: u32,
    /// The number of distinct values stored in this column.
    pub unique_values: u32,
    /// A map from each distinct value to the number of documents containing it.
    pub value_frequencies: HashMap<ColumnValue, u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory::MemoryStorage;

    #[test]
    fn test_column_value_serialization() {
        let values = vec![
            ColumnValue::String("hello".to_string()),
            ColumnValue::I32(42),
            ColumnValue::I64(-1000),
            ColumnValue::U32(100),
            ColumnValue::U64(99999),
            ColumnValue::F32(std::f32::consts::PI),
            ColumnValue::F64(std::f64::consts::E),
            ColumnValue::Bool(true),
            ColumnValue::Bool(false),
            ColumnValue::DateTime(1609459200), // 2021-01-01 00:00:00 UTC
            ColumnValue::Null,
        ];

        for value in values {
            let bytes = value.to_bytes().unwrap();
            let deserialized = ColumnValue::from_bytes(&bytes).unwrap();
            assert_eq!(value, deserialized);
        }
    }

    #[test]
    fn test_column_value_comparison() {
        assert!(ColumnValue::I32(5) < ColumnValue::I32(10));
        assert!(
            ColumnValue::String("apple".to_string()) < ColumnValue::String("banana".to_string())
        );
        assert!(ColumnValue::Bool(false) < ColumnValue::Bool(true));
        assert!(ColumnValue::Null < ColumnValue::I32(0));

        // Cross-type numeric comparison
        assert!(ColumnValue::I32(5) < ColumnValue::I64(10));
        assert!(ColumnValue::U32(5) < ColumnValue::U64(10));
        assert!(ColumnValue::F32(std::f32::consts::PI) < ColumnValue::F64(3.15));
    }

    #[test]
    fn test_column_operations() {
        let column = Column::new("test_field".to_string());

        // Add some values
        column
            .add_value(1, ColumnValue::String("apple".to_string()))
            .unwrap();
        column
            .add_value(2, ColumnValue::String("banana".to_string()))
            .unwrap();
        column
            .add_value(3, ColumnValue::String("apple".to_string()))
            .unwrap();
        column.add_value(4, ColumnValue::Null).unwrap();

        assert_eq!(column.doc_count(), 4);
        assert_eq!(
            column.get_value(1),
            Some(ColumnValue::String("apple".to_string()))
        );
        assert_eq!(column.get_value(5), None);

        let frequencies = column.get_value_frequencies();
        assert_eq!(
            frequencies.get(&ColumnValue::String("apple".to_string())),
            Some(&2)
        );
        assert_eq!(
            frequencies.get(&ColumnValue::String("banana".to_string())),
            Some(&1)
        );
        assert_eq!(frequencies.get(&ColumnValue::Null), Some(&1));

        let apple_docs =
            column.find_documents_with_value(&ColumnValue::String("apple".to_string()));
        assert_eq!(apple_docs, vec![1, 3]);
    }

    #[test]
    fn test_column_storage() {
        let storage = Arc::new(MemoryStorage::new(
            crate::storage::memory::MemoryStorageConfig::default(),
        ));
        let column_storage = ColumnStorage::new(storage);

        // Add values to different fields
        column_storage
            .add_value("title", 1, ColumnValue::String("Document 1".to_string()))
            .unwrap();
        column_storage
            .add_value("title", 2, ColumnValue::String("Document 2".to_string()))
            .unwrap();
        column_storage
            .add_value("score", 1, ColumnValue::F32(0.85))
            .unwrap();
        column_storage
            .add_value("score", 2, ColumnValue::F32(0.92))
            .unwrap();

        assert_eq!(
            column_storage.get_value("title", 1),
            Some(ColumnValue::String("Document 1".to_string()))
        );
        assert_eq!(
            column_storage.get_value("score", 2),
            Some(ColumnValue::F32(0.92))
        );

        let field_names = column_storage.get_field_names();
        assert!(field_names.contains(&"title".to_string()));
        assert!(field_names.contains(&"score".to_string()));

        let title_stats = column_storage.get_column_stats("title").unwrap();
        assert_eq!(title_stats.doc_count, 2);
        assert_eq!(title_stats.unique_values, 2);
    }

    #[test]
    fn test_column_range_queries() {
        let column = Column::new("score".to_string());

        column.add_value(1, ColumnValue::F32(0.1)).unwrap();
        column.add_value(2, ColumnValue::F32(0.5)).unwrap();
        column.add_value(3, ColumnValue::F32(0.8)).unwrap();
        column.add_value(4, ColumnValue::F32(0.9)).unwrap();
        column.add_value(5, ColumnValue::F32(1.0)).unwrap();

        let docs_in_range =
            column.find_documents_in_range(&ColumnValue::F32(0.4), &ColumnValue::F32(0.85));
        assert_eq!(docs_in_range, vec![2, 3]);

        let values_in_range = column.get_values_in_range(2, 4);
        assert_eq!(values_in_range.len(), 3);
        assert_eq!(values_in_range[0].0, 2);
        assert_eq!(values_in_range[1].0, 3);
        assert_eq!(values_in_range[2].0, 4);
    }
}
