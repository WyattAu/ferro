use common::metadata::{ContentHash, FileMetadata};
use common::storage::StorageEngine;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ferro_core::metadata::InMemoryMetadataStore;
use ferro_core::metadata::MetadataStore;
use ferro_server::storage::InMemoryStorageEngine;
use tokio::runtime::Runtime;

mod helpers;
use helpers::*;

fn bench_in_memory_storage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("storage/in_memory");

    let sizes: &[usize] = &[1024, 1024 * 1024];
    let size_labels: &[&str] = &["1KB", "1MB"];

    for (size, label) in sizes.iter().zip(size_labels.iter()) {
        let body = generate_test_body(*size);

        // PUT
        group.bench_with_input(BenchmarkId::new("put", label), size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let engine = InMemoryStorageEngine::new();
                    let result = engine
                        .put("/bench.txt", body.clone(), "bench")
                        .await
                        .unwrap();
                    black_box(result);
                })
            })
        });

        // GET (file pre-created)
        group.bench_with_input(BenchmarkId::new("get", label), size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let engine = InMemoryStorageEngine::new();
                    let body = generate_test_body(size);
                    engine.put("/bench.txt", body, "bench").await.unwrap();
                    let data = engine.get("/bench.txt").await.unwrap();
                    black_box(data);
                })
            })
        });

        // HEAD
        group.bench_with_input(BenchmarkId::new("head", label), size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let engine = InMemoryStorageEngine::new();
                    let body = generate_test_body(size);
                    engine.put("/bench.txt", body, "bench").await.unwrap();
                    let meta = engine.head("/bench.txt").await.unwrap();
                    black_box(meta);
                })
            })
        });

        // DELETE
        group.bench_with_input(BenchmarkId::new("delete", label), size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let engine = InMemoryStorageEngine::new();
                    let body = generate_test_body(size);
                    engine.put("/bench.txt", body, "bench").await.unwrap();
                    engine.delete("/bench.txt").await.unwrap();
                })
            })
        });
    }

    // LIST with different item counts
    for &item_count in &[10usize, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("list", item_count), &item_count, |b, &count| {
            b.iter(|| {
                rt.block_on(async {
                    let engine = InMemoryStorageEngine::new();
                    engine
                        .create_collection("/bench_dir", "bench")
                        .await
                        .unwrap();
                    let body = generate_test_body(64);
                    for i in 0..count {
                        engine
                            .put(&format!("/bench_dir/file_{}.txt", i), body.clone(), "bench")
                            .await
                            .unwrap();
                    }
                    let items = engine.list("/bench_dir").await.unwrap();
                    black_box(items);
                })
            })
        });
    }

    // EXISTS
    group.bench_function("exists", |b| {
        b.iter(|| {
            rt.block_on(async {
                let engine = InMemoryStorageEngine::new();
                let body = generate_test_body(64);
                engine.put("/bench.txt", body, "bench").await.unwrap();
                let exists = engine.exists("/bench.txt").await.unwrap();
                black_box(exists);
            })
        })
    });

    group.finish();
}

fn bench_metadata_store(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("metadata/in_memory");

    // PUT metadata
    group.bench_function("put", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemoryMetadataStore::new();
                let hash = ContentHash::compute(b"benchmark file content");
                let meta = FileMetadata::new("/bench.txt".to_string(), hash, 1024, "bench".to_string());
                store.put(meta).await.unwrap();
            })
        })
    });

    // GET metadata
    group.bench_function("get", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemoryMetadataStore::new();
                let hash = ContentHash::compute(b"benchmark file content");
                let meta = FileMetadata::new("/bench.txt".to_string(), hash, 1024, "bench".to_string());
                store.put(meta).await.unwrap();
                let result = store.get("/bench.txt").await.unwrap();
                black_box(result);
            })
        })
    });

    // LIST metadata with different counts
    for &item_count in &[10usize, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("list", item_count),
            &item_count,
            |b, &count| {
                b.iter(|| {
                    rt.block_on(async {
                        let store = InMemoryMetadataStore::new();
                        let hash = ContentHash::compute(b"benchmark file content");
                        for i in 0..count {
                            let meta = FileMetadata::new(
                                format!("/docs/file_{}.txt", i),
                                hash.clone(),
                                1024,
                                "bench".to_string(),
                            );
                            store.put(meta).await.unwrap();
                        }
                        let items = store.list("/docs").await.unwrap();
                        black_box(items);
                    })
                })
            },
        );
    }

    // DELETE metadata
    group.bench_function("delete", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemoryMetadataStore::new();
                let hash = ContentHash::compute(b"benchmark file content");
                let meta = FileMetadata::new("/bench.txt".to_string(), hash, 1024, "bench".to_string());
                store.put(meta).await.unwrap();
                store.delete("/bench.txt").await.unwrap();
            })
        })
    });

    // EXISTS metadata
    group.bench_function("exists", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemoryMetadataStore::new();
                let hash = ContentHash::compute(b"benchmark file content");
                let meta = FileMetadata::new("/bench.txt".to_string(), hash, 1024, "bench".to_string());
                store.put(meta).await.unwrap();
                let exists = store.exists("/bench.txt").await.unwrap();
                black_box(exists);
            })
        })
    });

    group.finish();
}

criterion_group!(benches, bench_in_memory_storage, bench_metadata_store);
criterion_main!(benches);
