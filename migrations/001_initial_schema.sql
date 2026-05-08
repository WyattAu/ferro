-- Ferro Migration 001: Initial schema
-- All tables for the SQLite database

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    role TEXT NOT NULL DEFAULT 'User',
    created_at TEXT NOT NULL,
    last_login TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    storage_quota_bytes INTEGER NOT NULL DEFAULT 0,
    storage_used_bytes INTEGER NOT NULL DEFAULT 0,
    is_ldap INTEGER NOT NULL DEFAULT 0,
    password_hash TEXT
);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);

CREATE TABLE IF NOT EXISTS shares (
    token TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    password TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    created_by TEXT NOT NULL,
    download_count INTEGER NOT NULL DEFAULT 0,
    max_downloads INTEGER,
    is_public INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS favorites (
    path TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS webhooks (
    id TEXT PRIMARY KEY,
    url TEXT NOT NULL,
    events TEXT NOT NULL DEFAULT '[]',
    secret TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS trash (
    original_path TEXT PRIMARY KEY,
    trash_path TEXT NOT NULL,
    deleted_at TEXT NOT NULL,
    size INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS file_tags (
    file_path TEXT NOT NULL,
    tag TEXT NOT NULL,
    PRIMARY KEY (file_path, tag)
);
CREATE INDEX IF NOT EXISTS idx_file_tags_tag ON file_tags(tag);

CREATE TABLE IF NOT EXISTS sync_ops (
    op_id TEXT PRIMARY KEY,
    site_id TEXT NOT NULL,
    clock_counter INTEGER NOT NULL,
    op_type TEXT NOT NULL,
    path TEXT NOT NULL,
    new_path TEXT,
    size INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT,
    owner TEXT NOT NULL DEFAULT '',
    checksum TEXT NOT NULL DEFAULT '',
    timestamp TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sync_ops_clock ON sync_ops(clock_counter);

CREATE TABLE IF NOT EXISTS fed_activities (
    activity_id TEXT PRIMARY KEY,
    actor TEXT NOT NULL,
    type TEXT NOT NULL,
    object TEXT,
    target TEXT,
    published TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    box_type TEXT NOT NULL DEFAULT 'inbox'
);
CREATE INDEX IF NOT EXISTS idx_fed_activities_box ON fed_activities(box_type);

CREATE TABLE IF NOT EXISTS fed_followers (
    actor TEXT NOT NULL,
    follower TEXT NOT NULL,
    PRIMARY KEY (actor, follower)
);

CREATE TABLE IF NOT EXISTS fed_following (
    actor TEXT NOT NULL,
    target TEXT NOT NULL,
    PRIMARY KEY (actor, target)
);

CREATE TABLE IF NOT EXISTS preferences (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS locks (
    token TEXT PRIMARY KEY,
    path TEXT NOT NULL,
    principal TEXT NOT NULL DEFAULT '',
    scope TEXT NOT NULL DEFAULT 'exclusive',
    lock_type TEXT NOT NULL DEFAULT 'write',
    depth TEXT NOT NULL DEFAULT 'infinity',
    timeout_seconds INTEGER NOT NULL DEFAULT 60,
    created_at TEXT NOT NULL,
    refresh_count INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_locks_path ON locks(path);

CREATE TABLE IF NOT EXISTS file_metadata (
    path TEXT PRIMARY KEY,
    content_hash TEXT NOT NULL,
    size INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    is_collection INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    modified_at TEXT NOT NULL DEFAULT (datetime('now')),
    owner TEXT NOT NULL DEFAULT 'anonymous',
    etag TEXT NOT NULL DEFAULT ''
);
