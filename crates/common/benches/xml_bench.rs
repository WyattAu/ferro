use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ferro_common::xml_escape::escape_xml;

fn bench_escape_plain(c: &mut Criterion) {
    c.bench_function("escape_xml/plain", |b| b.iter(|| escape_xml("hello world")));
}

fn bench_escape_special_chars(c: &mut Criterion) {
    c.bench_function("escape_xml/special_chars", |b| {
        b.iter(|| escape_xml("<tag attr=\"val\">&'text'</tag>"))
    });
}

fn bench_escape_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("escape_xml/sizes");
    for &size in &[64usize, 1024, 8192, 65536] {
        let input: String = "<tag>content &amp; data</tag>\n".repeat(size / 30);
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| escape_xml(input))
        });
    }
    group.finish();
}

fn bench_escape_all_special(c: &mut Criterion) {
    let input: String = "&<>\"'".repeat(500);
    c.bench_function("escape_xml/all_special_chars", |b| b.iter(|| escape_xml(&input)));
}

criterion_group!(
    benches,
    bench_escape_plain,
    bench_escape_special_chars,
    bench_escape_sizes,
    bench_escape_all_special,
);
criterion_main!(benches);
