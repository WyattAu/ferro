use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ferro_auth::totp;

fn bench_totp_generate(c: &mut Criterion) {
    let secret = totp::generate_secret();
    let timestamp = 1_700_000_000u64;

    c.bench_function("totp_generate_sha1", |b| {
        b.iter(|| totp::generate_totp(&secret, timestamp, 6, 30, 0))
    });
}

fn bench_totp_generate_sha256(c: &mut Criterion) {
    let secret = totp::generate_secret();
    let timestamp = 1_700_000_000u64;

    c.bench_function("totp_generate_sha256", |b| {
        b.iter(|| totp::generate_totp_sha256(&secret, 0, timestamp).unwrap())
    });
}

fn bench_totp_verify(c: &mut Criterion) {
    let secret = totp::generate_secret();
    let timestamp = 1_700_000_000u64;
    let code = totp::generate_totp(&secret, timestamp, 6, 30, 0);

    c.bench_function("totp_verify", |b| {
        b.iter(|| totp::verify_totp(&secret, code, timestamp, 6, 30, 0, 1))
    });
}

fn bench_totp_verify_with_skew(c: &mut Criterion) {
    let secret = totp::generate_secret();
    let timestamp = 1_700_000_000u64;
    let code = totp::generate_totp(&secret, timestamp, 6, 30, 0);

    c.bench_function("totp_verify_skew_2", |b| {
        b.iter(|| totp::verify_totp(&secret, code, timestamp, 6, 30, 0, 2))
    });
}

fn bench_totp_secret_generation(c: &mut Criterion) {
    c.bench_function("totp_secret_generate", |b| b.iter(totp::generate_secret));
}

fn bench_totp_base32_encode(c: &mut Criterion) {
    let secret = totp::generate_secret();

    c.bench_function("totp_base32_encode", |b| b.iter(|| totp::encode_secret_base32(&secret)));
}

fn bench_totp_base32_decode(c: &mut Criterion) {
    let secret = totp::generate_secret();
    let encoded = totp::encode_secret_base32(&secret);

    c.bench_function("totp_base32_decode", |b| {
        b.iter(|| totp::decode_secret_base32(&encoded))
    });
}

fn bench_totp_otpauth_uri(c: &mut Criterion) {
    let secret = totp::generate_secret();
    let encoded = totp::encode_secret_base32(&secret);

    c.bench_function("totp_otpauth_uri", |b| {
        b.iter(|| totp::generate_otpauth_uri("Ferro", "user@example.com", &encoded, 6, 30))
    });
}

fn bench_totp_secret_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("totp_secret_sizes");
    for &size in &[16usize, 20, 32, 64] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let mut secret = vec![0u8; size];
                rand::RngCore::fill_bytes(&mut rand::rng(), &mut secret);
                totp::encode_secret_base32(&secret)
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_totp_generate,
    bench_totp_generate_sha256,
    bench_totp_verify,
    bench_totp_verify_with_skew,
    bench_totp_secret_generation,
    bench_totp_base32_encode,
    bench_totp_base32_decode,
    bench_totp_otpauth_uri,
    bench_totp_secret_sizes,
);
criterion_main!(benches);
