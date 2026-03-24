//! End-to-end vector search benchmarks for Flat, IVF, and HNSW indexes.
//!
//! Compares construction and search performance across all three index types
//! using the same dataset and query parameters.

use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use laurus::storage::memory::MemoryStorageConfig;
use laurus::storage::{Storage, StorageConfig, StorageFactory};
use laurus::vector::core::distance::DistanceMetric;
use laurus::vector::core::vector::Vector;
use laurus::vector::index::ManagedVectorIndex;
use laurus::vector::index::config::{
    FlatIndexConfig, HnswIndexConfig, IvfIndexConfig, VectorIndexTypeConfig,
};
use laurus::vector::{
    FlatVectorSearcher, HnswSearcher, IvfSearcher, VectorIndexQuery, VectorIndexSearcher,
};
use rand::RngExt;

fn generate_random_vector(dim: usize) -> Vector {
    let mut rng = rand::rng();
    let data: Vec<f32> = (0..dim).map(|_| rng.random::<f32>()).collect();
    Vector::new(data)
}

fn generate_vectors(count: usize, dim: usize) -> Vec<(u64, String, Vector)> {
    (0..count)
        .map(|i| (i as u64, "field".to_string(), generate_random_vector(dim)))
        .collect()
}

fn create_storage() -> Arc<dyn Storage> {
    StorageFactory::create(StorageConfig::Memory(MemoryStorageConfig::default())).unwrap()
}

// ---------------------------------------------------------------------------
// Construction benchmarks
// ---------------------------------------------------------------------------

fn bench_flat_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Flat Construction");
    group.sample_size(10);
    let dim = 128;

    for &count in &[1000, 5000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let vectors = generate_vectors(count, dim);
            b.iter(|| {
                let storage = create_storage();
                let config = FlatIndexConfig {
                    dimension: dim,
                    distance_metric: DistanceMetric::Cosine,
                    ..Default::default()
                };
                let mut index =
                    ManagedVectorIndex::new(VectorIndexTypeConfig::Flat(config), storage, "bench")
                        .unwrap();
                index.add_vectors(vectors.clone()).unwrap();
                index.finalize().unwrap();
            });
        });
    }
    group.finish();
}

fn bench_ivf_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("IVF Construction");
    group.sample_size(10);
    let dim = 128;

    for &count in &[1000, 5000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            let vectors = generate_vectors(count, dim);
            b.iter(|| {
                let storage = create_storage();
                let config = IvfIndexConfig {
                    dimension: dim,
                    distance_metric: DistanceMetric::Cosine,
                    n_clusters: 10,
                    n_probe: 3,
                    ..Default::default()
                };
                let mut index =
                    ManagedVectorIndex::new(VectorIndexTypeConfig::IVF(config), storage, "bench")
                        .unwrap();
                index.add_vectors(vectors.clone()).unwrap();
                index.finalize().unwrap();
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Search benchmarks
// ---------------------------------------------------------------------------

fn bench_flat_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("Flat Search");
    let dim = 128;

    for &count in &[1000, 5000] {
        let vectors = generate_vectors(count, dim);
        let storage = create_storage();
        let config = FlatIndexConfig {
            dimension: dim,
            distance_metric: DistanceMetric::Cosine,
            ..Default::default()
        };
        let mut index =
            ManagedVectorIndex::new(VectorIndexTypeConfig::Flat(config), storage, "flat_bench")
                .unwrap();
        index.add_vectors(vectors).unwrap();
        index.finalize().unwrap();

        let reader = index.reader().unwrap();
        let searcher = FlatVectorSearcher::new(reader).unwrap();
        let query = generate_random_vector(dim);

        group.bench_with_input(BenchmarkId::new("top10", count), &count, |b, _| {
            b.iter(|| {
                let request = VectorIndexQuery::new(query.clone()).top_k(10);
                searcher.search(&request).unwrap()
            });
        });
    }
    group.finish();
}

fn bench_ivf_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("IVF Search");
    let dim = 128;

    for &count in &[1000, 5000] {
        let vectors = generate_vectors(count, dim);
        let storage = create_storage();
        let config = IvfIndexConfig {
            dimension: dim,
            distance_metric: DistanceMetric::Cosine,
            n_clusters: 10,
            n_probe: 3,
            ..Default::default()
        };
        let mut index =
            ManagedVectorIndex::new(VectorIndexTypeConfig::IVF(config), storage, "ivf_bench")
                .unwrap();
        index.add_vectors(vectors).unwrap();
        index.finalize().unwrap();
        index.write().unwrap();

        let reader = index.reader().unwrap();
        let searcher = IvfSearcher::new(reader).unwrap();
        let query = generate_random_vector(dim);

        group.bench_with_input(BenchmarkId::new("top10", count), &count, |b, _| {
            b.iter(|| {
                let request = VectorIndexQuery::new(query.clone()).top_k(10);
                searcher.search(&request).unwrap()
            });
        });
    }
    group.finish();
}

fn bench_hnsw_search_compare(c: &mut Criterion) {
    let mut group = c.benchmark_group("HNSW Search");
    let dim = 128;

    for &count in &[1000, 5000] {
        let vectors = generate_vectors(count, dim);
        let storage = create_storage();
        let config = HnswIndexConfig {
            dimension: dim,
            m: 16,
            ef_construction: 200,
            distance_metric: DistanceMetric::Cosine,
            ..Default::default()
        };
        let mut index =
            ManagedVectorIndex::new(VectorIndexTypeConfig::HNSW(config), storage, "hnsw_bench")
                .unwrap();
        index.add_vectors(vectors).unwrap();
        index.finalize().unwrap();
        index.write().unwrap();

        let reader = index.reader().unwrap();
        let searcher = HnswSearcher::new(reader).unwrap();
        let query = generate_random_vector(dim);

        group.bench_with_input(BenchmarkId::new("top10", count), &count, |b, _| {
            b.iter(|| {
                let request = VectorIndexQuery::new(query.clone()).top_k(10);
                searcher.search(&request).unwrap()
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_flat_construction,
    bench_ivf_construction,
    bench_flat_search,
    bench_ivf_search,
    bench_hnsw_search_compare,
);
criterion_main!(benches);
