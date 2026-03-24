//! Segmented storage for documents (Unified).
//!
//! This module provides a way to store and retrieve documents in segments,
//! avoiding the need to keep all documents in memory or a single massive JSON file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::data::Document;
use crate::error::{LaurusError, Result};
use crate::storage::Storage;
use crate::storage::structured::{StructReader, StructWriter};

/// A segment of stored documents.
///
/// Each segment represents a contiguous batch of documents that have been flushed
/// to persistent storage as a single binary file. The segment tracks the range of
/// document IDs it contains, enabling efficient lookup without scanning every file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSegment {
    /// Unique identifier for this segment, used to derive the segment file name.
    pub id: u32,
    /// Lowest (inclusive) document ID stored in this segment.
    pub start_doc_id: u64,
    /// Highest (inclusive) document ID stored in this segment.
    pub end_doc_id: u64,
    /// Number of documents stored in this segment.
    pub doc_count: usize,
}

impl DocumentSegment {
    /// Returns the file name for this segment's binary data file.
    ///
    /// The file name is derived from the segment [`id`](Self::id) with zero-padded
    /// formatting (e.g. `doc_segment_000042.docs`).
    ///
    /// # Returns
    ///
    /// A `String` containing the segment file name.
    pub fn file_name(&self) -> String {
        format!("doc_segment_{:06}.docs", self.id)
    }

    /// Checks whether the given document ID falls within this segment's range.
    ///
    /// # Arguments
    ///
    /// * `doc_id` - The document ID to check.
    ///
    /// # Returns
    ///
    /// `true` if `doc_id` is between [`start_doc_id`](Self::start_doc_id) and
    /// [`end_doc_id`](Self::end_doc_id) (inclusive), `false` otherwise.
    pub fn contains(&self, doc_id: u64) -> bool {
        doc_id >= self.start_doc_id && doc_id <= self.end_doc_id
    }
}

/// Writer for document segments.
#[derive(Debug)]
pub struct DocumentSegmentWriter {
    storage: Arc<dyn Storage>,
}

impl DocumentSegmentWriter {
    /// Creates a new `DocumentSegmentWriter` backed by the given storage.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend used to persist segment files.
    ///
    /// # Returns
    ///
    /// A new `DocumentSegmentWriter` instance.
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Writes a set of documents to a new segment file and returns the resulting
    /// [`DocumentSegment`] metadata.
    ///
    /// Documents are serialized to JSON and written in ascending document-ID order
    /// using a simple binary format: `[u32: doc_count] ([u64: doc_id][bytes: json_data])*`.
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The unique ID to assign to the new segment.
    /// * `docs` - A map of document IDs to [`Document`] values to be written.
    ///
    /// # Returns
    ///
    /// A [`DocumentSegment`] describing the segment that was written.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] if `docs` is empty, serialization fails, or the
    /// underlying storage I/O fails.
    pub fn write_segment(
        &self,
        segment_id: u32,
        docs: &HashMap<u64, Document>,
    ) -> Result<DocumentSegment> {
        if docs.is_empty() {
            return Err(LaurusError::invalid_argument(
                "cannot write empty document segment",
            ));
        }

        let mut sorted_ids: Vec<_> = docs.keys().cloned().collect();
        sorted_ids.sort();

        let start_doc_id = *sorted_ids.first().unwrap();
        let end_doc_id = *sorted_ids.last().unwrap();
        let doc_count = docs.len();

        let segment = DocumentSegment {
            id: segment_id,
            start_doc_id,
            end_doc_id,
            doc_count,
        };

        let file_name = segment.file_name();
        let output = self.storage.create_output(&file_name)?;
        let mut writer = StructWriter::new(output);

        // Simple binary format using StructWriter:
        // [u32: doc_count]
        // [u64: doc_id][bytes: json_data] * doc_count

        let doc_count_u32: u32 = doc_count.try_into().map_err(|_| {
            LaurusError::InvalidOperation(format!("document count {doc_count} exceeds u32::MAX"))
        })?;
        writer.write_u32(doc_count_u32)?;
        for id in sorted_ids {
            let doc = docs.get(&id).unwrap();
            let json = serde_json::to_vec(doc)
                .map_err(|e| LaurusError::index(format!("failed to serialize document: {e}")))?;
            writer.write_u64(id)?;
            writer.write_bytes(&json)?;
        }

        writer.close()?;
        Ok(segment)
    }
}

/// Reader for document segments.
#[derive(Debug)]
pub struct DocumentSegmentReader {
    storage: Arc<dyn Storage>,
    segment: DocumentSegment,
}

impl DocumentSegmentReader {
    /// Creates a new `DocumentSegmentReader` for the specified segment.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend from which segment files are read.
    /// * `segment` - The [`DocumentSegment`] metadata describing the segment to read.
    ///
    /// # Returns
    ///
    /// A new `DocumentSegmentReader` instance.
    pub fn new(storage: Arc<dyn Storage>, segment: DocumentSegment) -> Self {
        Self { storage, segment }
    }

    /// Retrieves a single document by its internal document ID.
    ///
    /// If the `doc_id` is outside this segment's range the method returns `Ok(None)`
    /// without performing any I/O.
    ///
    /// # Arguments
    ///
    /// * `doc_id` - The internal document ID to look up.
    ///
    /// # Returns
    ///
    /// `Ok(Some(document))` if found, `Ok(None)` if the document is not in this segment.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on storage I/O or deserialization failure.
    pub fn get_document(&self, doc_id: u64) -> Result<Option<Document>> {
        if !self.segment.contains(doc_id) {
            return Ok(None);
        }

        let input = self.storage.open_input(&self.segment.file_name())?;
        let mut reader = StructReader::new(input)?;
        let doc_count = reader.read_u32()?;

        for _ in 0..doc_count {
            let current_id = reader.read_u64()?;
            let json = reader.read_bytes()?;
            if current_id == doc_id {
                let doc: Document = serde_json::from_slice(&json).map_err(|e| {
                    LaurusError::index(format!("failed to deserialize document: {e}"))
                })?;
                return Ok(Some(doc));
            }
        }

        Ok(None)
    }

    /// Retrieve multiple documents from this segment in a single pass.
    ///
    /// Opens the segment file once and scans through, collecting all
    /// documents whose IDs are in the requested set. More efficient
    /// than multiple individual [`get_document()`](Self::get_document) calls.
    ///
    /// # Arguments
    ///
    /// * `doc_ids` - Set of document IDs to retrieve.
    ///
    /// # Returns
    ///
    /// A map of doc_id to [`Document`] for all found documents in this segment.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on storage I/O or deserialization failure.
    pub fn get_documents_batch(
        &self,
        doc_ids: &std::collections::HashSet<u64>,
    ) -> Result<HashMap<u64, Document>> {
        let mut results = HashMap::with_capacity(doc_ids.len());
        if doc_ids.is_empty() {
            return Ok(results);
        }

        // Quick check: are any requested IDs within this segment's range?
        if !doc_ids.iter().any(|id| self.segment.contains(*id)) {
            return Ok(results);
        }

        let input = self.storage.open_input(&self.segment.file_name())?;
        let mut reader = StructReader::new(input)?;
        let doc_count = reader.read_u32()?;

        let mut remaining = doc_ids.len();
        for _ in 0..doc_count {
            if remaining == 0 {
                break; // All requested docs found, stop early.
            }
            let current_id = reader.read_u64()?;
            let json = reader.read_bytes()?;
            if doc_ids.contains(&current_id) {
                let doc: Document = serde_json::from_slice(&json).map_err(|e| {
                    LaurusError::index(format!("failed to deserialize document: {e}"))
                })?;
                results.insert(current_id, doc);
                remaining -= 1;
            }
        }
        Ok(results)
    }

    /// Finds the first internal document ID whose `_id` field matches the given external ID.
    ///
    /// The method performs a linear scan over all documents in this segment.
    ///
    /// # Arguments
    ///
    /// * `external_id` - The external document identifier to search for (value of the `_id` field).
    ///
    /// # Returns
    ///
    /// `Ok(Some(doc_id))` if a matching document is found, `Ok(None)` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on storage I/O or deserialization failure.
    pub fn find_by_external_id(&self, external_id: &str) -> Result<Option<u64>> {
        let input = self.storage.open_input(&self.segment.file_name())?;
        let mut reader = StructReader::new(input)?;
        let doc_count = reader.read_u32()?;

        for _ in 0..doc_count {
            let current_id = reader.read_u64()?;
            let json = reader.read_bytes()?;
            let doc: Document = serde_json::from_slice(&json)
                .map_err(|e| LaurusError::index(format!("failed to deserialize document: {e}")))?;
            if doc.fields.get("_id").and_then(|v| v.as_text()) == Some(external_id) {
                return Ok(Some(current_id));
            }
        }

        Ok(None)
    }

    /// Finds all internal document IDs whose `_id` field matches the given external ID.
    ///
    /// Unlike [`find_by_external_id`](Self::find_by_external_id) this method does not
    /// stop at the first match and returns every matching document ID in the segment.
    ///
    /// # Arguments
    ///
    /// * `external_id` - The external document identifier to search for (value of the `_id` field).
    ///
    /// # Returns
    ///
    /// A `Vec<u64>` of all matching internal document IDs (may be empty).
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on storage I/O or deserialization failure.
    pub fn find_all_by_external_id(&self, external_id: &str) -> Result<Vec<u64>> {
        let input = self.storage.open_input(&self.segment.file_name())?;
        let mut reader = StructReader::new(input)?;
        let doc_count = reader.read_u32()?;
        let mut results = Vec::new();

        for _ in 0..doc_count {
            let current_id = reader.read_u64()?;
            let json = reader.read_bytes()?;
            let doc: Document = serde_json::from_slice(&json)
                .map_err(|e| LaurusError::index(format!("failed to deserialize document: {e}")))?;
            if doc.fields.get("_id").and_then(|v| v.as_text()) == Some(external_id) {
                results.push(current_id);
            }
        }

        Ok(results)
    }
}

const MANIFEST_FILE: &str = "segments.json";

#[derive(Debug, Serialize, Deserialize)]
struct StoreManifest {
    version: u32,
    segments: Vec<DocumentSegment>,
    next_segment_id: u32,
}

/// Unified segmented document store.
///
/// `UnifiedDocumentStore` manages document persistence across multiple binary segment
/// files. Newly added documents are held in an in-memory pending buffer until
/// [`commit`](Self::commit) is called, at which point they are flushed to a new segment
/// file and the manifest is atomically updated.
///
/// A JSON manifest (`segments.json`) tracks all committed segments and the next
/// segment ID so that the store can be re-opened across process restarts.
#[derive(Debug)]
pub struct UnifiedDocumentStore {
    storage: Arc<dyn Storage>,
    segments: Vec<DocumentSegment>,
    next_segment_id: u32,
    pending_docs: HashMap<u64, Document>,
    next_doc_id: u64,
}

impl UnifiedDocumentStore {
    /// Creates a new, empty `UnifiedDocumentStore`.
    ///
    /// No manifest file is read or written; the store starts with zero segments and
    /// document IDs beginning at 1.
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend for segment and manifest files.
    ///
    /// # Returns
    ///
    /// A fresh `UnifiedDocumentStore` instance.
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            segments: Vec::new(),
            next_segment_id: 0,
            pending_docs: HashMap::new(),
            next_doc_id: 1,
        }
    }

    /// Opens an existing document store from the given storage backend.
    ///
    /// If a manifest file (`segments.json`) exists it is read and the segment list and
    /// ID counters are restored. Otherwise a fresh, empty store is returned (equivalent
    /// to calling [`new`](Self::new)).
    ///
    /// # Arguments
    ///
    /// * `storage` - The storage backend containing the manifest and segment files.
    ///
    /// # Returns
    ///
    /// A `UnifiedDocumentStore` populated from the persisted manifest.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] if the manifest file exists but cannot be read or
    /// deserialized.
    pub fn open(storage: Arc<dyn Storage>) -> Result<Self> {
        if storage.file_exists(MANIFEST_FILE) {
            let input = storage.open_input(MANIFEST_FILE)?;
            let mut reader = StructReader::new(input)?;
            let json = reader.read_bytes()?;
            let manifest: StoreManifest = serde_json::from_slice(&json)
                .map_err(|e| LaurusError::index(format!("failed to deserialize manifest: {e}")))?;

            let mut next_doc_id = 1;
            for segment in &manifest.segments {
                if segment.end_doc_id >= next_doc_id {
                    next_doc_id = segment.end_doc_id + 1;
                }
            }

            Ok(Self {
                storage,
                segments: manifest.segments,
                next_segment_id: manifest.next_segment_id,
                pending_docs: HashMap::new(),
                next_doc_id,
            })
        } else {
            Ok(Self::new(storage))
        }
    }

    /// Flushes pending documents to a new segment and atomically updates the manifest.
    ///
    /// If there are no pending documents the manifest is still written so that any
    /// previously added segments are persisted. After the manifest is written the
    /// storage is synced to ensure durability.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on serialization or storage I/O failure.
    pub fn commit(&mut self) -> Result<()> {
        // Flush pending documents if any
        if !self.pending_docs.is_empty() {
            let docs = std::mem::take(&mut self.pending_docs);
            self.add_segment(&docs)?;
        }

        let manifest = StoreManifest {
            version: 1,
            segments: self.segments.clone(),
            next_segment_id: self.next_segment_id,
        };

        let json = serde_json::to_vec(&manifest)
            .map_err(|e| LaurusError::index(format!("failed to serialize manifest: {e}")))?;

        // Atomic write
        let tmp_file = format!("{}.tmp", MANIFEST_FILE);
        let output = self.storage.create_output(&tmp_file)?;
        let mut writer = StructWriter::new(output);
        writer.write_bytes(&json)?;
        writer.close()?;

        self.storage.rename_file(&tmp_file, MANIFEST_FILE)?;

        // Sync storage to ensure directory metadata (new segment files, renamed
        // manifest) is visible to subsequent reads. Critical on Windows where
        // directory listings may be cached.
        self.storage.sync()?;

        Ok(())
    }

    /// Adds a document to the pending buffer and assigns it a new internal document ID.
    ///
    /// The document is **not** written to storage until [`commit`](Self::commit) is called.
    ///
    /// # Arguments
    ///
    /// * `doc` - The [`Document`] to add.
    ///
    /// # Returns
    ///
    /// The newly assigned internal document ID.
    ///
    /// # Errors
    ///
    /// Currently infallible, but returns `Result` for forward compatibility.
    pub fn add_document(&mut self, doc: Document) -> Result<u64> {
        let doc_id = self.next_doc_id;
        self.next_doc_id += 1;
        self.pending_docs.insert(doc_id, doc);
        // Flushing is intentionally left to the caller via `commit()` to give full
        // control over transaction boundaries and batch sizes.
        Ok(doc_id)
    }

    /// Get the current next_doc_id counter.
    ///
    /// Used by [`DocumentLog`](super::log::DocumentLog) to sync its own
    /// counter with committed document store segments on startup.
    pub fn next_doc_id(&self) -> u64 {
        self.next_doc_id
    }

    /// Insert a document with a specific doc_id (used during WAL recovery).
    ///
    /// Updates `next_doc_id` if the given `doc_id` is >= current counter
    /// to avoid ID conflicts on subsequent `add_document()` calls.
    pub fn put_document_with_id(&mut self, doc_id: u64, doc: Document) {
        self.pending_docs.insert(doc_id, doc);
        if doc_id >= self.next_doc_id {
            self.next_doc_id = doc_id + 1;
        }
    }

    /// Writes a set of documents into a new segment file and registers it in the store.
    ///
    /// This is a lower-level method; most callers should use [`add_document`](Self::add_document)
    /// followed by [`commit`](Self::commit) instead.
    ///
    /// # Arguments
    ///
    /// * `docs` - A map of internal document IDs to [`Document`] values.
    ///
    /// # Returns
    ///
    /// The [`DocumentSegment`] metadata for the newly created segment.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] if `docs` is empty or the segment write fails.
    pub fn add_segment(&mut self, docs: &HashMap<u64, Document>) -> Result<DocumentSegment> {
        let writer = DocumentSegmentWriter::new(self.storage.clone());
        let segment = writer.write_segment(self.next_segment_id, docs)?;
        self.segments.push(segment.clone());
        self.next_segment_id += 1;
        Ok(segment)
    }

    /// Retrieves a document by its internal document ID.
    ///
    /// Pending (uncommitted) documents are checked first, followed by committed
    /// segments in reverse order (newest first).
    ///
    /// # Arguments
    ///
    /// * `doc_id` - The internal document ID.
    ///
    /// # Returns
    ///
    /// `Ok(Some(document))` if found, `Ok(None)` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on storage I/O or deserialization failure.
    pub fn get_document(&self, doc_id: u64) -> Result<Option<Document>> {
        // Check pending docs first
        if let Some(doc) = self.pending_docs.get(&doc_id) {
            return Ok(Some(doc.clone()));
        }

        // Search in reverse order (newer segments first might have the doc if it was updated?)
        // Actually doc_ids are unique, so any segment is fine.
        for segment in self.segments.iter().rev() {
            if segment.contains(doc_id) {
                let reader = DocumentSegmentReader::new(self.storage.clone(), segment.clone());
                if let Some(doc) = reader.get_document(doc_id)? {
                    return Ok(Some(doc));
                }
            }
        }
        Ok(None)
    }

    /// Retrieve multiple documents by their internal IDs in a single batch.
    ///
    /// More efficient than individual [`get_document()`](Self::get_document) calls because
    /// each segment file is opened and scanned only once.
    ///
    /// # Arguments
    ///
    /// * `doc_ids` - Slice of internal document IDs to retrieve.
    ///
    /// # Returns
    ///
    /// A map of doc_id to [`Document`] for all found documents.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on storage I/O or deserialization failure.
    pub fn get_documents_batch(&self, doc_ids: &[u64]) -> Result<HashMap<u64, Document>> {
        let mut results = HashMap::with_capacity(doc_ids.len());
        if doc_ids.is_empty() {
            return Ok(results);
        }

        let id_set: std::collections::HashSet<u64> = doc_ids.iter().copied().collect();

        // Check pending docs first.
        for &doc_id in doc_ids {
            if let Some(doc) = self.pending_docs.get(&doc_id) {
                results.insert(doc_id, doc.clone());
            }
        }

        // Find remaining IDs not yet resolved.
        let remaining: std::collections::HashSet<u64> = id_set
            .iter()
            .filter(|id| !results.contains_key(id))
            .copied()
            .collect();

        if remaining.is_empty() {
            return Ok(results);
        }

        // Batch-load from segments (one file open per segment).
        for segment in &self.segments {
            let segment_ids: std::collections::HashSet<u64> = remaining
                .iter()
                .filter(|id| segment.contains(**id))
                .copied()
                .collect();
            if segment_ids.is_empty() {
                continue;
            }

            let reader = DocumentSegmentReader::new(self.storage.clone(), segment.clone());
            let batch = reader.get_documents_batch(&segment_ids)?;
            results.extend(batch);
        }

        Ok(results)
    }

    /// Finds the first internal document ID whose `_id` field matches the given external ID.
    ///
    /// Pending documents are searched first, then committed segments in reverse order.
    ///
    /// # Arguments
    ///
    /// * `external_id` - The external document identifier to search for.
    ///
    /// # Returns
    ///
    /// `Ok(Some(doc_id))` if a matching document is found, `Ok(None)` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on storage I/O or deserialization failure.
    pub fn find_by_external_id(&self, external_id: &str) -> Result<Option<u64>> {
        // Check pending docs first
        for (id, doc) in &self.pending_docs {
            if doc.fields.get("_id").and_then(|v| v.as_text()) == Some(external_id) {
                return Ok(Some(*id));
            }
        }

        for segment in self.segments.iter().rev() {
            let reader = DocumentSegmentReader::new(self.storage.clone(), segment.clone());
            if let Some(id) = reader.find_by_external_id(external_id)? {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }

    /// Finds all internal document IDs whose `_id` field matches the given external ID.
    ///
    /// Both pending documents and all committed segments are searched.
    ///
    /// # Arguments
    ///
    /// * `external_id` - The external document identifier to search for.
    ///
    /// # Returns
    ///
    /// A `Vec<u64>` of all matching internal document IDs (may be empty).
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] on storage I/O or deserialization failure.
    pub fn find_all_by_external_id(&self, external_id: &str) -> Result<Vec<u64>> {
        let mut results = Vec::new();

        // Check pending docs
        for (id, doc) in &self.pending_docs {
            if doc.fields.get("_id").and_then(|v| v.as_text()) == Some(external_id) {
                results.push(*id);
            }
        }

        for segment in self.segments.iter() {
            let reader = DocumentSegmentReader::new(self.storage.clone(), segment.clone());
            results.extend(reader.find_all_by_external_id(external_id)?);
        }
        Ok(results)
    }

    /// Marks a document as deleted.
    ///
    /// Logical deletion is handled externally by the deletion bitmap / deletion manager;
    /// this method is a no-op placeholder that exists for API symmetry.
    ///
    /// # Arguments
    ///
    /// * `_doc_id` - The internal document ID to delete (currently unused).
    ///
    /// # Errors
    ///
    /// Currently infallible.
    pub fn delete_document(&mut self, _doc_id: u64) -> Result<()> {
        // Logical deletion is handled by DeletionBitmap/DeletionManager.
        Ok(())
    }

    /// Returns a slice of all committed [`DocumentSegment`]s.
    ///
    /// # Returns
    ///
    /// A borrowed slice of segment metadata, ordered by creation time.
    pub fn segments(&self) -> &[DocumentSegment] {
        &self.segments
    }

    /// Deletes the underlying data file for the segment with the given ID.
    ///
    /// If no segment with `segment_id` exists in the store the call is a no-op.
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The ID of the segment whose file should be removed.
    ///
    /// # Errors
    ///
    /// Returns [`LaurusError`] if the storage backend fails to delete the file.
    pub fn delete_segment_files(&self, segment_id: u32) -> Result<()> {
        if let Some(segment) = self.segments.iter().find(|s| s.id == segment_id) {
            self.storage.delete_file(&segment.file_name())?;
        }
        Ok(())
    }
}
