use common::auth::Claims;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use jsonwebtoken::{EncodingKey, Header, Validation, decode, encode};
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_claims() -> Claims {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    Claims {
        sub: "user-123".to_string(),
        aud: "ferro".to_string(),
        iss: "https://auth.example.com".to_string(),
        exp: now + 3600,
        iat: now,
        nonce: Some("nonce-abc".to_string()),
        email: Some("user@example.com".to_string()),
        name: Some("Test User".to_string()),
        groups: Some(vec!["admin".to_string(), "editors".to_string()]),
    }
}

fn bench_jwt_encode(c: &mut Criterion) {
    let claims = sample_claims();
    let encoding_key = EncodingKey::from_secret(b"benchmark-secret-key-32-bytes-long!!");

    c.bench_function("jwt_encode", |b| {
        b.iter(|| encode(&Header::default(), &claims, &encoding_key).unwrap())
    });
}

fn bench_jwt_decode(c: &mut Criterion) {
    let claims = sample_claims();
    let encoding_key = EncodingKey::from_secret(b"benchmark-secret-key-32-bytes-long!!");
    let token = encode(&Header::default(), &claims, &encoding_key).unwrap();
    let decoding_key = jsonwebtoken::DecodingKey::from_secret(b"benchmark-secret-key-32-bytes-long!!");

    c.bench_function("jwt_decode", |b| {
        b.iter(|| decode::<Claims>(&token, &decoding_key, &Validation::default()).unwrap())
    });
}

fn bench_jwt_encode_payload_sizes(c: &mut Criterion) {
    let encoding_key = EncodingKey::from_secret(b"benchmark-secret-key-32-bytes-long!!");
    let base_claims = sample_claims();

    let mut group = c.benchmark_group("jwt_encode/payload_sizes");
    for &group_count in &[1usize, 10, 50] {
        let mut claims = base_claims.clone();
        claims.groups = Some(vec!["role".to_string(); group_count]);
        group.bench_with_input(BenchmarkId::from_parameter(group_count), &claims, |b, claims| {
            b.iter(|| encode(&Header::default(), claims, &encoding_key).unwrap())
        });
    }
    group.finish();
}

fn bench_jwt_decode_invalid_token(c: &mut Criterion) {
    let decoding_key = jsonwebtoken::DecodingKey::from_secret(b"benchmark-secret-key-32-bytes-long!!");

    c.bench_function("jwt_decode/invalid_token", |b| {
        b.iter(|| {
            let _ = decode::<Claims>("invalid.token.value", &decoding_key, &Validation::default());
        })
    });
}

fn bench_api_key_hash(c: &mut Criterion) {
    c.bench_function("api_key_hash", |b| {
        b.iter(|| ferro_auth::api_keys::hash_api_key("ferro_abcdef1234567890abcdef1234567890"))
    });
}

fn bench_api_key_hash_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_key_hash/lengths");
    for &size in &[16usize, 32, 64, 128] {
        let key = "ferro_".to_string() + &"a".repeat(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &key, |b, key| {
            b.iter(|| ferro_auth::api_keys::hash_api_key(key))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_jwt_encode,
    bench_jwt_decode,
    bench_jwt_encode_payload_sizes,
    bench_jwt_decode_invalid_token,
    bench_api_key_hash,
    bench_api_key_hash_sizes,
);
criterion_main!(benches);
