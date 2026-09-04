-- Ferro Migration 015: Add wipe_pending column to users table
-- Tracks pending remote wipe for a user's devices (ferro-server-user-mgmt wipe API).

ALTER TABLE users ADD COLUMN wipe_pending INTEGER NOT NULL DEFAULT 0;
