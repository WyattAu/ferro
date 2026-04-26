use bytes::Bytes;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tokio::runtime::Runtime;

mod helpers;
use helpers::*;

fn bench_request_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("request_latency");

    let sizes: &[usize] = &[1024, 1024 * 1024, 10 * 1024 * 1024];
    let size_labels: &[&str] = &["1KB", "1MB", "10MB"];

    for (size, label) in sizes.iter().zip(size_labels.iter()) {
        let body = generate_test_body(*size);

        // PUT latency
        group.bench_with_input(
            BenchmarkId::new("put", label),
            size,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let state = create_test_app_state();
                        let app = create_test_router(state);
                        make_request(&app, "PUT", "/bench.txt", body.clone()).await;
                    })
                })
            },
        );

        // GET latency (file must exist)
        group.bench_with_input(
            BenchmarkId::new("get", label),
            size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let state = create_test_app_state();
                        create_test_file(&state, "/bench.txt", size).await;
                        let app = create_test_router(state);
                        make_request(&app, "GET", "/bench.txt", Bytes::new()).await;
                    })
                })
            },
        );

        // DELETE latency (file must exist)
        group.bench_with_input(
            BenchmarkId::new("delete", label),
            size,
            |b, &size| {
                b.iter(|| {
                    rt.block_on(async {
                        let state = create_test_app_state();
                        create_test_file(&state, "/bench.txt", size).await;
                        let app = create_test_router(state);
                        make_request(&app, "DELETE", "/bench.txt", Bytes::new()).await;
                    })
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_request_latency);
criterion_main!(benches);
