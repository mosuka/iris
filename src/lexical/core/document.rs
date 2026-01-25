//! Document structure for schema-less indexing.
//!
//! This module adapts the unified [`crate::data::Document`] for Lexical indexing.

use crate::lexical::core::field::{
    BlobOption, BooleanOption, DateTimeOption, FieldValue, FloatOption, GeoOption, IntegerOption,
    TextOption,
};

// Re-export Unified Document
pub use crate::data::Document;

/// A builder for constructing documents in a fluent manner.
#[derive(Debug)]
pub struct DocumentBuilder {
    document: Document,
}

impl DocumentBuilder {
    pub fn new() -> Self {
        DocumentBuilder {
            document: Document::new(),
        }
    }

    /// Add a text field. Options are ignored (controlled by Schema/Config).
    pub fn add_text<S: Into<String>, T: Into<String>>(
        mut self,
        name: S,
        value: T,
        _option: TextOption,
    ) -> Self {
        self.document = self
            .document
            .with_field(name, crate::data::DataValue::Text(value.into()));
        self
    }

    /// Add an integer field. Options are ignored.
    pub fn add_integer<S: Into<String>>(
        mut self,
        name: S,
        value: i64,
        _option: IntegerOption,
    ) -> Self {
        self.document = self
            .document
            .with_field(name, crate::data::DataValue::Int64(value));
        self
    }

    /// Add a float field. Options are ignored.
    pub fn add_float<S: Into<String>>(mut self, name: S, value: f64, _option: FloatOption) -> Self {
        self.document = self
            .document
            .with_field(name, crate::data::DataValue::Float64(value));
        self
    }

    /// Add a boolean field. Options are ignored.
    pub fn add_boolean<S: Into<String>>(
        mut self,
        name: S,
        value: bool,
        _option: BooleanOption,
    ) -> Self {
        self.document = self
            .document
            .with_field(name, crate::data::DataValue::Bool(value));
        self
    }

    /// Add a datetime field. Options are ignored.
    pub fn add_datetime<S: Into<String>>(
        mut self,
        name: S,
        value: chrono::DateTime<chrono::Utc>,
        _option: DateTimeOption,
    ) -> Self {
        self.document = self
            .document
            .with_field(name, crate::data::DataValue::DateTime(value));
        self
    }

    /// Add a geo field. Options are ignored.
    pub fn add_geo<S: Into<String>>(
        mut self,
        name: S,
        lat: f64,
        lon: f64,
        _option: GeoOption,
    ) -> Self {
        self.document = self
            .document
            .with_field(name, crate::data::DataValue::Geo(lat, lon));
        self
    }

    /// Add a blob field. Options are ignored.
    pub fn add_blob<S: Into<String>, M: Into<String>>(
        mut self,
        name: S,
        mime_type: M,
        data: Vec<u8>,
        _option: BlobOption,
    ) -> Self {
        self.document = self.document.with_field(
            name,
            crate::data::DataValue::Bytes(data.into(), Some(mime_type.into())),
        );
        self
    }

    /// Add a generic field value.
    pub fn add_field<S: Into<String>>(mut self, name: S, value: FieldValue) -> Self {
        self.document = self.document.with_field(name, value);
        self
    }

    pub fn build(self) -> Document {
        self.document
    }
}

impl Default for DocumentBuilder {
    fn default() -> Self {
        Self::new()
    }
}
