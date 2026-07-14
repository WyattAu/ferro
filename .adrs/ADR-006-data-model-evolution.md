# ADR-006: Data Model Evolution

**Status:** Accepted
**Date:** 2026-07-12
**Deciders:** Wyatt (Sole developer)

## Context

Ferro persists data in SQLite (unified via `--data-dir`) with WAL mode, supporting metadata, CAS dedup storage, snapshots, audit logs, comments, retention policies, WORM policies, and WASM event triggers. The migration directory currently contains 8 sequential SQL migrations (`001_initial_schema.sql` through `005_comments.sql` and beyond). The project also supports PostgreSQL as an optional backend.

As the schema evolves with new features (comments v5, event triggers v6, WORM v7, remote mounts v8), there is a need for a formal migration strategy, schema versioning scheme, and backward compatibility policy to prevent data loss during upgrades and enable safe rollbacks.

## Decision

### Migration Strategy

Ferro uses **SQLx migrate!() macro** for compile-time verified migrations, with sequential numbered SQL files in `crates/server/migrations/` (or the project root `migrations/` directory).

**Migration file naming:**
```
migrations/
  001_initial_schema.sql
  002_totp_2fa.sql
  003_extended_features.sql
  004_retention_policies_v2.sql
  005_comments.sql
  006_event_triggers.sql
  007_worm_mode.sql
  008_remote_mounts.sql
  ...
```

**Migration rules:**
1. Migrations are **append-only** -- never modify or delete an existing migration file
2. Each migration is a single SQL file executed in sequence
3. Migrations must be **idempotent-safe** (use `IF NOT EXISTS`, `IF EXISTS` for DDL)
4. Migrations run **automatically on startup** before accepting requests
5. Migrations are **forward-only** -- no down migrations (see rollback strategy below)

**Startup sequence:**
```rust
// crates/server/src/main.rs (simplified)
sqlx::migrate!("migrations")
    .run(&pool)
    .await?;
// Only then start accepting connections
```

### Schema Versioning

Track schema version in a dedicated table:

```sql
CREATE TABLE IF NOT EXISTS _ferro_schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT (datetime('now')),
    description TEXT
);
```

The current schema version is queryable via:
- SQL: `SELECT MAX(version) FROM _ferro_schema_version;`
- API: `GET /api/health` (includes `schema_version` in response)
- CLI: `ferro-server --version` (shows schema version)

**Versioning scheme:**
- Schema version increments by 1 for each migration
- Schema version is **independent** of the application semantic version
- Example: Ferro v3.1.0 may run schema version 8

### Backward Compatibility

**SQLite WAL mode considerations:**
- WAL mode allows concurrent reads during writes
- Readers don't block writers; writers don't block readers
- Schema changes (DDL) acquire an exclusive lock momentarily

**Backward compatibility policy:**

| Change Type | Allowed? | Notes |
|-------------|----------|-------|
| Add table | Yes | Non-breaking, additive |
| Add column (nullable or with default) | Yes | Existing rows get default value |
| Add NOT NULL column without default | No | Breaks existing inserts |
| Drop column | No | Breaking; must use deprecation cycle (ADR-003) |
| Rename column | No | Breaking; use ADD + migrate data + DROP pattern |
| Add index | Yes | Non-breaking, additive |
| Drop index | Yes | Performance may degrade, but not breaking |
| Change column type | No | Breaking; must maintain old type during transition |
| Add constraint | Only if existing data satisfies it | Validate before applying |

**Safe column migration pattern (rename):**
```sql
-- Migration N: Add new column
ALTER TABLE users ADD COLUMN display_name TEXT;

-- Migration N+1: Backfill data
UPDATE users SET display_name = username WHERE display_name IS NULL;

-- Migration N+2 (after deprecation cycle): Drop old column
-- Only after ADR-003 deprecation window expires
ALTER TABLE users DROP COLUMN username;
```

### Rollback Strategy

Since migrations are forward-only, rollback is handled at the **application level**, not the database level:

1. **Pre-migration backup**: Before running migrations, copy the SQLite database file
   ```bash
   cp data/ferro.db data/ferro.db.bak.$(date +%s)
   ```
2. **Application-level rollback**: If migration fails, restore from backup and run the previous Ferro version
3. **No down migrations**: Writing reverse migrations is error-prone and rarely tested; backup + restore is simpler and more reliable
4. **Backup rotation**: Keep last 3 backups, auto-purge older ones

**On startup:**
```
[INFO] Current schema version: 7
[INFO] Running migrations...
[INFO] Applied migration 008_remote_mounts.sql
[INFO] Schema version: 8
[INFO] Ready to accept requests
```

**On migration failure:**
```
[ERROR] Migration 008_remote_mounts.sql failed: column "mount_url" already exists
[ERROR] Database backup saved to: data/ferro.db.bak.1720800000
[ERROR] To rollback: restore backup and run previous Ferro version
[ERROR] Aborting startup
```

### Multi-Backend Support

| Backend | Migration Method | Notes |
|---------|-----------------|-------|
| SQLite | SQLx migrate!() | Primary, unified (metadata + CAS + audit + snapshots) |
| PostgreSQL | SQLx migrate!() | Optional, feature-gated (`pg` feature) |

Migrations are written in **portable SQL** (SQLite and PostgreSQL compatible). When a backend-specific migration is required, use conditional SQL:

```sql
-- SQLite
ALTER TABLE foo ADD COLUMN bar TEXT DEFAULT 'baz';
-- PostgreSQL equivalent (handled by SQLx dialect detection)
```

### CAS (Content-Addressable Storage) Evolution

CAS data is addressed by SHA-256 hash, making it inherently versioned:
- Adding new metadata fields: new sidecar `.meta` files (non-breaking)
- Changing hash algorithm: requires a full re-index (major version event)
- CAS layout on disk: `{data_dir}/cas/{hash[0:2]}/{hash[2:4]}/{hash}`

CAS evolution is independent of SQL schema evolution because content is addressed by hash, not by row ID.

## Consequences

### Positive
- Compile-time migration verification via SQLx (catches SQL errors before runtime)
- Forward-only migrations eliminate complex rollback logic
- Pre-migration backup provides simple, reliable rollback path
- Schema version in health check enables monitoring of migration status
- WAL mode allows zero-downtime migrations for read-heavy workloads

### Negative
- Forward-only means broken migrations require manual backup restoration
- No down migrations means schema cannot be easily reverted
- SQLite DDL acquires exclusive lock momentarily (brief write stall during migration)
- Multi-backend SQL compatibility adds testing burden

### Risks
- A malformed migration could corrupt data; backup is the only safety net
- Long migrations on large databases could cause extended startup time
- Schema version drift between nodes in a hypothetical cluster (not yet applicable, but future consideration)
- PostgreSQL and SQLite DQL differences could cause migration divergence

## Alternatives Considered

### Diesel Migrations
- **Description:** Use Diesel's migration system with up/down migrations
- **Pros:** Built-in rollback (down migrations), type-safe queries, schema diffing
- **Cons:** Diesel is a full ORM -- heavier than SQLx's query-only approach; adds compile-time overhead; doesn't match existing SQLx usage
- **Why Rejected:** Project already uses SQLx for async database access; switching to Diesel would require rewriting all database code

### Schemaless/JSON Storage
- **Description:** Store all data as JSON documents (e.g., SQLite JSON1 extension)
- **Pros:** No migration needed, flexible schema
- **Cons:** No type safety, no foreign keys, poor query performance, loses relational benefits
- **Why Rejected:** Ferro uses structured relational data (users, files, shares, locks); schemaless defeats the purpose of a database

### Versioned Schema with Migration Runner
- **Description:** Build a custom migration runner with version tracking, up/down support, and dependency resolution
- **Pros:** Full control over migration lifecycle, rollback support
- **Cons:** Significant engineering effort; SQLx migrate!() already provides most of this; overengineered for solo-developer project
- **Why Rejected:** SQLx migrate!() covers 90% of the use case; custom runner is unnecessary complexity

## Related ADRs
- [ADR-002](ADR-002-deprecation-policy.md) -- Deprecation Policy (column renames/removals must follow deprecation cycle)
- [ADR-005](ADR-005-concurrency-model.md) -- Concurrency Model (WAL mode, SQLite single-writer considerations)

## References
- SQLx migrations: https://docs.rs/sqlx/latest/sqlx/macro.migrate.html
- SQLite WAL mode: https://www.sqlite.org/wal.html
- Ferro migrations directory: `migrations/`
- Schema versions in use: v8 (001 through 008)
- Unified SQLite persistence: `--data-dir` flag (Sprint L)
