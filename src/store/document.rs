//! Segmented storage for documents (Unified).
//!
//! This module provides a way to store and retrieve documents in segments,
//! avoiding the need to keep all documents in memory or a single massive JSON file.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::data::Document;
use crate::error::{IrisError, Result};
use crate::storage::Storage;
use crate::storage::structured::{StructReader, StructWriter};

/// A segment of stored documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSegment {
    pub id: u32,
    pub start_doc_id: u64,
    pub end_doc_id: u64,
    pub doc_count: usize,
}

impl DocumentSegment {
    pub fn file_name(&self) -> String {
        format!("doc_segment_{:06}.docs", self.id)
    }

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
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self { storage }
    }

    pub fn write_segment(
        &self,
        segment_id: u32,
        docs: &HashMap<u64, Document>,
    ) -> Result<DocumentSegment> {
        if docs.is_empty() {
            return Err(IrisError::invalid_argument(
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
            IrisError::InvalidOperation(format!(
                "document count {doc_count} exceeds u32::MAX"
            ))
        })?;
        writer.write_u32(doc_count_u32)?;
        for id in sorted_ids {
            let doc = docs.get(&id).unwrap();
            let json = serde_json::to_vec(doc)
                .map_err(|e| IrisError::index(format!("failed to serialize document: {e}")))?;
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
    pub fn new(storage: Arc<dyn Storage>, segment: DocumentSegment) -> Self {
        Self { storage, segment }
    }

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
                    IrisError::index(format!("failed to deserialize document: {e}"))
                })?;
                return Ok(Some(doc));
            }
        }

        Ok(None)
    }

    pub fn find_by_external_id(&self, external_id: &str) -> Result<Option<u64>> {
        let input = self.storage.open_input(&self.segment.file_name())?;
        let mut reader = StructReader::new(input)?;
        let doc_count = reader.read_u32()?;

        for _ in 0..doc_count {
            let current_id = reader.read_u64()?;
            let json = reader.read_bytes()?;
            let doc: Document = serde_json::from_slice(&json)
                .map_err(|e| IrisError::index(format!("failed to deserialize document: {e}")))?;
            if doc.fields.get("_id").and_then(|v| v.as_text()) == Some(external_id) {
                return Ok(Some(current_id));
            }
        }

        Ok(None)
    }

    pub fn find_all_by_external_id(&self, external_id: &str) -> Result<Vec<u64>> {
        let input = self.storage.open_input(&self.segment.file_name())?;
        let mut reader = StructReader::new(input)?;
        let doc_count = reader.read_u32()?;
        let mut results = Vec::new();

        for _ in 0..doc_count {
            let current_id = reader.read_u64()?;
            let json = reader.read_bytes()?;
            let doc: Document = serde_json::from_slice(&json)
                .map_err(|e| IrisError::index(format!("failed to deserialize document: {e}")))?;
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

/// A segmented storage for documents (Unified).
#[derive(Debug)]
pub struct UnifiedDocumentStore {
    storage: Arc<dyn Storage>,
    segments: Vec<DocumentSegment>,
    next_segment_id: u32,
    pending_docs: HashMap<u64, Document>,
    next_doc_id: u64,
}

impl UnifiedDocumentStore {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            segments: Vec::new(),
            next_segment_id: 0,
            pending_docs: HashMap::new(),
            next_doc_id: 1,
        }
    }

    pub fn open(storage: Arc<dyn Storage>) -> Result<Self> {
        if storage.file_exists(MANIFEST_FILE) {
            let input = storage.open_input(MANIFEST_FILE)?;
            let mut reader = StructReader::new(input)?;
            let json = reader.read_bytes()?;
            let manifest: StoreManifest = serde_json::from_slice(&json)
                .map_err(|e| IrisError::index(format!("failed to deserialize manifest: {e}")))?;

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
            .map_err(|e| IrisError::index(format!("failed to serialize manifest: {e}")))?;

        // Atomic write
        let tmp_file = format!("{}.tmp", MANIFEST_FILE);
        let output = self.storage.create_output(&tmp_file)?;
        let mut writer = StructWriter::new(output);
        writer.write_bytes(&json)?;
        writer.close()?;

        self.storage.rename_file(&tmp_file, MANIFEST_FILE)?;
        Ok(())
    }

    pub fn add_document(&mut self, doc: Document) -> Result<u64> {
        let doc_id = self.next_doc_id;
        self.next_doc_id += 1;
        self.pending_docs.insert(doc_id, doc);
        // TODO: Auto-flush if pending_docs too large?
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

    pub fn add_segment(&mut self, docs: &HashMap<u64, Document>) -> Result<DocumentSegment> {
        let writer = DocumentSegmentWriter::new(self.storage.clone());
        let segment = writer.write_segment(self.next_segment_id, docs)?;
        self.segments.push(segment.clone());
        self.next_segment_id += 1;
        Ok(segment)
    }

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

    pub fn delete_document(&mut self, _doc_id: u64) -> Result<()> {
        // Logical deletion is handled by DeletionBitmap/DeletionManager.
        Ok(())
    }

    pub fn segments(&self) -> &[DocumentSegment] {
        &self.segments
    }

    pub fn delete_segment_files(&self, segment_id: u32) -> Result<()> {
        if let Some(segment) = self.segments.iter().find(|s| s.id == segment_id) {
            self.storage.delete_file(&segment.file_name())?;
        }
        Ok(())
    }
}
