use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use iris::storage::memory::MemoryStorageConfig;
use iris::storage::{StorageConfig, StorageFactory};
use iris::vector::core::distance::DistanceMetric;
use iris::vector::core::vector::Vector;
use iris::vector::index::ManagedVectorIndex;
use iris::vector::index::config::{HnswIndexConfig, VectorIndexTypeConfig};
use rand::RngExt;

fn generate_random_vector(dim: usize) -> Vector {
    let mut rng = rand::rng();
    let data: Vec<f32> = (0..dim).map(|_| rng.random::<f32>()).collect();
    Vector::new(data)
}

fn generate_vectors(count: usize, dim: usize) -> Vec<(u64, String, Vector)> {
    (0..count)
        .map(|i| (i as u64, format!("doc_{}", i), generate_random_vector(dim)))
        .collect()
}

fn bench_hnsw_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("HNSW Construction");
    group.sample_size(10);
    let dim = 128;
    let vector_counts = [1000, 5000];

    for count in vector_counts.iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            let vectors = generate_vectors(count, dim);
            b.iter(|| {
                let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
                let storage = StorageFactory::create(storage_config).unwrap();

                let config = HnswIndexConfig {
                    dimension: dim,
                    m: 16,
                    ef_construction: 200,
                    distance_metric: DistanceMetric::Cosine,
                    ..Default::default()
                };
                let type_config = VectorIndexTypeConfig::HNSW(config);
                let mut index =
                    ManagedVectorIndex::new(type_config, storage, "bench_vectors").unwrap();

                index.add_vectors(vectors.clone()).unwrap();
                index.finalize().unwrap();
            })
        });
    }
    group.finish();
}

fn bench_hnsw_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("HNSW Search");
    group.sample_size(10);
    let dim = 128;
    let count = 5000;

    // Setup index once
    let vectors = generate_vectors(count, dim);
    let storage_config = StorageConfig::Memory(MemoryStorageConfig::default());
    let storage = StorageFactory::create(storage_config).unwrap();

    let config = HnswIndexConfig {
        dimension: dim,
        m: 16,
        ef_construction: 200,
        distance_metric: DistanceMetric::Cosine,
        ..Default::default()
    };
    let type_config = VectorIndexTypeConfig::HNSW(config);
    let mut index = ManagedVectorIndex::new(type_config, storage, "bench_vectors_search").unwrap();
    index.add_vectors(vectors).unwrap();
    index.finalize().unwrap();

    let reader = index.reader().unwrap();
    let query_vector = generate_random_vector(dim);

    use iris::vector::HnswSearcher;
    use iris::vector::VectorIndexSearchRequest;
    use iris::vector::VectorIndexSearcher;

    let searcher = HnswSearcher::new(reader).unwrap();

    group.bench_function("search_10_neighbors", |b| {
        b.iter(|| {
            let request = VectorIndexSearchRequest::new(query_vector.clone()).top_k(10);
            let _results = searcher.search(&request).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, bench_hnsw_construction, bench_hnsw_search);
criterion_main!(benches);
