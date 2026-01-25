//! Vector field configuration options.
//!
//! This module defines options for configuring vector fields, including
//! index types and parameters for different algorithms (Flat, HNSW, IVF).

use serde::{Deserialize, Serialize};

use crate::vector::core::distance::DistanceMetric;
use crate::vector::core::quantization;

/// Options for vector fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "options", rename_all = "snake_case")]
pub enum VectorOption {
    /// Flat index options.
    Flat(FlatOption),
    /// HNSW index options.
    Hnsw(HnswOption),
    /// IVF index options.
    Ivf(IvfOption),
}

impl Default for VectorOption {
    fn default() -> Self {
        VectorOption::Flat(FlatOption::default())
    }
}

impl VectorOption {
    /// Get the dimension of the vector field.
    pub fn dimension(&self) -> usize {
        match self {
            VectorOption::Flat(opt) => opt.dimension,
            VectorOption::Hnsw(opt) => opt.dimension,
            VectorOption::Ivf(opt) => opt.dimension,
        }
    }

    /// Get the distance metric.
    pub fn distance(&self) -> DistanceMetric {
        match self {
            VectorOption::Flat(opt) => opt.distance,
            VectorOption::Hnsw(opt) => opt.distance,
            VectorOption::Ivf(opt) => opt.distance,
        }
    }

    /// Get the base weight.
    pub fn base_weight(&self) -> f32 {
        match self {
            VectorOption::Flat(opt) => opt.base_weight,
            VectorOption::Hnsw(opt) => opt.base_weight,
            VectorOption::Ivf(opt) => opt.base_weight,
        }
    }

    /// Get the index kind.
    pub fn index_kind(&self) -> VectorIndexKind {
        match self {
            VectorOption::Flat(_) => VectorIndexKind::Flat,
            VectorOption::Hnsw(_) => VectorIndexKind::Hnsw,
            VectorOption::Ivf(_) => VectorIndexKind::Ivf,
        }
    }
}

/// Options for Flat vector index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatOption {
    pub dimension: usize,
    #[serde(default = "default_distance_metric")]
    pub distance: DistanceMetric,
    #[serde(default = "default_weight")]
    pub base_weight: f32,
    #[serde(default)]
    pub quantizer: Option<quantization::QuantizationMethod>,
}

impl Default for FlatOption {
    fn default() -> Self {
        Self {
            dimension: 128,
            distance: default_distance_metric(),
            base_weight: default_weight(),
            quantizer: None,
        }
    }
}

/// Options for HNSW vector index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswOption {
    pub dimension: usize,
    #[serde(default = "default_distance_metric")]
    pub distance: DistanceMetric,
    #[serde(default = "default_getting_m")]
    pub m: usize,
    #[serde(default = "default_getting_ef_construction")]
    pub ef_construction: usize,
    #[serde(default = "default_weight")]
    pub base_weight: f32,
    #[serde(default)]
    pub quantizer: Option<quantization::QuantizationMethod>,
}

fn default_getting_m() -> usize {
    16
}

fn default_getting_ef_construction() -> usize {
    200
}

impl Default for HnswOption {
    fn default() -> Self {
        Self {
            dimension: 128,
            distance: default_distance_metric(),
            m: default_getting_m(),
            ef_construction: default_getting_ef_construction(),
            base_weight: default_weight(),
            quantizer: None,
        }
    }
}

/// Options for IVF vector index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IvfOption {
    pub dimension: usize,
    #[serde(default = "default_distance_metric")]
    pub distance: DistanceMetric,
    #[serde(default = "default_getting_n_clusters")]
    pub n_clusters: usize,
    #[serde(default = "default_getting_n_probe")]
    pub n_probe: usize,
    #[serde(default = "default_weight")]
    pub base_weight: f32,
    #[serde(default)]
    pub quantizer: Option<quantization::QuantizationMethod>,
}

fn default_getting_n_clusters() -> usize {
    100
}

fn default_getting_n_probe() -> usize {
    1
}

impl Default for IvfOption {
    fn default() -> Self {
        Self {
            dimension: 128,
            distance: default_distance_metric(),
            n_clusters: default_getting_n_clusters(),
            n_probe: default_getting_n_probe(),
            base_weight: default_weight(),
            quantizer: None,
        }
    }
}

/// The type of vector index to use.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VectorIndexKind {
    /// Flat (brute-force) index - exact but slower for large datasets.
    Flat,
    /// HNSW (Hierarchical Navigable Small World) - approximate but fast.
    Hnsw,
    /// IVF (Inverted File Index) - approximate with clustering.
    Ivf,
}

// From implementations for VectorOption
impl From<FlatOption> for VectorOption {
    fn from(opt: FlatOption) -> Self {
        VectorOption::Flat(opt)
    }
}

impl From<HnswOption> for VectorOption {
    fn from(opt: HnswOption) -> Self {
        VectorOption::Hnsw(opt)
    }
}

impl From<IvfOption> for VectorOption {
    fn from(opt: IvfOption) -> Self {
        VectorOption::Ivf(opt)
    }
}

// Builder pattern for FlatOption
impl FlatOption {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            ..Default::default()
        }
    }

    pub fn dimension(mut self, dimension: usize) -> Self {
        self.dimension = dimension;
        self
    }

    pub fn distance(mut self, distance: DistanceMetric) -> Self {
        self.distance = distance;
        self
    }

    pub fn base_weight(mut self, weight: f32) -> Self {
        self.base_weight = weight;
        self
    }

    pub fn quantizer(mut self, quantizer: quantization::QuantizationMethod) -> Self {
        self.quantizer = Some(quantizer);
        self
    }
}

// Builder pattern for HnswOption
impl HnswOption {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            ..Default::default()
        }
    }

    pub fn dimension(mut self, dimension: usize) -> Self {
        self.dimension = dimension;
        self
    }

    pub fn distance(mut self, distance: DistanceMetric) -> Self {
        self.distance = distance;
        self
    }

    pub fn m(mut self, m: usize) -> Self {
        self.m = m;
        self
    }

    pub fn ef_construction(mut self, ef: usize) -> Self {
        self.ef_construction = ef;
        self
    }

    pub fn base_weight(mut self, weight: f32) -> Self {
        self.base_weight = weight;
        self
    }

    pub fn quantizer(mut self, quantizer: quantization::QuantizationMethod) -> Self {
        self.quantizer = Some(quantizer);
        self
    }
}

// Builder pattern for IvfOption
impl IvfOption {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            ..Default::default()
        }
    }

    pub fn dimension(mut self, dimension: usize) -> Self {
        self.dimension = dimension;
        self
    }

    pub fn distance(mut self, distance: DistanceMetric) -> Self {
        self.distance = distance;
        self
    }

    pub fn n_clusters(mut self, n: usize) -> Self {
        self.n_clusters = n;
        self
    }

    pub fn n_probe(mut self, n: usize) -> Self {
        self.n_probe = n;
        self
    }

    pub fn base_weight(mut self, weight: f32) -> Self {
        self.base_weight = weight;
        self
    }

    pub fn quantizer(mut self, quantizer: quantization::QuantizationMethod) -> Self {
        self.quantizer = Some(quantizer);
        self
    }
}

// Helpers

fn default_distance_metric() -> DistanceMetric {
    DistanceMetric::Cosine
}

fn default_weight() -> f32 {
    1.0
}
