//! VectorStore フィルタ関連の型定義
//!
//! このモジュールはメタデータフィルタ、エンジンフィルタ、フィルタマッチ結果を提供する。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::lexical::core::field::FieldValue;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataFilter {
    #[serde(default)]
    pub equals: HashMap<String, FieldValue>,
}

use crate::vector::store::request::LexicalQuery;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VectorFilter {
    Simple {
        #[serde(default)]
        document: MetadataFilter,
        #[serde(default)]
        field: MetadataFilter,
    },
    Advanced(LexicalQuery),
}

impl Default for VectorFilter {
    fn default() -> Self {
        VectorFilter::Simple {
            document: MetadataFilter::default(),
            field: MetadataFilter::default(),
        }
    }
}
