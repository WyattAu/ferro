use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ferro_common::storage::StorageEngine;
use ferro_core::storage::InMemoryStorageEngine;
use std::sync::Arc;
use std::time::Instant;

fn make_owner() -> String {
    "bench-user".to_string()
}

fn make_content(size: usize) -> bytes::Bytes {
    bytes::Bytes::from(vec![0xAB; size])
}

fn bench_upload(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("owncloud_upload");

    let sizes: Vec<(usize, &str)> = vec![
        (1_024, "1KB"),
        (10_240, "10KB"),
        (102_400, "100KB"),
        (1_048_576, "1MB"),
        (10_485_760, "10MB"),
    ];

    for (size, label) in &sizes {
        let content = make_content(*size);
        let size_mb = *size as f64 / (1024.0 * 1024.0);

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("put", label), &content, |b, content| {
            let storage = Arc::new(InMemoryStorageEngine::new());
            b.to_async(&rt).iter(|| {
                let storage = storage.clone();
                let content = content.clone();
                async move {
                    let path = format!("/owncloud/sync/{}", uuid::Uuid::new_v4());
                    storage.put(&path, content, &make_owner()).await.unwrap();
                }
            })
        });

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("put_throughput_mb", label),
            &size_mb,
            |b, _size_mb| {
                let storage = Arc::new(InMemoryStorageEngine::new());
                let content = make_content(*size);
                b.iter(|| {
                    let data = content.clone();
                    let path = format!("/owncloud/sync/{}", uuid::Uuid::new_v4());
                    let start = Instant::now();
                    rt.block_on(async {
                        storage.put(&path, data, &make_owner()).await.unwrap();
                    });
                    let elapsed = start.elapsed().as_secs_f64();
                    let throughput = size_mb / elapsed;
                    std::hint::black_box(throughput);
                });
            },
        );
    }
    group.finish();
}

fn bench_download(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("owncloud_download");

    let sizes: Vec<(usize, &str)> = vec![
        (1_024, "1KB"),
        (10_240, "10KB"),
        (102_400, "100KB"),
        (1_048_576, "1MB"),
        (10_485_760, "10MB"),
    ];

    for (size, label) in &sizes {
        let storage = Arc::new(InMemoryStorageEngine::new());
        let path = format!("/owncloud/files/{}", label);
        let content = make_content(*size);
        let size_mb = *size as f64 / (1024.0 * 1024.0);

        rt.block_on(async {
            storage.put(&path, content, &make_owner()).await.unwrap();
        });

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_function(BenchmarkId::new("get", label), |b| {
            let storage = storage.clone();
            let path = path.clone();
            b.to_async(&rt).iter(|| {
                let storage = storage.clone();
                let path = path.clone();
                async move {
                    let _ = storage.get(&path).await.unwrap();
                }
            })
        });

        group.bench_with_input(
            BenchmarkId::new("get_throughput_mb", label),
            &size_mb,
            |b, _size_mb| {
                let storage = storage.clone();
                let path = path.clone();
                b.iter(|| {
                    let storage = storage.clone();
                    let path = path.clone();
                    let start = Instant::now();
                    rt.block_on(async {
                        let _ = storage.get(&path).await.unwrap();
                    });
                    let elapsed = start.elapsed().as_secs_f64();
                    let throughput = size_mb / elapsed;
                    std::hint::black_box(throughput);
                });
            },
        );
    }
    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("owncloud_concurrent");

    for concurrency in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_put", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    let storage = Arc::new(InMemoryStorageEngine::new());
                    rt.block_on(async {
                        let mut handles = Vec::with_capacity(concurrency);
                        for i in 0..concurrency {
                            let storage = storage.clone();
                            handles.push(tokio::spawn(async move {
                                let path = format!("/owncloud/concurrent/file_{}.dat", i);
                                let content = make_content(10_240);
                                storage.put(&path, content, &make_owner()).await.unwrap();
                            }));
                        }
                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("concurrent_get", concurrency),
            &concurrency,
            |b, &concurrency| {
                let storage = Arc::new(InMemoryStorageEngine::new());
                rt.block_on(async {
                    for i in 0..concurrency {
                        let path = format!("/owncloud/concurrent/file_{}.dat", i);
                        storage
                            .put(&path, make_content(10_240), &make_owner())
                            .await
                            .unwrap();
                    }
                });

                b.iter(|| {
                    let storage = storage.clone();
                    rt.block_on(async {
                        let mut handles = Vec::with_capacity(concurrency);
                        for i in 0..concurrency {
                            let storage = storage.clone();
                            let path = format!("/owncloud/concurrent/file_{}.dat", i);
                            handles.push(tokio::spawn(async move {
                                let _ = storage.get(&path).await.unwrap();
                            }));
                        }
                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("concurrent_mixed", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.iter(|| {
                    let storage = Arc::new(InMemoryStorageEngine::new());
                    rt.block_on(async {
                        let mut handles = Vec::with_capacity(concurrency * 2);
                        let half = concurrency / 2;
                        for i in 0..half {
                            let storage = storage.clone();
                            handles.push(tokio::spawn(async move {
                                let path = format!("/owncloud/mixed/write_{}.dat", i);
                                let content = make_content(10_240);
                                storage.put(&path, content, &make_owner()).await.unwrap();
                            }));
                        }
                        for i in 0..half {
                            let storage = storage.clone();
                            handles.push(tokio::spawn(async move {
                                let path = format!("/owncloud/mixed/read_{}.dat", i);
                                storage
                                    .put(&path, make_content(10_240), &make_owner())
                                    .await
                                    .unwrap();
                                let _ = storage.get(&path).await.unwrap();
                            }));
                        }
                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );
    }
    group.finish();
}

fn bench_propfind(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("owncloud_propfind");

    use ferro_dav::xml_ext::{DavProp, DavResponse, PropStat};

    let entry_counts: Vec<(usize, &str)> = vec![
        (100, "100_entries"),
        (1000, "1000_entries"),
        (10_000, "10000_entries"),
    ];

    for (count, label) in &entry_counts {
        let responses: Vec<DavResponse> = (0..*count)
            .map(|i| DavResponse {
                href: format!(
                    "/owncloud/remote.php/dav/files/user/Photos/photo_{:06}.jpg",
                    i
                ),
                propstats: vec![PropStat {
                    status: 200,
                    props: vec![
                        DavProp {
                            name: "D:resourcetype".to_string(),
                            namespace: None,
                            value: Some(String::new()),
                        },
                        DavProp {
                            name: "D:getcontentlength".to_string(),
                            namespace: None,
                            value: Some("5242880".to_string()),
                        },
                        DavProp {
                            name: "D:getlastmodified".to_string(),
                            namespace: None,
                            value: Some("Wed, 01 Jan 2025 00:00:00 GMT".to_string()),
                        },
                        DavProp {
                            name: "D:getetag".to_string(),
                            namespace: None,
                            value: Some(format!("\"etag-{}\"", i)),
                        },
                        DavProp {
                            name: "D:getcontenttype".to_string(),
                            namespace: None,
                            value: Some("image/jpeg".to_string()),
                        },
                    ],
                }],
            })
            .collect();

        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(
            BenchmarkId::new("build_multistatus", label),
            &responses,
            |b, responses| {
                b.iter(|| {
                    let xml = ferro_dav::xml_ext::build_dav_multistatus(responses);
                    std::hint::black_box(xml);
                });
            },
        );

        let storage = Arc::new(InMemoryStorageEngine::new());
        rt.block_on(async {
            for i in 0..*count {
                let path = format!(
                    "/owncloud/remote.php/dav/files/user/Photos/photo_{:06}.jpg",
                    i
                );
                let content = make_content(1024);
                storage.put(&path, content, &make_owner()).await.unwrap();
            }
        });

        group.bench_with_input(BenchmarkId::new("list_directory", label), count, |b, _| {
            let storage = storage.clone();
            b.to_async(&rt).iter(|| {
                let storage = storage.clone();
                async move {
                    let _ = storage
                        .list("/owncloud/remote.php/dav/files/user/Photos/")
                        .await
                        .unwrap();
                }
            });
        });

        group.bench_with_input(
            BenchmarkId::new("list_all_depth_1", label),
            count,
            |b, _| {
                let storage = storage.clone();
                b.to_async(&rt).iter(|| {
                    let storage = storage.clone();
                    async move {
                        let _ = storage
                            .list_all("/owncloud/remote.php/dav/files/user/", 2)
                            .await
                            .unwrap();
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("head_operation", label),
            count,
            |b, &count| {
                let storage = storage.clone();
                let target = format!(
                    "/owncloud/remote.php/dav/files/user/Photos/photo_{:06}.jpg",
                    count / 2
                );
                b.to_async(&rt).iter(|| {
                    let storage = storage.clone();
                    let target = target.clone();
                    async move {
                        let _ = storage.head(&target).await.unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

fn bench_sync_workflow(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("owncloud_sync_workflow");

    group.bench_function("single_file_sync_cycle", |b| {
        let storage = Arc::new(InMemoryStorageEngine::new());
        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                let path = "/owncloud/sync/doc.txt";
                let content = make_content(10240);
                let meta = storage
                    .put(path, content.clone(), &make_owner())
                    .await
                    .unwrap();
                let _ = storage.head(path).await.unwrap();
                let _ = storage.get(path).await.unwrap();
                let _ = storage.exists(path).await.unwrap();
                let _ = storage.list("/owncloud/sync/").await.unwrap();
                std::hint::black_box(meta);
            }
        });
    });

    group.bench_function("batch_sync_50_files", |b| {
        b.iter(|| {
            let storage = Arc::new(InMemoryStorageEngine::new());
            rt.block_on(async {
                for i in 0..50 {
                    let path = format!("/owncloud/batch/sync_{}.txt", i);
                    let content = make_content(4096);
                    storage.put(&path, content, &make_owner()).await.unwrap();
                }
                let _ = storage.list("/owncloud/batch/").await.unwrap();
                for i in 0..50 {
                    let path = format!("/owncloud/batch/sync_{}.txt", i);
                    let _ = storage.get(&path).await.unwrap();
                }
            });
        });
    });

    group.bench_function("rename_and_move", |b| {
        let storage = Arc::new(InMemoryStorageEngine::new());
        rt.block_on(async {
            storage
                .put(
                    "/owncloud/rename/original.txt",
                    make_content(8192),
                    &make_owner(),
                )
                .await
                .unwrap();
        });

        b.to_async(&rt).iter(|| {
            let storage = storage.clone();
            async move {
                storage
                    .move_path(
                        "/owncloud/rename/original.txt",
                        "/owncloud/rename/renamed.txt",
                    )
                    .await
                    .unwrap();
                let _ = storage.get("/owncloud/rename/renamed.txt").await.unwrap();
                storage
                    .move_path(
                        "/owncloud/rename/renamed.txt",
                        "/owncloud/rename/original.txt",
                    )
                    .await
                    .unwrap();
            }
        });
    });

    group.bench_function("concurrent_sync_10_clients", |b| {
        b.iter(|| {
            let storage = Arc::new(InMemoryStorageEngine::new());
            rt.block_on(async {
                let mut handles = Vec::with_capacity(10);
                for client in 0..10 {
                    let storage = storage.clone();
                    handles.push(tokio::spawn(async move {
                        for i in 0..10 {
                            let path =
                                format!("/owncloud/clients/client_{}/file_{}.txt", client, i);
                            let content = make_content(2048);
                            let _ = storage.put(&path, content, &make_owner()).await;
                            let _ = storage.get(&path).await;
                        }
                    }));
                }
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });

    group.bench_function("delete_cascade", |b| {
        b.iter(|| {
            let storage = Arc::new(InMemoryStorageEngine::new());
            rt.block_on(async {
                for i in 0..100 {
                    let path = format!("/owncloud/delete/file_{:03}.txt", i);
                    storage
                        .put(&path, make_content(512), &make_owner())
                        .await
                        .unwrap();
                }
                for i in 0..100 {
                    let path = format!("/owncloud/delete/file_{:03}.txt", i);
                    storage.delete(&path).await.unwrap();
                }
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_upload,
    bench_download,
    bench_concurrent_operations,
    bench_propfind,
    bench_sync_workflow,
);
criterion_main!(benches);
