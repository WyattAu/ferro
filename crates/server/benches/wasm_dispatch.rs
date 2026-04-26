use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ferro_core::wasm::{WasmWorkerRuntime, WorkerConfig, WorkerEvent};
use tokio::runtime::Runtime;

fn bench_wasm_dispatch(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("wasm_dispatch");

    // Benchmark: WasmWorkerRuntime creation (engine instantiation)
    group.bench_function("runtime_creation", |b| {
        b.iter(|| {
            let runtime = WasmWorkerRuntime::new().unwrap();
            black_box(runtime);
        })
    });

    // Benchmark: worker registration
    group.bench_function("register_worker", |b| {
        let runtime = rt.block_on(async {
            let r = WasmWorkerRuntime::new().unwrap();
            r
        });

        b.iter(|| {
            rt.block_on(async {
                runtime
                    .register_worker(WorkerEvent {
                        pattern: "*.pdf".to_string(),
                        module_path: "/workers/pdf_processor.wasm".to_string(),
                        function_name: "process".to_string(),
                        config: WorkerConfig::default(),
                    })
                    .await;
            })
        })
    });

    // Benchmark: pattern matching with different numbers of registered workers
    for &worker_count in &[1usize, 10, 50, 100] {
        group.bench_with_input(
            criterion::BenchmarkId::new("find_matching", worker_count),
            &worker_count,
            |b, &worker_count| {
                let runtime = rt.block_on(async {
                    let r = WasmWorkerRuntime::new().unwrap();
                    let patterns = ["*.pdf", "*.txt", "*.jpg", "*.png", "*.md"];
                    for i in 0..worker_count {
                        r.register_worker(WorkerEvent {
                            pattern: patterns[i % patterns.len()].to_string(),
                            module_path: format!("/workers/module_{}.wasm", i),
                            function_name: "process".to_string(),
                            config: WorkerConfig::default(),
                        })
                        .await;
                    }
                    r
                });

                b.iter(|| {
                    rt.block_on(async {
                        let matches = runtime
                            .find_matching_workers("/documents/report.pdf")
                            .await;
                        black_box(matches);
                    })
                })
            },
        );
    }

    // Benchmark: dispatch overhead (execute with nonexistent module)
    // Measures the full dispatch path: file lookup → module read failure
    group.bench_function("dispatch_overhead/noop_module", |b| {
        let runtime = rt.block_on(async {
            let r = WasmWorkerRuntime::new().unwrap();
            r.register_worker(WorkerEvent {
                pattern: "*.bench".to_string(),
                module_path: "/nonexistent/module.wasm".to_string(),
                function_name: "process".to_string(),
                config: WorkerConfig {
                    max_time_ms: 1,
                    ..Default::default()
                },
            })
            .await;
            r
        });

        b.iter(|| {
            rt.block_on(async {
                let result = runtime
                    .execute(
                        "/nonexistent/module.wasm",
                        "process",
                        b"benchmark input data",
                        Some(WorkerConfig {
                            max_time_ms: 1,
                            ..Default::default()
                        }),
                    )
                    .await
                    .unwrap();
                black_box(result);
            })
        })
    });

    group.finish();
}

criterion_group!(benches, bench_wasm_dispatch);
criterion_main!(benches);
