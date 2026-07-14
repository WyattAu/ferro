use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ferro_common::simd::bulk::{memchr_simd, memcpy_simd, memset_simd};
use ferro_common::simd::checksum::crc32_simd;
use ferro_common::simd::compare::{contains_simd, strcmp_simd};

fn bench_crc32(c: &mut Criterion) {
    let mut group = c.benchmark_group("crc32");
    for &size in &[64usize, 256, 1024, 4096, 65536] {
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| crc32_simd(data))
        });
    }
    group.finish();
}

fn bench_strcmp(c: &mut Criterion) {
    let mut group = c.benchmark_group("strcmp");
    for &size in &[32usize, 256, 1024, 4096] {
        let a: String = "a".repeat(size);
        let b: String = "a".repeat(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &(&a, &b), |bench, (a, b)| {
            bench.iter(|| strcmp_simd(a, b))
        });
    }
    group.finish();
}

fn bench_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("contains");
    for &size in &[64usize, 256, 1024, 4096] {
        let haystack: String = "hello world ".repeat(size / 12);
        let needle = "world";
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(&haystack, needle),
            |bench, (haystack, needle)| bench.iter(|| contains_simd(haystack, needle)),
        );
    }
    group.finish();
}

fn bench_memcpy(c: &mut Criterion) {
    let mut group = c.benchmark_group("memcpy");
    for &size in &[64usize, 256, 1024, 4096, 65536] {
        let src: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        let mut dst = vec![0u8; size];
        group.bench_with_input(BenchmarkId::from_parameter(size), &src, |b, src| {
            b.iter(|| {
                memcpy_simd(&mut dst, src);
                dst[0]
            })
        });
    }
    group.finish();
}

fn bench_memset(c: &mut Criterion) {
    let mut group = c.benchmark_group("memset");
    for &size in &[64usize, 256, 1024, 4096, 65536] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let mut buf = vec![0u8; size];
                memset_simd(&mut buf, 0xFF);
                buf[0]
            })
        });
    }
    group.finish();
}

fn bench_memchr(c: &mut Criterion) {
    let mut group = c.benchmark_group("memchr");
    for &size in &[64usize, 256, 1024, 4096, 65536] {
        let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.iter(|| memchr_simd(data, 128))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_crc32,
    bench_strcmp,
    bench_contains,
    bench_memcpy,
    bench_memset,
    bench_memchr,
);
criterion_main!(benches);
