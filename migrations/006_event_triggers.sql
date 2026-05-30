-- Ferro Migration 006: WASM event triggers

CREATE TABLE IF NOT EXISTS wasm_event_triggers (
    id TEXT PRIMARY KEY NOT NULL,
    event_type TEXT NOT NULL,
    worker_name TEXT NOT NULL,
    path_pattern TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_wasm_event_triggers_event ON wasm_event_triggers(event_type);
CREATE INDEX IF NOT EXISTS idx_wasm_event_triggers_enabled ON wasm_event_triggers(enabled);
