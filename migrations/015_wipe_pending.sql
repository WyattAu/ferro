-- Ferro Migration 015: Add wipe_pending column to users table
-- Tracks whether a remote wipe is pending for a user's devices

ALTER TABLE users ADD COLUMN wipe_pending INTEGER NOT NULL DEFAULT 0;
