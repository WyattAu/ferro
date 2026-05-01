use criterion::{Criterion, criterion_group, criterion_main};

fn bench_path_normalization(c: &mut Criterion) {
    c.bench_function("normalize_simple_path", |b| {
        b.iter(|| {
            let _ = ferro_common::path::normalize_path("/Documents/Photos/2024");
        })
    });

    c.bench_function("normalize_traversal_path", |b| {
        b.iter(|| {
            let _ = ferro_common::path::normalize_path("/../../../etc/passwd");
        })
    });
}

fn bench_metadata_json(c: &mut Criterion) {
    use ferro_common::metadata::{ContentHash, FileMetadata};

    let meta = FileMetadata::new(
        "test.txt".to_string(),
        ContentHash::compute(&[0u8; 1024]),
        1024,
        "bench".to_string(),
    );

    c.bench_function("metadata_serialize", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(&meta);
        })
    });

    let json = serde_json::to_string(&meta).unwrap();
    c.bench_function("metadata_deserialize", |b| {
        b.iter(|| {
            let _: FileMetadata = serde_json::from_str(&json).unwrap();
        })
    });
}

fn bench_error_creation(c: &mut Criterion) {
    use ferro_common::error::FerroError;

    c.bench_function("ferro_error_not_found", |b| {
        b.iter(|| {
            let _ = FerroError::NotFound("/test/path".to_string());
        })
    });
}

criterion_group!(
    benches,
    bench_path_normalization,
    bench_metadata_json,
    bench_error_creation,
);
criterion_main!(benches);
