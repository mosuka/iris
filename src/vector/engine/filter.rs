//! VectorEngine フィルタ関連の型定義
//!
//! このモジュールはメタデータフィルタ、エンジンフィルタ、フィルタマッチ結果を提供する。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataFilter {
    #[serde(default)]
    pub equals: HashMap<String, String>,
}

impl MetadataFilter {
    pub(crate) fn is_empty(&self) -> bool {
        self.equals.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorFilter {
    #[serde(default)]
    pub document: MetadataFilter,
    #[serde(default)]
    pub field: MetadataFilter,
}

impl VectorFilter {
    pub(crate) fn is_empty(&self) -> bool {
        self.document.is_empty() && self.field.is_empty()
    }
}
