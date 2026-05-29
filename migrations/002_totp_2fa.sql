-- Ferro Migration 002: Add TOTP 2FA columns to users table
-- Adds totp_secret (Base32-encoded) and totp_enabled (boolean) columns

ALTER TABLE users ADD COLUMN totp_secret TEXT;
ALTER TABLE users ADD COLUMN totp_enabled INTEGER NOT NULL DEFAULT 0;
