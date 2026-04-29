use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

mod helpers;
use helpers::*;

fn bench_upload_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("upload_throughput");

    let sizes: &[usize] = &[1024, 1024 * 1024, 10 * 1024 * 1024];
    let size_labels: &[&str] = &["1KB", "1MB", "10MB"];

    for (size, label) in sizes.iter().zip(size_labels.iter()) {
        let body = generate_test_body(*size);

        // Sequential upload
        group.bench_with_input(BenchmarkId::new("sequential", label), size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let state = create_test_app_state();
                    let app = create_test_router(state);
                    make_request(&app, "PUT", "/bench.txt", body.clone()).await;
                })
            })
        });

        // Concurrent uploads (1KB and 1MB only — 10MB concurrent would be too slow)
        if *size <= 1024 * 1024 {
            for &clients in &[10usize, 50, 100] {
                let bench_id = BenchmarkId::new(format!("concurrent_{}clients", clients), label);
                group.bench_with_input(bench_id, &(size, clients), |b, _| {
                    b.iter(|| {
                        rt.block_on(async {
                            let state = create_test_app_state();
                            let app = create_test_router(state);
                            let mut handles = Vec::with_capacity(clients);
                            for i in 0..clients {
                                let app_clone = app.clone();
                                let req_body = body.clone();
                                handles.push(tokio::spawn(async move {
                                    make_request(
                                        &app_clone,
                                        "PUT",
                                        &format!("/bench_{}.txt", i),
                                        req_body,
                                    )
                                    .await;
                                }));
                            }
                            for h in handles {
                                h.await.unwrap();
                            }
                        })
                    })
                });
            }
        }
    }

    group.finish();
}

fn bench_download_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("download_throughput");

    let sizes: &[usize] = &[1024, 1024 * 1024, 10 * 1024 * 1024];
    let size_labels: &[&str] = &["1KB", "1MB", "10MB"];

    for (size, label) in sizes.iter().zip(size_labels.iter()) {
        // Sequential download
        group.bench_with_input(BenchmarkId::new("sequential", label), size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let state = create_test_app_state();
                    create_test_file(&state, "/bench.txt", size).await;
                    let app = create_test_router(state);
                    make_request(&app, "GET", "/bench.txt", Bytes::new()).await;
                })
            })
        });

        // Concurrent downloads (1KB and 1MB only)
        if *size <= 1024 * 1024 {
            for &clients in &[10usize, 50, 100] {
                let bench_id = BenchmarkId::new(format!("concurrent_{}clients", clients), label);
                group.bench_with_input(bench_id, &(size, clients), |b, &(size, clients)| {
                    b.iter(|| {
                        rt.block_on(async {
                            let state = create_test_app_state();
                            for i in 0..clients {
                                create_test_file(&state, &format!("/bench_{}.txt", i), *size).await;
                            }
                            let app = create_test_router(state);
                            let mut handles = Vec::with_capacity(clients);
                            for i in 0..clients {
                                let app_clone = app.clone();
                                handles.push(tokio::spawn(async move {
                                    make_request(
                                        &app_clone,
                                        "GET",
                                        &format!("/bench_{}.txt", i),
                                        Bytes::new(),
                                    )
                                    .await;
                                }));
                            }
                            for h in handles {
                                h.await.unwrap();
                            }
                        })
                    })
                });
            }
        }
    }

    group.finish();
}

criterion_group!(benches, bench_upload_throughput, bench_download_throughput);
criterion_main!(benches);
