-- Ferro Migration 017: tasks.owner column (per-user scoping)
-- Legacy rows keep owner='' (visible to all authenticated users).

ALTER TABLE tasks ADD COLUMN owner TEXT NOT NULL DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_tasks_owner ON tasks(owner);
