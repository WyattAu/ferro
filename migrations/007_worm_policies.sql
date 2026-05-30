-- Ferro Migration 007: WORM (Write Once Read Many) policies

CREATE TABLE IF NOT EXISTS worm_policies (
    id TEXT PRIMARY KEY NOT NULL,
    path_prefix TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_worm_policies_prefix ON worm_policies(path_prefix);
CREATE INDEX IF NOT EXISTS idx_worm_policies_enabled ON worm_policies(enabled);
