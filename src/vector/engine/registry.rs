//! VectorEngine ドキュメントレジストリ関連の型定義
//!
//! このモジュールはドキュメントレジストリ、ドキュメントエントリ、フィールドエントリを提供する。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{Result, SarissaError};
use crate::vector::core::document::DocumentVector;
use crate::vector::engine::filter::{RegistryFilterMatches, VectorFilter};

pub type RegistryVersion = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEntry {
    pub field_name: String,
    pub version: RegistryVersion,
    pub vector_count: usize,
    pub weight: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentEntry {
    pub doc_id: u64,
    pub version: RegistryVersion,
    pub metadata: HashMap<String, String>,
    pub fields: HashMap<String, FieldEntry>,
}

#[derive(Debug, Default)]
pub struct DocumentVectorRegistry {
    entries: RwLock<HashMap<u64, DocumentEntry>>,
    external_id_to_doc_id: RwLock<HashMap<String, u64>>,
    next_version: AtomicU64,
}

impl DocumentVectorRegistry {
    pub fn upsert(
        &self,
        doc_id: u64,
        fields: &[FieldEntry],
        metadata: HashMap<String, String>,
    ) -> Result<RegistryVersion> {
        let version = self.next_version.fetch_add(1, Ordering::SeqCst) + 1;
        let mut map = HashMap::new();
        for entry in fields {
            let mut cloned = entry.clone();
            cloned.version = version;
            map.insert(cloned.field_name.clone(), cloned);
        }

        let doc_entry = DocumentEntry {
            doc_id,
            version,
            metadata: metadata.clone(),
            fields: map,
        };

        let mut entries_guard = self.entries.write();
        entries_guard.insert(doc_id, doc_entry);

        // Update external ID mapping if present
        if let Some(ext_id) = metadata.get("_id") {
            self.external_id_to_doc_id
                .write()
                .insert(ext_id.clone(), doc_id);
        }

        Ok(version)
    }

    pub fn delete(&self, doc_id: u64) -> Result<()> {
        let mut guard = self.entries.write();
        if let Some(entry) = guard.remove(&doc_id) {
            // Remove from external ID mapping if present
            if let Some(ext_id) = entry.metadata.get("_id") {
                self.external_id_to_doc_id.write().remove(ext_id);
            }
            Ok(())
        } else {
            Err(SarissaError::not_found(format!("doc_id {doc_id}")))
        }
    }

    pub fn contains(&self, doc_id: u64) -> bool {
        self.entries.read().contains_key(&doc_id)
    }

    pub fn filter_existing(&self, doc_ids: &[u64]) -> HashSet<u64> {
        let guard = self.entries.read();
        doc_ids
            .iter()
            .filter(|id| guard.contains_key(id))
            .copied()
            .collect()
    }

    pub fn get(&self, doc_id: u64) -> Option<DocumentEntry> {
        self.entries.read().get(&doc_id).cloned()
    }

    pub fn get_doc_id_by_external_id(&self, external_id: &str) -> Option<u64> {
        self.external_id_to_doc_id.read().get(external_id).copied()
    }

    pub fn snapshot(&self) -> Result<Vec<u8>> {
        let guard = self.entries.read();
        serde_json::to_vec(&*guard).map_err(SarissaError::from)
    }

    pub fn from_snapshot(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Ok(Self::default());
        }

        let entries: HashMap<u64, DocumentEntry> = serde_json::from_slice(bytes)?;
        let mut external_id_to_doc_id = HashMap::new();
        let mut max_version = 0;

        for (doc_id, entry) in &entries {
            if entry.version > max_version {
                max_version = entry.version;
            }
            if let Some(ext_id) = entry.metadata.get("_id") {
                external_id_to_doc_id.insert(ext_id.clone(), *doc_id);
            }
        }

        Ok(Self {
            entries: RwLock::new(entries),
            external_id_to_doc_id: RwLock::new(external_id_to_doc_id),
            next_version: AtomicU64::new(max_version),
        })
    }

    pub fn document_count(&self) -> usize {
        self.entries.read().len()
    }

    pub fn filter_matches(
        &self,
        filter: &VectorFilter,
        target_fields: &[String],
    ) -> RegistryFilterMatches {
        let guard = self.entries.read();
        let mut allowed_fields: HashMap<u64, HashSet<String>> = HashMap::new();

        for entry in guard.values() {
            if !filter.document.is_empty() && !filter.document.matches(&entry.metadata) {
                continue;
            }

            let mut matched_fields: HashSet<String> = HashSet::new();
            for field_name in target_fields {
                if let Some(field_entry) = entry.fields.get(field_name)
                    && (filter.field.is_empty() || filter.field.matches(&field_entry.metadata))
                {
                    matched_fields.insert(field_name.clone());
                }
            }

            if matched_fields.is_empty() {
                continue;
            }

            allowed_fields.insert(entry.doc_id, matched_fields);
        }

        RegistryFilterMatches { allowed_fields }
    }
}

pub fn build_field_entries(document: &DocumentVector) -> Vec<FieldEntry> {
    document
        .fields
        .iter()
        .map(|(name, vector)| FieldEntry {
            field_name: name.clone(),
            version: 0,
            vector_count: 1, // Always 1 in flattened model
            weight: vector.weight,
            metadata: vector.attributes.clone(),
        })
        .collect()
}
