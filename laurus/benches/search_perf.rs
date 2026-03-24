//! Performance benchmarks for lexical search hot paths.
//!
//! Covers posting list traversal, BM25 scoring, SIMD batch scoring,
//! and compact posting conversion.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use laurus::lexical::index::inverted::core::posting::{Posting, PostingIterator, PostingList};
use laurus::lexical::query::scorer::{BM25Scorer, Scorer};
use laurus::util::simd::numeric::{batch_bm25_final_score, batch_bm25_tf};

/// Build a posting list with `n` postings, each having 3 positions.
fn make_posting_list(n: usize) -> PostingList {
    let mut list = PostingList::new("bench_term".to_string());
    for i in 0..n {
        let doc_id = (i as u64) * 3; // sparse doc IDs
        list.add_posting(Posting::with_positions(doc_id, vec![0, 5, 10]).with_weight(1.0));
    }
    list
}

// ---------------------------------------------------------------------------
// Posting iterator skip_to
// ---------------------------------------------------------------------------

fn bench_posting_skip_to(c: &mut Criterion) {
    let mut group = c.benchmark_group("PostingIterator/skip_to");

    for &size in &[100usize, 1_000, 10_000] {
        let list = make_posting_list(size);
        let postings = list.postings.clone();

        // Pick targets spread across the list.
        let targets: Vec<u64> = (0..100).map(|i| (i * (size as u64) * 3) / 100).collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let mut iter = PostingIterator::new(postings.clone());
                for &target in &targets {
                    iter.skip_to(black_box(target));
                }
            });
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// BM25 single-doc scoring
// ---------------------------------------------------------------------------

fn bench_bm25_scoring(c: &mut Criterion) {
    let scorer = BM25Scorer::new(
        /* doc_freq */ 500, /* total_term_freq */ 5_000,
        /* field_doc_count */ 10_000, /* avg_field_length */ 120.0,
        /* total_docs */ 10_000, /* boost */ 1.0,
    );

    c.bench_function("BM25Scorer/score", |b| {
        b.iter(|| {
            // Score 1000 documents in a tight loop.
            for doc_id in 0u64..1_000 {
                black_box(scorer.score(black_box(doc_id), black_box(3.0), black_box(Some(150.0))));
            }
        });
    });
}

// ---------------------------------------------------------------------------
// SIMD batch scoring
// ---------------------------------------------------------------------------

fn bench_bm25_batch_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("SIMD/batch_bm25");

    for &size in &[64usize, 256, 1_024, 4_096] {
        let tfs: Vec<f32> = (0..size).map(|i| (i % 10 + 1) as f32).collect();
        let norms: Vec<f32> = (0..size).map(|i| 0.8 + (i as f32 * 0.001)).collect();
        let idfs: Vec<f32> = (0..size).map(|i| 1.0 + (i as f32 * 0.01)).collect();
        let boosts: Vec<f32> = vec![1.0; size];

        group.bench_with_input(BenchmarkId::new("batch_bm25_tf", size), &size, |b, _| {
            b.iter(|| {
                black_box(batch_bm25_tf(
                    black_box(&tfs),
                    black_box(1.2),
                    black_box(&norms),
                ));
            });
        });

        group.bench_with_input(
            BenchmarkId::new("batch_bm25_final_score", size),
            &size,
            |b, _| {
                b.iter(|| {
                    black_box(batch_bm25_final_score(
                        black_box(&tfs),
                        black_box(&idfs),
                        black_box(&boosts),
                    ));
                });
            },
        );
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Compact posting conversion
// ---------------------------------------------------------------------------

fn bench_compact_posting(c: &mut Criterion) {
    let mut group = c.benchmark_group("PostingList/to_compact");

    for &size in &[100usize, 1_000, 10_000] {
        let list = make_posting_list(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                black_box(list.to_compact());
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_posting_skip_to,
    bench_bm25_scoring,
    bench_bm25_batch_scoring,
    bench_compact_posting,
);
criterion_main!(benches);
