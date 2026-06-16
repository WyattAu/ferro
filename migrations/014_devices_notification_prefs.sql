-- Ferro Migration 014: Devices and notification preferences tables

CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    platform TEXT NOT NULL CHECK(platform IN ('ios', 'android', 'desktop')),
    push_token TEXT NOT NULL,
    last_seen TEXT NOT NULL DEFAULT (datetime('now')),
    revoked INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_devices_user ON devices(user_id);
CREATE INDEX IF NOT EXISTS idx_devices_platform ON devices(platform);
CREATE INDEX IF NOT EXISTS idx_devices_revoked ON devices(revoked);

CREATE TABLE IF NOT EXISTS notification_prefs (
    user_id TEXT PRIMARY KEY NOT NULL,
    share_received_email INTEGER NOT NULL DEFAULT 1,
    share_received_push INTEGER NOT NULL DEFAULT 1,
    comment_added_email INTEGER NOT NULL DEFAULT 1,
    comment_added_push INTEGER NOT NULL DEFAULT 1,
    task_assigned_email INTEGER NOT NULL DEFAULT 1,
    task_assigned_push INTEGER NOT NULL DEFAULT 1,
    mention_push INTEGER NOT NULL DEFAULT 1,
    system_alert_push INTEGER NOT NULL DEFAULT 1,
    daily_digest_email INTEGER NOT NULL DEFAULT 1
);