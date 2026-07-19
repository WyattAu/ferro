# Performance Measurement Report

**Date:** 2026-07-16 | **Tool:** Criterion | **Iterations:** 100 per benchmark

---

## Storage Operations (DashMap, lock-free)

| Operation | Latency (p50) | Latency (p95) | Complexity | HFT Target |
|-----------|---------------|---------------|------------|------------|
| get (10KB) | **286 ns** | 334 ns | O(1) | < 1 µs |
| exists/hit | **321 ns** | 574 ns | O(1) | < 1 µs |
| exists/miss | **209 ns** | 251 ns | O(1) | < 1 µs |
| head | **582 ns** | 716 ns | O(1) | < 1 µs |
| delete | **5.3 µs** | 7.0 µs | O(1) | < 10 µs |
| put (1KB) | **8.6 µs** | 9.3 µs | O(n) SHA-256 | < 10 µs |
| put (10KB) | **64 µs** | 67 µs | O(n) SHA-256 | < 100 µs |
| put (100KB) | **621 µs** | 657 µs | O(n) SHA-256 | < 1 ms |
| list (100 files) | **87 µs** | 95 µs | O(n) scan | < 100 µs |

**Assessment:** All hot-path operations (get, exists, head) are sub-microsecond. The lock-free DashMap conversion is confirmed effective. Put operations are dominated by SHA-256 hashing (~6µs for 1KB), which is expected.

---

## Cryptographic Operations

| Operation | Latency (p50) | Notes |
|-----------|---------------|-------|
| password_hash (bcrypt) | **525 ms** | Intentionally slow (cost factor 12) |
| password_verify (bcrypt) | **667 ms** | Intentionally slow |
| hmac_sha256_sign | **56 µs** | Including key setup |
| sha256 (hash) | **45 µs** | Hardware-accelerated on x86_64 |
| content_hash_1KB | **7.3 µs** | SHA-256 of 1KB data |
| content_hash_1MB | **5.2 ms** | SHA-256 of 1MB data |

**Assessment:** Cryptographic operations are performant. SHA-256 is hardware-accelerated. Bcrypt is intentionally slow for security.

---

## DAV Protocol Operations

| Operation | Size | Latency (p50) |
|-----------|------|---------------|
| iCal parse | Small | **5.7 µs** |
| iCal parse | Medium | **29 µs** |
| iCal parse | Large | **50 µs** |
| iCal serialize | Small | **4.4 µs** |
| iCal serialize | Medium | **16.5 µs** |
| iCal serialize | Large | **39.8 µs** |
| vCard parse | Small | **3.6 µs** |
| vCard parse | Medium | **11.2 µs** |
| vCard parse | Large | **21.1 µs** |
| vCard serialize | Small | **2.4 µs** |
| vCard serialize | Medium | **6.8 µs** |
| vCard serialize | Large | **11.8 µs** |
| XML escape (1KB) | | **324 ns** |

**Assessment:** DAV operations are fast. iCal parsing is the slowest at 50µs for large files. vCard operations are faster than iCal due to simpler structure.

---

## Auth/TOTP Operations

| Operation | Latency (p50) |
|-----------|---------------|
| TOTP generate (SHA-1) | **1.2 µs** |
| TOTP generate (SHA-256) | **2.4 µs** |
| TOTP verify | **1.7 µs** |
| TOTP verify (skew=2) | **1.9 µs** |
| TOTP secret generate | **79 ns** |
| TOTP base32 encode | **275 ns** |
| TOTP base32 decode | **591 ns** |

**Assessment:** TOTP operations are extremely fast. Secret generation is sub-100ns. Verification is under 2µs.

---

## Summary

| Category | Best Operation | Latency | HFT Assessment |
|----------|---------------|---------|----------------|
| Storage get | DashMap O(1) | **286 ns** | Sub-microsecond |
| Storage exists | DashMap O(1) | **209 ns** | Sub-microsecond |
| Storage head | DashMap O(1) | **582 ns** | Sub-microsecond |
| SHA-256 hash | Hardware-accelerated | **45 µs** | Acceptable |
| iCal parse | Parsed in-memory | **5.7 µs** | Fast |
| TOTP verify | HMAC-SHA1 | **1.7 µs** | Very fast |
| Password hash | bcrypt cost=12 | **525 ms** | Intentionally slow |

**Key Finding:** The lock-free DashMap conversion achieved sub-microsecond latency on all hot-path storage operations (get: 286ns, exists: 209ns, head: 582ns). This meets HFT requirements for storage access.
