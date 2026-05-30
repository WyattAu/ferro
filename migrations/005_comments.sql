-- Ferro Migration 005: File comments/annotations

CREATE TABLE IF NOT EXISTS comments (
    id TEXT PRIMARY KEY NOT NULL,
    path TEXT NOT NULL,
    user_id TEXT NOT NULL,
    parent_id TEXT,
    body TEXT NOT NULL,
    resolved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_comments_path ON comments(path);
CREATE INDEX IF NOT EXISTS idx_comments_parent_id ON comments(parent_id);
