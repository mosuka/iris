//! Conversions between protobuf types and JSON values.
//!
//! These helpers translate between the laurus-server proto `Document` / `Value`
//! types and `serde_json::Value` for the MCP tool input/output.

use laurus_server::proto::laurus::v1;
use serde_json::{Value, json};

/// Convert a proto [`v1::Document`] to a [`serde_json::Value`] (JSON object).
///
/// # Arguments
///
/// * `doc` - The proto document to convert.
pub fn document_to_json(doc: &v1::Document) -> Value {
    let fields: serde_json::Map<String, Value> = doc
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), proto_value_to_json(v)))
        .collect();
    Value::Object(fields)
}

/// Convert a [`serde_json::Value`] (JSON object) to a proto [`v1::Document`].
///
/// JSON types are mapped to proto `Value` kinds as follows:
///
/// | JSON type | Proto Value kind |
/// |-----------|-----------------|
/// | null | `null_value` |
/// | boolean | `bool_value` |
/// | integer number | `int64_value` |
/// | float number | `float64_value` |
/// | string | `text_value` |
/// | array of numbers | `vector_value` (f32 elements) |
/// | other | `null_value` |
///
/// # Arguments
///
/// * `value` - A JSON object whose keys become document field names.
///
/// # Errors
///
/// Returns an error if `value` is not a JSON object.
pub fn json_to_document(value: Value) -> anyhow::Result<v1::Document> {
    let obj = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("document must be a JSON object"))?;

    let fields = obj
        .iter()
        .map(|(k, v)| (k.clone(), json_to_proto_value(v)))
        .collect();

    Ok(v1::Document { fields })
}

fn proto_value_to_json(val: &v1::Value) -> Value {
    use v1::value::Kind;
    match &val.kind {
        None | Some(Kind::NullValue(_)) => Value::Null,
        Some(Kind::BoolValue(b)) => Value::Bool(*b),
        Some(Kind::Int64Value(i)) => json!(i),
        Some(Kind::Float64Value(f)) => json!(f),
        Some(Kind::TextValue(s)) => Value::String(s.clone()),
        Some(Kind::BytesValue(b)) => {
            Value::String(b.iter().map(|byte| format!("{byte:02x}")).collect())
        }
        Some(Kind::VectorValue(v)) => json!(v.values),
        Some(Kind::DatetimeValue(us)) => {
            // Convert Unix microseconds to ISO 8601.
            let secs = us / 1_000_000;
            let nanos = ((us % 1_000_000) * 1_000) as u32;
            if let Some(dt) = chrono::DateTime::from_timestamp(secs, nanos) {
                Value::String(dt.to_rfc3339())
            } else {
                json!(us)
            }
        }
        Some(Kind::GeoValue(g)) => json!({ "lat": g.latitude, "lon": g.longitude }),
    }
}

fn json_to_proto_value(val: &Value) -> v1::Value {
    use v1::value::Kind;
    let kind = match val {
        Value::Null => Some(Kind::NullValue(true)),
        Value::Bool(b) => Some(Kind::BoolValue(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(Kind::Int64Value(i))
            } else if let Some(f) = n.as_f64() {
                Some(Kind::Float64Value(f))
            } else {
                Some(Kind::NullValue(true))
            }
        }
        Value::String(s) => Some(Kind::TextValue(s.clone())),
        Value::Array(arr) => {
            let floats: Option<Vec<f32>> =
                arr.iter().map(|v| v.as_f64().map(|f| f as f32)).collect();
            floats.map(|values| Kind::VectorValue(v1::VectorValue { values }))
        }
        Value::Object(_) => Some(Kind::NullValue(true)),
    };
    v1::Value { kind }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn text_value(s: &str) -> v1::Value {
        v1::Value {
            kind: Some(v1::value::Kind::TextValue(s.to_string())),
        }
    }

    fn int_value(i: i64) -> v1::Value {
        v1::Value {
            kind: Some(v1::value::Kind::Int64Value(i)),
        }
    }

    fn float_value(f: f64) -> v1::Value {
        v1::Value {
            kind: Some(v1::value::Kind::Float64Value(f)),
        }
    }

    #[test]
    fn test_document_to_json() {
        let mut fields = HashMap::new();
        fields.insert("title".to_string(), text_value("hello"));
        fields.insert("score".to_string(), float_value(1.5));
        fields.insert("count".to_string(), int_value(42));
        let doc = v1::Document { fields };

        let json = document_to_json(&doc);
        assert_eq!(json["title"], "hello");
        assert_eq!(json["score"], 1.5);
        assert_eq!(json["count"], 42);
    }

    #[test]
    fn test_json_to_document() {
        let json_val = json!({
            "text_field": "hello",
            "int_field": 10,
            "float_field": 3.14,
            "bool_field": true,
            "null_field": null,
            "vec_field": [0.1_f32, 0.2_f32, 0.3_f32]
        });

        let doc = json_to_document(json_val).unwrap();
        assert!(matches!(
            doc.fields["text_field"].kind,
            Some(v1::value::Kind::TextValue(_))
        ));
        assert!(matches!(
            doc.fields["int_field"].kind,
            Some(v1::value::Kind::Int64Value(10))
        ));
        assert!(matches!(
            doc.fields["bool_field"].kind,
            Some(v1::value::Kind::BoolValue(true))
        ));
        assert!(matches!(
            doc.fields["null_field"].kind,
            Some(v1::value::Kind::NullValue(_))
        ));
        assert!(matches!(
            doc.fields["vec_field"].kind,
            Some(v1::value::Kind::VectorValue(_))
        ));
    }
}
