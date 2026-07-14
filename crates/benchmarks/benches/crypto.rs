use criterion::{Criterion, criterion_group, criterion_main};
use ferro_common::metadata::ContentHash;
use ferro_common::xml_escape::escape_xml;
use std::hint::black_box;

fn benchmark_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");

    group.bench_function("content_hash_1kb", |b| {
        let data = vec![0u8; 1024];
        b.iter(|| {
            black_box(ContentHash::compute(&data));
        });
    });

    group.bench_function("content_hash_1mb", |b| {
        let data = vec![0u8; 1024 * 1024];
        b.iter(|| {
            black_box(ContentHash::compute(&data));
        });
    });

    group.finish();
}

fn benchmark_xml_escape(c: &mut Criterion) {
    let mut group = c.benchmark_group("xml_escape");

    group.bench_function("escape_plain", |b| {
        let input = "Hello World";
        b.iter(|| {
            black_box(escape_xml(input));
        });
    });

    group.bench_function("escape_special", |b| {
        let input = "<script>alert('xss')</script>";
        b.iter(|| {
            black_box(escape_xml(input));
        });
    });

    group.bench_function("escape_mixed", |b| {
        let input = "Tom & Jerry <played> in the \"garden\" with 'friends'";
        b.iter(|| {
            black_box(escape_xml(input));
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_hash, benchmark_xml_escape);
criterion_main!(benches);
