-- Mail accounts table (P3-03)
CREATE TABLE IF NOT EXISTS mail_accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email_address TEXT NOT NULL,
    imap_host TEXT NOT NULL,
    imap_port INTEGER NOT NULL DEFAULT 993,
    imap_security TEXT NOT NULL DEFAULT 'ssl',  -- 'ssl' or 'starttls'
    imap_username TEXT NOT NULL,
    imap_password TEXT NOT NULL,                 -- encrypted
    smtp_host TEXT NOT NULL,
    smtp_port INTEGER NOT NULL DEFAULT 587,
    smtp_security TEXT NOT NULL DEFAULT 'starttls',  -- 'ssl' or 'starttls'
    smtp_username TEXT NOT NULL,
    smtp_password TEXT NOT NULL,                 -- encrypted
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Link analytics table (P3-06)
CREATE TABLE IF NOT EXISTS link_analytics (
    id TEXT PRIMARY KEY,
    share_token TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    ip_address TEXT,
    user_agent TEXT,
    referrer TEXT,
    file_path TEXT,
    event_type TEXT NOT NULL DEFAULT 'view'      -- 'view' or 'download'
);
CREATE INDEX IF NOT EXISTS idx_link_analytics_token ON link_analytics(share_token);
CREATE INDEX IF NOT EXISTS idx_link_analytics_timestamp ON link_analytics(timestamp);

-- Watermark policies table (P3-07)
CREATE TABLE IF NOT EXISTS watermark_policies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    text TEXT NOT NULL,
    position TEXT NOT NULL DEFAULT 'center',     -- 'center', 'top-left', 'top-right', 'bottom-left', 'bottom-right', 'tiled'
    opacity REAL NOT NULL DEFAULT 0.3,
    font_size INTEGER NOT NULL DEFAULT 48,
    color TEXT NOT NULL DEFAULT '#FFFFFF',
    scope TEXT NOT NULL DEFAULT 'all',           -- 'all', 'images', 'documents'
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
