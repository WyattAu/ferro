# Structured Logging Standard

**Document:** Logging Standard  
**Version:** 1.0.0  
**Status:** Active  
**Last Updated:** 2026-07-12  

---

## Overview

All production log output MUST use structured logging via the `tracing` crate with JSON format in production environments and pretty format in development.

---

## Log Event Schema

Every log event MUST include the following fields:

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `timestamp` | ISO 8601 | Yes | Event timestamp |
| `level` | string | Yes | TRACE, DEBUG, INFO, WARN, ERROR |
| `module` | string | Yes | Source module path |
| `message` | string | Yes | Human-readable message |
| `request_id` | UUID | No | X-Request-ID (if in request context) |
| `tenant_id` | string | No | Tenant identifier (if in tenant context) |
| `user_id` | string | No | Authenticated user ID |
| `span` | string | Yes | Active tracing span |

### Example (JSON)

```json
{
  "timestamp": "2026-07-12T10:30:00.123Z",
  "level": "INFO",
  "module": "ferro_server::handlers",
  "message": "File uploaded",
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "admin",
  "path": "/documents/report.pdf",
  "size_bytes": 1048576,
  "duration_ms": 45
}
```

---

## Sensitive Field Redaction

The following fields MUST be automatically redacted in all log output:

| Field Pattern | Redaction | Reason |
|---------------|-----------|--------|
| `password`, `*password*` | `[REDACTED]` | Credentials |
| `token`, `*token*` | `[REDACTED]` | Auth tokens |
| `secret`, `*secret*` | `[REDACTED]` | Secrets |
| `api_key`, `*api_key*` | `[REDACTED]` | API credentials |
| `authorization` | `[REDACTED]` | Auth header |

### Implementation

Use a custom `tracing_subscriber::Layer` that filters sensitive fields:

```rust
// In server startup
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, Registry};

let redaction_layer = fmt::layer()
    .with_target(true)
    .with_thread_ids(true)
    .json();

let subscriber = Registry::default()
    .with(EnvFilter::from_default_env())
    .with(redaction_layer);

tracing::subscriber::set_global_default(subscriber)
    .expect("Failed to set tracing subscriber");
```

---

## Log Levels

| Level | When to Use | Example |
|-------|-------------|---------|
| TRACE | Verbose debugging, disabled in production | `trace!(key = %key, "Encrypting value")` |
| DEBUG | Development debugging, disabled in production | `debug!(path = %path, "File accessed")` |
| INFO | Normal operations | `info!(user = %user, "Login successful")` |
| WARN | Recoverable errors, degraded state | `warn!(retry_count = 3, "Retrying operation")` |
| ERROR | Unrecoverable errors, requires attention | `error!(error = %e, "Storage failure")` |

---

## Request Context Propagation

All log events within a request handler MUST include the request context:

```rust
// Middleware sets request_id, user_id, tenant_id in tracing span
let request_span = info_span!(
    "request",
    request_id = %request_id,
    method = %method,
    path = %path,
    user_id = tracing::field::Empty,
    tenant_id = tracing::field::Empty,
);

// Handler populates fields
tracing::Span::current().record("user_id", &user_id.as_str());
```

---

## Prohibited Patterns

| Pattern | Reason | Alternative |
|---------|--------|-------------|
| `println!()` | Unstructured, not timestamped | `tracing::info!()` |
| `eprintln!()` | Unstructured, stderr only | `tracing::error!()` |
| `dbg!()` | Debug only, not for production | `tracing::debug!()` |
| `println!("{:?}", secret)` | Secret leakage | `tracing::debug!(secret = "[REDACTED]")` |
| String interpolation in log macros | Performance cost | Use structured fields |

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level filter |
| `FERRO_LOG_FORMAT` | `json` | Output format: `json` or `pretty` |
| `FERRO_LOG_FILE` | none | Optional log file path |

### Production Configuration

```toml
[env]
RUST_LOG = "ferro_server=info,tower_http=info,ferro_auth=info"
FERRO_LOG_FORMAT = "json"
```

### Development Configuration

```toml
[env]
RUST_LOG = "debug,hyper=info,tower=info"
FERRO_LOG_FORMAT = "pretty"
```

---

## Compliance

| Standard | Requirement | Implementation |
|----------|-------------|----------------|
| SOC 2 CC7.2 | Monitor for anomalies | Structured logs with request context |
| SOC 2 CC6.1 | Access logging | All auth events logged with user_id |
| GDPR Art. 30 | Processing records | Audit log with structured fields |
| OWASP ASVS 7.1 | Log content controls | Sensitive field redaction |

---

## Verification

Run the following to verify compliance:

```bash
# No println!/eprintln! in production code
grep -rn "println!\|eprintln!" crates/ --include="*.rs" | grep -v test | grep -v "#\[cfg(test)\]"

# No dbg! in production code
grep -rn "dbg!" crates/ --include="*.rs" | grep -v test

# All crates use tracing
grep -rn "use tracing" crates/*/src/lib.rs crates/*/src/main.rs
```
