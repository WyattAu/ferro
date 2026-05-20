use criterion::{Criterion, criterion_group, criterion_main};
use ferro_crypto::CryptoProvider;

fn bench_password_hash(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let provider = ferro_crypto::ring_provider::RingProvider::new();

    c.bench_function("password_hash", |b| {
        b.to_async(&rt).iter(|| {
            let provider = &provider;
            async move {
                provider.hash_password("test_password").await.unwrap();
            }
        })
    });
}

fn bench_password_verify(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let provider = ferro_crypto::ring_provider::RingProvider::new();

    let hashed = rt.block_on(async { provider.hash_password("test_password").await.unwrap() });

    c.bench_function("password_verify", |b| {
        let provider = &provider;
        let hashed = &hashed;
        b.to_async(&rt).iter(|| async move {
            provider
                .verify_password("test_password", hashed)
                .await
                .unwrap();
        })
    });
}

fn bench_hmac_sha256(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let provider = ferro_crypto::ring_provider::RingProvider::new();
    let key = b"test-secret-key-for-hmac-sha256-benchmarking";
    let data = b"this is the data to sign with hmac sha256 for benchmarking purposes";

    c.bench_function("hmac_sha256_sign", |b| {
        let provider = &provider;
        b.to_async(&rt).iter(|| async move {
            provider.hmac_sha256(key, data).await.unwrap();
        })
    });
}

fn bench_sha256(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let provider = ferro_crypto::ring_provider::RingProvider::new();
    let data = b"this is the data to hash with sha256 for benchmarking purposes";

    c.bench_function("sha256", |b| {
        let provider = &provider;
        b.to_async(&rt).iter(|| async move {
            provider.sha256(data).await.unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_password_hash,
    bench_password_verify,
    bench_hmac_sha256,
    bench_sha256,
);
criterion_main!(benches);
