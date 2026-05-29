-- Ferro Migration 003: Extended sharing, guest accounts, branding, GDPR, retention

-- Extend shares table with new capabilities
ALTER TABLE shares ADD COLUMN share_type TEXT NOT NULL DEFAULT 'download';
-- share_type: 'download' | 'upload' | 'view' (secure view, no download)
ALTER TABLE shares ADD COLUMN allow_download INTEGER NOT NULL DEFAULT 1;
-- For upload shares: target directory where uploads go
ALTER TABLE shares ADD COLUMN upload_target TEXT;

-- Branding configuration stored in preferences
-- key='branding', value=JSON with logo_url, primary_color, title, favicon_url

-- Guest accounts: extend users table with guest-specific fields
ALTER TABLE users ADD COLUMN is_guest INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN guest_expires_at TEXT;
-- guest_expires_at: ISO8601 datetime when guest access expires, NULL = never

-- Data retention policies table
CREATE TABLE IF NOT EXISTS retention_policies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path_prefix TEXT NOT NULL,
    max_age_days INTEGER NOT NULL,
    max_versions INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_run_at TEXT
);

-- GDPR export requests table
CREATE TABLE IF NOT EXISTS gdpr_requests (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    request_type TEXT NOT NULL,
    -- request_type: 'export' | 'erasure'
    status TEXT NOT NULL DEFAULT 'pending',
    -- status: 'pending' | 'processing' | 'completed' | 'failed'
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    result_path TEXT,
    -- For export: path to ZIP file; for erasure: summary of deleted data
    error_message TEXT
);
CREATE INDEX IF NOT EXISTS idx_gdpr_requests_user ON gdpr_requests(user_id);

-- Upload share tracking
CREATE TABLE IF NOT EXISTS share_uploads (
    id TEXT PRIMARY KEY,
    share_token TEXT NOT NULL,
    file_path TEXT NOT NULL,
    size INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    uploaded_at TEXT NOT NULL DEFAULT (datetime('now')),
    uploaded_by TEXT NOT NULL DEFAULT 'anonymous'
);
CREATE INDEX IF NOT EXISTS idx_share_uploads_token ON share_uploads(share_token);
