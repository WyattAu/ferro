-- Ferro Migration 001: Initial file_metadata table
-- This replaces the inline CREATE TABLE in sqlx_metadata.rs

CREATE TABLE IF NOT EXISTS file_metadata (
    path VARCHAR(4096) PRIMARY KEY,
    content_hash VARCHAR(64) NOT NULL,
    size BIGINT NOT NULL DEFAULT 0,
    mime_type VARCHAR(256) NOT NULL DEFAULT 'application/octet-stream',
    is_collection BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    modified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    owner VARCHAR(256) NOT NULL DEFAULT 'anonymous',
    etag VARCHAR(128) NOT NULL DEFAULT ''
);

-- PostgreSQL-specific index using varchar_pattern_ops for prefix searches
CREATE INDEX IF NOT EXISTS idx_file_metadata_path_prefix
    ON file_metadata (path varchar_pattern_ops);

-- SQLite compatibility note: SQLite uses GLOB instead of varchar_pattern_ops.
-- The application code handles this difference in the SqliteMetadataStore.
