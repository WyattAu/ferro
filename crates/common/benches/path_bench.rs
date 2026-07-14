use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ferro_common::path::normalize_path;

fn bench_normalize_short(c: &mut Criterion) {
    c.bench_function("normalize_path/short", |b| b.iter(|| normalize_path("/a/b")));
}

fn bench_normalize_medium(c: &mut Criterion) {
    c.bench_function("normalize_path/medium", |b| {
        b.iter(|| normalize_path("/foo/bar/baz/qux/file.txt"))
    });
}

fn bench_normalize_long(c: &mut Criterion) {
    let path = "/".to_string() + &"segment/".repeat(50) + "file.txt";
    c.bench_function("normalize_path/long", |b| b.iter(|| normalize_path(&path)));
}

fn bench_normalize_deep_nesting(c: &mut Criterion) {
    let path = "/a".repeat(30) + &"/../b".repeat(29);
    c.bench_function("normalize_path/deep_nesting", |b| b.iter(|| normalize_path(&path)));
}

fn bench_normalize_with_dotdot(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize_path/traversal");
    for depth in [5, 20, 100] {
        let prefix = "/".to_string() + &"x/".repeat(depth);
        let path = format!("{}../../../target", prefix);
        group.bench_with_input(BenchmarkId::from_parameter(depth), &path, |b, path| {
            b.iter(|| normalize_path(path))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_normalize_short,
    bench_normalize_medium,
    bench_normalize_long,
    bench_normalize_deep_nesting,
    bench_normalize_with_dotdot,
);
criterion_main!(benches);
