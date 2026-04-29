use bytes::Bytes;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;

mod helpers;
use helpers::*;

fn bench_webdav_ops(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("webdav_ops");

    // MKCOL benchmark
    group.bench_function("mkcol", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state = create_test_app_state();
                let app = create_test_router(state);
                make_request(&app, "MKCOL", "/bench_dir", Bytes::new()).await;
            })
        })
    });

    // PROPFIND with different item counts
    for &item_count in &[10usize, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::new("propfind", item_count),
            &item_count,
            |b, &item_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let state = create_test_app_state();
                        let app = create_test_router(state);

                        // Create the collection
                        make_request(&app, "MKCOL", "/bench_dir", Bytes::new()).await;

                        // Populate with items
                        for i in 0..item_count {
                            let body = generate_test_body(64);
                            make_request(&app, "PUT", &format!("/bench_dir/file_{}.txt", i), body)
                                .await;
                        }

                        // Benchmark the PROPFIND
                        make_request(&app, "PROPFIND", "/bench_dir", Bytes::new()).await;
                    })
                })
            },
        );
    }

    // DELETE single file
    group.bench_function("delete/file", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state = create_test_app_state();
                let app = create_test_router(state);
                let body = generate_test_body(1024);
                make_request(&app, "PUT", "/to_delete.txt", body).await;
                make_request(&app, "DELETE", "/to_delete.txt", Bytes::new()).await;
            })
        })
    });

    // DELETE recursive (collection with children)
    for &child_count in &[10usize, 100] {
        group.bench_with_input(
            BenchmarkId::new("delete/recursive", child_count),
            &child_count,
            |b, &child_count| {
                b.iter(|| {
                    rt.block_on(async {
                        let state = create_test_app_state();
                        let app = create_test_router(state);

                        // Create collection and children
                        make_request(&app, "MKCOL", "/del_dir", Bytes::new()).await;
                        for i in 0..child_count {
                            let body = generate_test_body(64);
                            make_request(&app, "PUT", &format!("/del_dir/file_{}.txt", i), body)
                                .await;
                        }

                        // Benchmark recursive delete (delete collection + children)
                        for i in 0..child_count {
                            make_request(
                                &app,
                                "DELETE",
                                &format!("/del_dir/file_{}.txt", i),
                                Bytes::new(),
                            )
                            .await;
                        }
                        make_request(&app, "DELETE", "/del_dir", Bytes::new()).await;
                    })
                })
            },
        );
    }

    // PROPFIND depth:0 (single item metadata)
    group.bench_function("propfind/depth0", |b| {
        b.iter(|| {
            rt.block_on(async {
                let state = create_test_app_state();
                let app = create_test_router(state);
                let body = generate_test_body(1024);
                make_request(&app, "PUT", "/single.txt", body).await;
                // Use axum Request builder for the Depth header
                use axum::body::Body;
                use axum::http::Request;
                use tower::ServiceExt;
                let response = app
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("PROPFIND")
                            .uri("/single.txt")
                            .header("Depth", "0")
                            .body(Body::from(Bytes::new()))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                std::hint::black_box(response.status());
            })
        })
    });

    group.finish();
}

criterion_group!(benches, bench_webdav_ops);
criterion_main!(benches);
