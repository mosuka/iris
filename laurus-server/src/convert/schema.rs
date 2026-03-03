//! Conversion between [`laurus::Schema`] and the protobuf `Schema` message.
//!
//! Handles mapping of all field option variants (text, integer, float, boolean,
//! datetime, geo, bytes, HNSW, flat, IVF), distance metrics, and quantization
//! configuration.

use std::collections::HashMap;

use laurus::{
    BooleanOption, BytesOption, DateTimeOption, DistanceMetric, FieldOption, FlatOption,
    FloatOption, GeoOption, HnswOption, IntegerOption, IvfOption, QuantizationMethod, Schema,
    TextOption,
};

use crate::proto::laurus::v1;

/// Convert a laurus Schema into a proto Schema.
pub fn to_proto(schema: &Schema) -> v1::Schema {
    let fields: HashMap<String, v1::FieldOption> = schema
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), field_option_to_proto(v)))
        .collect();
    v1::Schema {
        fields,
        default_fields: schema.default_fields.clone(),
    }
}

/// Convert a proto Schema into a laurus Schema.
pub fn from_proto(proto: &v1::Schema) -> Result<Schema, String> {
    let mut fields = HashMap::new();
    for (name, fo) in &proto.fields {
        let option = field_option_from_proto(fo)
            .ok_or_else(|| format!("Field '{name}' has no option set"))?;
        fields.insert(name.clone(), option);
    }
    Ok(Schema {
        fields,
        default_fields: proto.default_fields.clone(),
    })
}

fn field_option_to_proto(fo: &FieldOption) -> v1::FieldOption {
    use v1::field_option::Option as Opt;
    let option = match fo {
        FieldOption::Text(o) => Some(Opt::Text(v1::TextOption {
            indexed: o.indexed,
            stored: o.stored,
            term_vectors: o.term_vectors,
        })),
        FieldOption::Integer(o) => Some(Opt::Integer(v1::IntegerOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        FieldOption::Float(o) => Some(Opt::Float(v1::FloatOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        FieldOption::Boolean(o) => Some(Opt::Boolean(v1::BooleanOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        FieldOption::DateTime(o) => Some(Opt::DateTime(v1::DateTimeOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        FieldOption::Geo(o) => Some(Opt::Geo(v1::GeoOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        FieldOption::Bytes(o) => Some(Opt::Bytes(v1::BytesOption { stored: o.stored })),
        FieldOption::Hnsw(o) => Some(Opt::Hnsw(v1::HnswOption {
            dimension: o.dimension as u32,
            distance: distance_to_proto(&o.distance) as i32,
            m: o.m as u32,
            ef_construction: o.ef_construction as u32,
            base_weight: o.base_weight,
            quantizer: o.quantizer.map(|q| quantization_to_proto(&q)),
        })),
        FieldOption::Flat(o) => Some(Opt::Flat(v1::FlatOption {
            dimension: o.dimension as u32,
            distance: distance_to_proto(&o.distance) as i32,
            base_weight: o.base_weight,
            quantizer: o.quantizer.map(|q| quantization_to_proto(&q)),
        })),
        FieldOption::Ivf(o) => Some(Opt::Ivf(v1::IvfOption {
            dimension: o.dimension as u32,
            distance: distance_to_proto(&o.distance) as i32,
            n_clusters: o.n_clusters as u32,
            n_probe: o.n_probe as u32,
            base_weight: o.base_weight,
            quantizer: o.quantizer.map(|q| quantization_to_proto(&q)),
        })),
    };
    v1::FieldOption { option }
}

fn field_option_from_proto(fo: &v1::FieldOption) -> Option<FieldOption> {
    use v1::field_option::Option as Opt;
    match &fo.option {
        Some(Opt::Text(o)) => Some(FieldOption::Text(TextOption {
            indexed: o.indexed,
            stored: o.stored,
            term_vectors: o.term_vectors,
        })),
        Some(Opt::Integer(o)) => Some(FieldOption::Integer(IntegerOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        Some(Opt::Float(o)) => Some(FieldOption::Float(FloatOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        Some(Opt::Boolean(o)) => Some(FieldOption::Boolean(BooleanOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        Some(Opt::DateTime(o)) => Some(FieldOption::DateTime(DateTimeOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        Some(Opt::Geo(o)) => Some(FieldOption::Geo(GeoOption {
            indexed: o.indexed,
            stored: o.stored,
        })),
        Some(Opt::Bytes(o)) => Some(FieldOption::Bytes(BytesOption { stored: o.stored })),
        Some(Opt::Hnsw(o)) => Some(FieldOption::Hnsw(HnswOption {
            dimension: o.dimension as usize,
            distance: distance_from_proto(o.distance),
            m: o.m as usize,
            ef_construction: o.ef_construction as usize,
            base_weight: o.base_weight,
            quantizer: o.quantizer.as_ref().map(quantization_from_proto),
        })),
        Some(Opt::Flat(o)) => Some(FieldOption::Flat(FlatOption {
            dimension: o.dimension as usize,
            distance: distance_from_proto(o.distance),
            base_weight: o.base_weight,
            quantizer: o.quantizer.as_ref().map(quantization_from_proto),
        })),
        Some(Opt::Ivf(o)) => Some(FieldOption::Ivf(IvfOption {
            dimension: o.dimension as usize,
            distance: distance_from_proto(o.distance),
            n_clusters: o.n_clusters as usize,
            n_probe: o.n_probe as usize,
            base_weight: o.base_weight,
            quantizer: o.quantizer.as_ref().map(quantization_from_proto),
        })),
        None => None,
    }
}

fn distance_to_proto(d: &DistanceMetric) -> v1::DistanceMetric {
    match d {
        DistanceMetric::Cosine => v1::DistanceMetric::Cosine,
        DistanceMetric::Euclidean => v1::DistanceMetric::Euclidean,
        DistanceMetric::Manhattan => v1::DistanceMetric::Manhattan,
        DistanceMetric::DotProduct => v1::DistanceMetric::DotProduct,
        DistanceMetric::Angular => v1::DistanceMetric::Angular,
    }
}

fn distance_from_proto(d: i32) -> DistanceMetric {
    match v1::DistanceMetric::try_from(d) {
        Ok(v1::DistanceMetric::Cosine) => DistanceMetric::Cosine,
        Ok(v1::DistanceMetric::Euclidean) => DistanceMetric::Euclidean,
        Ok(v1::DistanceMetric::Manhattan) => DistanceMetric::Manhattan,
        Ok(v1::DistanceMetric::DotProduct) => DistanceMetric::DotProduct,
        Ok(v1::DistanceMetric::Angular) => DistanceMetric::Angular,
        Err(_) => DistanceMetric::Cosine,
    }
}

fn quantization_to_proto(q: &QuantizationMethod) -> v1::QuantizationConfig {
    match q {
        QuantizationMethod::None => v1::QuantizationConfig {
            method: v1::QuantizationMethod::None as i32,
            subvector_count: 0,
        },
        QuantizationMethod::Scalar8Bit => v1::QuantizationConfig {
            method: v1::QuantizationMethod::Scalar8bit as i32,
            subvector_count: 0,
        },
        QuantizationMethod::ProductQuantization { subvector_count } => v1::QuantizationConfig {
            method: v1::QuantizationMethod::ProductQuantization as i32,
            subvector_count: *subvector_count as u32,
        },
    }
}

fn quantization_from_proto(q: &v1::QuantizationConfig) -> QuantizationMethod {
    match v1::QuantizationMethod::try_from(q.method) {
        Ok(v1::QuantizationMethod::None) => QuantizationMethod::None,
        Ok(v1::QuantizationMethod::Scalar8bit) => QuantizationMethod::Scalar8Bit,
        Ok(v1::QuantizationMethod::ProductQuantization) => {
            QuantizationMethod::ProductQuantization {
                subvector_count: q.subvector_count as usize,
            }
        }
        Err(_) => QuantizationMethod::None,
    }
}
