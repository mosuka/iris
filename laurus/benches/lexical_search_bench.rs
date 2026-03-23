//! End-to-end lexical search benchmarks.
//!
//! Measures full query execution time including matching, scoring, and collection
//! for various query types: TermQuery, BooleanQuery, PhraseQuery, FuzzyQuery.

use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

use laurus::analysis::analyzer::analyzer::Analyzer;
use laurus::analysis::analyzer::standard::StandardAnalyzer;
use laurus::lexical::core::field::IntegerOption;
use laurus::lexical::{BooleanQuery, FuzzyQuery, PhraseQuery, TermQuery, TextOption};
use laurus::storage::memory::MemoryStorageConfig;
use laurus::storage::{Storage, StorageConfig, StorageFactory};
use laurus::{Document, Engine, LexicalSearchQuery, Result, Schema, SearchRequestBuilder};

/// Create an in-memory storage backend.
fn memory_storage() -> Result<Arc<dyn Storage>> {
    StorageFactory::create(StorageConfig::Memory(MemoryStorageConfig::default()))
}

/// Build a pre-populated engine with `n` documents.
async fn build_engine(n: usize) -> Result<Engine> {
    let storage = memory_storage()?;
    let analyzer: Arc<dyn Analyzer> = Arc::new(StandardAnalyzer::default());

    let schema = Schema::builder()
        .add_text_field("title", TextOption::default())
        .add_text_field("body", TextOption::default())
        .add_text_field("category", TextOption::default())
        .add_integer_field("year", IntegerOption::default())
        .add_default_field("body")
        .build();

    let engine = Engine::builder(storage, schema)
        .analyzer(analyzer)
        .build()
        .await?;

    // Diverse vocabulary for realistic search
    let topics = [
        "rust programming language systems safety concurrency memory ownership",
        "python data science machine learning artificial intelligence numpy",
        "javascript web development frontend backend node react framework",
        "database query optimization indexing performance search engine",
        "network protocol distributed systems cloud computing infrastructure",
        "security cryptography authentication authorization encryption",
        "algorithms data structures sorting searching graph traversal",
        "operating systems kernel processes threads scheduling memory",
    ];
    let categories = ["programming", "data-science", "web", "database", "systems"];

    for i in 0..n {
        let topic = topics[i % topics.len()];
        let body = format!(
            "Document {} about {}. This document covers various aspects of the topic \
             including advanced concepts and practical applications in real world scenarios. \
             The search engine should be able to find this document using relevant terms.",
            i, topic
        );
        let doc = Document::builder()
            .add_text("title", &format!("Title for document {}", i))
            .add_text("body", &body)
            .add_text("category", categories[i % categories.len()])
            .add_integer("year", 2020 + (i % 5) as i64)
            .build();

        engine.add_document(&i.to_string(), doc).await?;
    }

    engine.commit().await?;
    Ok(engine)
}

fn bench_term_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("lexical/term_query");

    for &n in &[100, 1000, 5000] {
        let engine = rt.block_on(build_engine(n)).unwrap();

        group.bench_with_input(BenchmarkId::new("search", n), &n, |b, _| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let query = Box::new(TermQuery::new("body", "programming"));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(query))
                        .limit(10)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });
    }
    group.finish();
}

fn bench_term_query_varying_limit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(build_engine(5000)).unwrap();
    let mut group = c.benchmark_group("lexical/term_query_limit");

    for &limit in &[10, 50, 100, 500] {
        group.bench_with_input(BenchmarkId::new("top", limit), &limit, |b, &limit| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let query = Box::new(TermQuery::new("body", "programming"));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(query))
                        .limit(limit)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });
    }
    group.finish();
}

fn bench_boolean_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("lexical/boolean_query");

    for &n in &[100, 1000, 5000] {
        let engine = rt.block_on(build_engine(n)).unwrap();

        // MUST + MUST (AND)
        group.bench_with_input(BenchmarkId::new("must_and", n), &n, |b, _| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let mut bq = BooleanQuery::new();
                    bq.add_must(Box::new(TermQuery::new("body", "programming")));
                    bq.add_must(Box::new(TermQuery::new("body", "language")));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(Box::new(bq)))
                        .limit(10)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });

        // SHOULD + SHOULD (OR)
        group.bench_with_input(BenchmarkId::new("should_or", n), &n, |b, _| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let mut bq = BooleanQuery::new();
                    bq.add_should(Box::new(TermQuery::new("body", "rust")));
                    bq.add_should(Box::new(TermQuery::new("body", "python")));
                    bq.add_should(Box::new(TermQuery::new("body", "javascript")));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(Box::new(bq)))
                        .limit(10)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });

        // MUST + MUST_NOT (AND NOT)
        group.bench_with_input(BenchmarkId::new("must_not", n), &n, |b, _| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let mut bq = BooleanQuery::new();
                    bq.add_must(Box::new(TermQuery::new("body", "programming")));
                    bq.add_must_not(Box::new(TermQuery::new("body", "python")));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(Box::new(bq)))
                        .limit(10)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });
    }
    group.finish();
}

fn bench_phrase_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("lexical/phrase_query");

    for &n in &[100, 1000, 5000] {
        let engine = rt.block_on(build_engine(n)).unwrap();

        group.bench_with_input(BenchmarkId::new("two_terms", n), &n, |b, _| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let query = Box::new(PhraseQuery::new(
                        "body",
                        vec!["search".into(), "engine".into()],
                    ));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(query))
                        .limit(10)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("three_terms", n), &n, |b, _| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let query = Box::new(PhraseQuery::new(
                        "body",
                        vec!["practical".into(), "applications".into(), "in".into()],
                    ));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(query))
                        .limit(10)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });
    }
    group.finish();
}

fn bench_fuzzy_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("lexical/fuzzy_query");

    for &n in &[100, 1000, 5000] {
        let engine = rt.block_on(build_engine(n)).unwrap();

        group.bench_with_input(BenchmarkId::new("edit1", n), &n, |b, _| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let query = Box::new(FuzzyQuery::new("body", "programing").max_edits(1));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(query))
                        .limit(10)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("edit2", n), &n, |b, _| {
            b.to_async(&rt).iter(|| {
                let engine = &engine;
                async move {
                    let query = Box::new(FuzzyQuery::new("body", "progrming").max_edits(2));
                    let request = SearchRequestBuilder::new()
                        .lexical_query(LexicalSearchQuery::Obj(query))
                        .limit(10)
                        .build();
                    black_box(engine.search(request).await.unwrap())
                }
            });
        });
    }
    group.finish();
}

fn bench_dsl_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let engine = rt.block_on(build_engine(5000)).unwrap();
    let mut group = c.benchmark_group("lexical/dsl_query");

    group.bench_function("simple_term", |b| {
        b.to_async(&rt).iter(|| {
            let engine = &engine;
            async move {
                let request = SearchRequestBuilder::new()
                    .lexical_query(LexicalSearchQuery::Dsl("body:programming".to_string()))
                    .limit(10)
                    .build();
                black_box(engine.search(request).await.unwrap())
            }
        });
    });

    group.bench_function("boolean_and", |b| {
        b.to_async(&rt).iter(|| {
            let engine = &engine;
            async move {
                let request = SearchRequestBuilder::new()
                    .lexical_query(LexicalSearchQuery::Dsl(
                        "body:programming AND body:language".to_string(),
                    ))
                    .limit(10)
                    .build();
                black_box(engine.search(request).await.unwrap())
            }
        });
    });

    group.bench_function("boolean_or", |b| {
        b.to_async(&rt).iter(|| {
            let engine = &engine;
            async move {
                let request = SearchRequestBuilder::new()
                    .lexical_query(LexicalSearchQuery::Dsl(
                        "body:rust OR body:python OR body:javascript".to_string(),
                    ))
                    .limit(10)
                    .build();
                black_box(engine.search(request).await.unwrap())
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_term_query,
    bench_term_query_varying_limit,
    bench_boolean_query,
    bench_phrase_query,
    bench_fuzzy_query,
    bench_dsl_query,
);
criterion_main!(benches);
