-- API key support for service-to-service and CLI authentication.
-- Keys are stored as SHA-256 hashes; raw keys are never persisted.
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    user_id TEXT NOT NULL,
    permission TEXT NOT NULL DEFAULT 'Read',
    created_at TEXT NOT NULL,
    expires_at TEXT,
    last_used_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);
