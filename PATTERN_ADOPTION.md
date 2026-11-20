# PATTERN_ADOPTION

Ferro's declared relationship to the WyattAu pattern canon
(`pattern-standards`, references/catalog.md). Machine-checked by
`pattern-standards/scripts/adoption-check.py`.

| Pattern | Status | Evidence |
|---|---|---|
| T23 | adopted | crates/server-collaboration/src/collab_ws.rs:5 (per-room crdts-kit `CrdtDocument` behind `RwLock`), crates/server/src/integration/crdt_integration.rs:24 (`apply_remote_ops` converges replicas via apply_ops, no coordination locks) |
| P51 | adopted | crates/circuit-breaker/src/lib.rs:1 (estate `breaker` kit surface: `CircuitBreaker`, `CircuitState`, metrics), crates/server-infra/src/circuit_breaker.rs:1 (per-dependency breaker used by server infra) |
| T13 | partial | crates/core/src/storage.rs:1 (`StorageEngine` trait consumed by the Axum shell, engines swappable); persistence and sqlx still sit inside core crates (crates/core/src/persistence.rs) |
