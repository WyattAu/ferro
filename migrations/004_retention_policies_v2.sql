-- Ferro Migration 004: Enhanced data retention policies

ALTER TABLE retention_policies ADD COLUMN max_age_seconds INTEGER;
ALTER TABLE retention_policies ADD COLUMN max_file_count INTEGER;
ALTER TABLE retention_policies ADD COLUMN min_free_bytes INTEGER;
ALTER TABLE retention_policies ADD COLUMN dry_run INTEGER NOT NULL DEFAULT 0;
