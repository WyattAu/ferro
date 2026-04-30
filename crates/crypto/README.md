# ferro-crypto

[![crates.io](https://img.shields.io/crates/v/ferro-crypto.svg)](https://crates.io/crates/ferro-crypto)
[![docs.rs](https://docs.rs/ferro-crypto/badge.svg)](https://docs.rs/ferro-crypto)
[![license](https://img.shields.io/badge/license-AGPL-3.0-blue.svg)](LICENSE)

Standalone cryptographic primitives for the Ferro platform. Provides a `CryptoProvider` trait with a Ring-based implementation for hashing, HMAC, password hashing, secure random generation, and constant-time comparisons.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `ring` | yes | Ring-based `CryptoProvider` implementation with bcrypt and base64 |
| `fips` | no | Enables FIPS-approved mode (implies `ring`) |

## Key Types

- **`CryptoProvider`** — async trait abstracting cryptographic operations
- **`RingProvider`** — production implementation backed by the Ring cryptography library
- **`CryptoError`** — error type for cryptographic operations

### CryptoProvider trait methods

| Method | Description |
|--------|-------------|
| `sha256(data)` | SHA-256 hash |
| `sha512(data)` | SHA-512 hash |
| `hmac_sha256(key, data)` | HMAC-SHA256 message authentication code |
| `random_bytes(len)` | Cryptographically secure random bytes |
| `hash_password(password)` | Bcrypt password hash |
| `verify_password(password, hash)` | Bcrypt password verification |
| `generate_token(len)` | URL-safe, no-pad base64 token |
| `constant_time_eq(a, b)` | Constant-time byte comparison |
| `provider_name()` | Provider identifier string |
| `is_fips_approved()` | Whether FIPS mode is active |

## Usage

### Hash a password and verify it

```rust
use ferro_crypto::{CryptoProvider, ring_provider::RingProvider};

let provider = RingProvider::new();

let hash = provider.hash_password("s3cret").await?;
assert!(provider.verify_password("s3cret", &hash).await?);
assert!(!provider.verify_password("wrong", &hash).await?);
```

### Compute HMAC-SHA256

```rust
use ferro_crypto::{CryptoProvider, ring_provider::RingProvider};

let provider = RingProvider::new();
let mac = provider.hmac_sha256(b"secret-key", b"message data").await?;
assert_eq!(mac.len(), 32);
```

### Generate a secure token

```rust
use ferro_crypto::{CryptoProvider, ring_provider::RingProvider};

let provider = RingProvider::new();
let token = provider.generate_token(32).await?;
// token is URL-safe base64, no padding
```

### Constant-time comparison

```rust
use ferro_crypto::ring_provider::RingProvider;

assert!(RingProvider::constant_time_eq(b"same", b"same"));
assert!(!RingProvider::constant_time_eq(b"a", b"b"));
```

## License

Licensed under AGPL-3.0-or-later.
