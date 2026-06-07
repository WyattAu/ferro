use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ferro_common::storage::StorageEngine;
use ferro_core::storage::InMemoryStorageEngine;
use ferro_search_index::{Document, SearchIndex};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn make_owner() -> String {
    "bench-user".to_string()
}

fn make_content(size: usize) -> bytes::Bytes {
    bytes::Bytes::from(vec![0xAB; size])
}

fn percentile(sorted: &mut [f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx]
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

fn populate_search_index(index: &SearchIndex, count: usize) {
    for i in 0..count {
        let name = if i % 3 == 0 {
            format!("presentation_{}.pptx", i)
        } else if i % 3 == 1 {
            format!("report_{}.pdf", i)
        } else {
            format!("spreadsheet_{}.xlsx", i)
        };
        let path = format!("/owncloud/remote.php/dav/files/user/Documents/{}", name);
        let mut metadata = HashMap::new();
        metadata.insert(
            "owner".to_string(),
            if i % 2 == 0 {
                "alice".to_string()
            } else {
                "bob".to_string()
            },
        );

        let doc = Document {
            id: format!("doc-{}", i),
            fields: {
                let mut f = HashMap::new();
                f.insert("name".to_string(), name);
                f.insert("path".to_string(), path);
                f.insert(
                    "content_type".to_string(),
                    if i % 3 == 0 {
                        "presentation".to_string()
                    } else if i % 3 == 1 {
                        "pdf".to_string()
                    } else {
                        "spreadsheet".to_string()
                    },
                );
                f
            },
            metadata,
        };
        index.add_document(doc).unwrap();
    }
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("owncloud_search");

    let doc_counts: Vec<(usize, &str)> = vec![
        (100, "100_docs"),
        (1_000, "1000_docs"),
        (10_000, "10000_docs"),
    ];

    for &(count, label) in &doc_counts {
        let index = SearchIndex::new(vec![
            "name".to_string(),
            "path".to_string(),
            "content_type".to_string(),
        ]);
        populate_search_index(&index, count);

        group.throughput(Throughput::Elements(1));

        group.bench_function(BenchmarkId::new("term_search", label), |b| {
            b.iter(|| {
                let results = index.search("report");
                std::hint::black_box(results.len());
            });
        });

        group.bench_function(BenchmarkId::new("prefix_search", label), |b| {
            b.iter(|| {
                let results = index.suggest("rep", 10);
                std::hint::black_box(results.len());
            });
        });

        group.bench_function(BenchmarkId::new("field_search", label), |b| {
            b.iter(|| {
                let results = index.search("name:report");
                std::hint::black_box(results.len());
            });
        });

        group.bench_function(BenchmarkId::new("boolean_search", label), |b| {
            b.iter(|| {
                let results = index.search("report AND pdf");
                std::hint::black_box(results.len());
            });
        });

        group.bench_function(BenchmarkId::new("phrase_search", label), |b| {
            b.iter(|| {
                let results = index.search("\"spreadsheet\" OR \"presentation\"");
                std::hint::black_box(results.len());
            });
        });

        group.bench_function(BenchmarkId::new("paginated_search", label), |b| {
            b.iter(|| {
                let (results, _metrics) = index.search_paginated("report", 0, 10);
                std::hint::black_box(results.len());
            });
        });
    }
    group.finish();
}

fn bench_search_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("owncloud_search_latency");

    let index = SearchIndex::new(vec![
        "name".to_string(),
        "path".to_string(),
        "content_type".to_string(),
    ]);

    for i in 0..10_000 {
        let name = format!("file_{}.txt", i);
        let path = format!("/owncloud/files/{}", name);
        let doc = Document {
            id: format!("latency-doc-{}", i),
            fields: {
                let mut f = HashMap::new();
                f.insert("name".to_string(), name);
                f.insert("path".to_string(), path);
                f.insert("content_type".to_string(), "text".to_string());
                f
            },
            metadata: HashMap::new(),
        };
        index.add_document(doc).unwrap();
    }

    let queries = vec![
        "file_42",
        "nonexistent_xyz",
        "file_0 AND file_1",
        "file_9999",
    ];

    for query in &queries {
        group.bench_with_input(
            BenchmarkId::new("latency_percentiles", query),
            query,
            |b, q| {
                b.iter_custom(|iters| {
                    let mut latencies: Vec<Duration> = Vec::with_capacity(iters as usize);
                    for _ in 0..iters {
                        let start = Instant::now();
                        let _ = index.search(q);
                        latencies.push(start.elapsed());
                    }
                    latencies.sort();
                    let p50 = percentile(
                        &mut latencies
                            .iter()
                            .map(|d| d.as_nanos() as f64)
                            .collect::<Vec<_>>(),
                        50.0,
                    );
                    let p95 = percentile(
                        &mut latencies
                            .iter()
                            .map(|d| d.as_nanos() as f64)
                            .collect::<Vec<_>>(),
                        95.0,
                    );
                    let p99 = percentile(
                        &mut latencies
                            .iter()
                            .map(|d| d.as_nanos() as f64)
                            .collect::<Vec<_>>(),
                        99.0,
                    );
                    let mean_ns = latencies.iter().map(|d| d.as_nanos() as f64).sum::<f64>()
                        / latencies.len() as f64;
                    std::hint::black_box((p50, p95, p99, mean_ns));
                    Duration::from_nanos(mean_ns as u64)
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
    bench_search,
    bench_search_latency,
    bench_sync_workflow,
);
criterion_main!(benches);
