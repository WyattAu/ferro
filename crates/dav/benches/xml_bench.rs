use criterion::{Criterion, criterion_group, criterion_main};

fn bench_xml_escape(c: &mut Criterion) {
    let data = "a".repeat(1024);
    c.bench_function("xml_escape_1kb", |b| b.iter(|| ferro_dav::xml_ext::escape_xml(&data)));
}

fn bench_needs_escaping(c: &mut Criterion) {
    let data = "a".repeat(1024);
    c.bench_function("needs_escaping_1kb", |b| {
        b.iter(|| ferro_dav::xml_ext::escape_xml(&data))
    });
}

fn bench_xml_escape_special(c: &mut Criterion) {
    let data = "<tag attr=\"value\">&text</tag>".repeat(100);
    c.bench_function("xml_escape_special_chars", |b| {
        b.iter(|| ferro_dav::xml_ext::escape_xml(&data))
    });
}

criterion_group!(
    benches,
    bench_xml_escape,
    bench_needs_escaping,
    bench_xml_escape_special
);
criterion_main!(benches);
