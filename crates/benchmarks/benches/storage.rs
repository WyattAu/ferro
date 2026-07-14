use criterion::{Criterion, criterion_group, criterion_main};
use ferro_common::storage::StorageEngine;
use ferro_core::storage::InMemoryStorageEngine;
use std::sync::Arc;

fn bench_put(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("put");
    group.bench_function("1kb", |b| {
        let storage = Arc::new(InMemoryStorageEngine::new());
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage
                    .put("/bench/test.txt", bytes::Bytes::from(vec![0u8; 1024]), "bench")
                    .await
                    .unwrap();
            }
        })
    });
    group.bench_function("10kb", |b| {
        let storage = Arc::new(InMemoryStorageEngine::new());
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage
                    .put("/bench/test.txt", bytes::Bytes::from(vec![0u8; 10_240]), "bench")
                    .await
                    .unwrap();
            }
        })
    });
    group.bench_function("100kb", |b| {
        let storage = Arc::new(InMemoryStorageEngine::new());
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage
                    .put("/bench/test.txt", bytes::Bytes::from(vec![0u8; 102_400]), "bench")
                    .await
                    .unwrap();
            }
        })
    });
    group.finish();
}

fn bench_get(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let storage = Arc::new(InMemoryStorageEngine::new());

    rt.block_on(async {
        storage
            .put("/bench/test.txt", bytes::Bytes::from(vec![0u8; 10_240]), "bench")
            .await
            .unwrap();
    });

    c.bench_function("get_10kb", |b| {
        let storage = storage.clone();
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage.get("/bench/test.txt").await.unwrap();
            }
        })
    });
}

fn bench_list(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let storage = Arc::new(InMemoryStorageEngine::new());

    rt.block_on(async {
        for i in 0..100u32 {
            storage
                .put(
                    &format!("/bench/dir/file_{:04}.txt", i),
                    bytes::Bytes::from_static(b"test content"),
                    "bench",
                )
                .await
                .unwrap();
        }
    });

    c.bench_function("list_100_files", |b| {
        let storage = storage.clone();
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage.list("/bench/dir/").await.unwrap();
            }
        })
    });
}

fn bench_delete(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("delete", |b| {
        b.to_async(&rt).iter(|| async {
            let storage = InMemoryStorageEngine::new();
            storage
                .put("/bench/test.txt", bytes::Bytes::from_static(b"data"), "bench")
                .await
                .unwrap();
            storage.delete("/bench/test.txt").await.unwrap();
        })
    });
}

fn bench_exists(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let storage = Arc::new(InMemoryStorageEngine::new());

    rt.block_on(async {
        storage
            .put("/bench/test.txt", bytes::Bytes::from_static(b"data"), "bench")
            .await
            .unwrap();
    });

    let mut group = c.benchmark_group("exists");
    group.bench_function("hit", |b| {
        let storage = storage.clone();
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage.exists("/bench/test.txt").await.unwrap();
            }
        })
    });
    group.bench_function("miss", |b| {
        let storage = storage.clone();
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage.exists("/bench/nonexistent").await.unwrap();
            }
        })
    });
    group.finish();
}

fn bench_head(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let storage = Arc::new(InMemoryStorageEngine::new());

    rt.block_on(async {
        storage
            .put(
                "/bench/test.txt",
                bytes::Bytes::from_static(b"test data content"),
                "bench",
            )
            .await
            .unwrap();
    });

    c.bench_function("head", |b| {
        let storage = storage.clone();
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage.head("/bench/test.txt").await.unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    bench_put,
    bench_get,
    bench_list,
    bench_delete,
    bench_exists,
    bench_head,
);
criterion_main!(benches);
