-- Ferro Migration 008: Remote WebDAV mount proxy

CREATE TABLE IF NOT EXISTS remote_mounts (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    remote_url TEXT NOT NULL,
    local_path TEXT NOT NULL,
    username TEXT,
    password TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_remote_mounts_name ON remote_mounts(name);
CREATE INDEX IF NOT EXISTS idx_remote_mounts_enabled ON remote_mounts(enabled);
