# PostgreSQL Migration Guide

Migrate from SQLite to PostgreSQL for deployments with more than 100 concurrent users.

## Prerequisites

- PostgreSQL 14+ running and accessible
- `pg_dump` and `psql` CLI tools installed
- Ferro server stopped during migration

## Step 1: Export SQLite Data

```bash
# Export the SQLite database
sqlite3 /var/lib/ferro/ferro.db .dump > ferro_export.sql

# Or export specific tables
sqlite3 /var/lib/ferro/ferro.db ".schema" > schema.sql
sqlite3 /var/lib/ferro/ferro.db "SELECT * FROM metadata;" > metadata.csv
sqlite3 /var/lib/ferro/ferro.db "SELECT * FROM cas_store;" > cas.csv
```

## Step 2: Create PostgreSQL Database

```sql
CREATE DATABASE ferro;
CREATE USER ferro WITH PASSWORD 'your_password';
GRANT ALL PRIVILEGES ON DATABASE ferro TO ferro;
\c ferro
GRANT ALL ON SCHEMA public TO ferro;
```

## Step 3: Apply Schema

```bash
psql -U ferro -d ferro -f migrations/001_initial_schema.sql
psql -U ferro -d ferro -f migrations/002_totp_2fa.sql
# Apply all migration files in order
for f in migrations/*.sql; do
    psql -U ferro -d ferro -f "$f"
done
```

## Step 4: Import Data

```bash
# Import the SQLite export (convert syntax as needed)
psql -U ferro -d ferro -f ferro_export.sql
```

## Step 5: Configure Ferro

```bash
ferro-server \
    --metadata-db "postgres://ferro:password@localhost:5432/ferro" \
    --storage local:/var/lib/ferro/files \
    --data-dir /var/lib/ferro
```

Or in `ferro.toml`:

```toml
metadata_db = "postgres://ferro:password@localhost:5432/ferro"
storage = "local:/var/lib/ferro/files"
data_dir = "/var/lib/ferro"
```

## Step 6: Verify

```bash
# Check health endpoint
curl http://localhost:8080/healthz | jq .

# Verify metadata count
psql -U ferro -d ferro -c "SELECT COUNT(*) FROM metadata;"
```

## Connection Pooling

For high-concurrency deployments, use PgBouncer:

```ini
[databases]
ferro = host=localhost port=5432 dbname=ferro

[pgbouncer]
pool_mode = transaction
max_client_conn = 1000
default_pool_size = 20
```

Then point Ferro at PgBouncer:

```toml
metadata_db = "postgres://ferro:password@localhost:6432/ferro"
```

## Rollback

If issues arise, stop Ferro and restart with SQLite:

```bash
ferro-server --data-dir /var/lib/ferro --storage local:/var/lib/ferro/files
```

The PostgreSQL database is not modified and can be retried.
