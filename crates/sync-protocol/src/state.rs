use crate::protocol::{NodeId, VectorClock};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during sync state operations.
#[derive(Debug, Error)]
pub enum SyncStateError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("sync state not found for node '{0}'")]
    NotFound(String),
}

/// High-level sync state for a peer node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// No sync activity in progress.
    Idle,
    /// Sync is actively in progress.
    Syncing,
    /// A conflict was detected and needs resolution.
    Conflict,
    /// An error occurred during sync.
    Error,
}

/// Persistent record of sync state for a single peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerSyncState {
    /// The remote node identifier.
    pub node_id: NodeId,
    /// Current sync status.
    pub status: SyncStatus,
    /// The vector clock of the last successful sync with this peer.
    pub last_sync_clock: VectorClock,
    /// When the last successful sync completed.
    pub last_sync_at: Option<DateTime<Utc>>,
    /// When the last sync attempt started.
    pub last_attempt_at: Option<DateTime<Utc>>,
    /// Number of consecutive sync failures (resets on success).
    pub consecutive_failures: u32,
    /// Human-readable description of the last error, if any.
    pub last_error: Option<String>,
    /// Number of files synced in the last successful sync.
    pub last_sync_file_count: u64,
}

impl PeerSyncState {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            status: SyncStatus::Idle,
            last_sync_clock: VectorClock::new(),
            last_sync_at: None,
            last_attempt_at: None,
            consecutive_failures: 0,
            last_error: None,
            last_sync_file_count: 0,
        }
    }
}

/// Manages persistent sync state for all peers, stored in SQLite.
pub struct SyncStateManager {
    conn: std::sync::Mutex<Connection>,
}

unsafe impl Send for SyncStateManager {}
unsafe impl Sync for SyncStateManager {}

impl SyncStateManager {
    /// Create a new manager and initialize the schema if needed.
    pub fn new(conn: Connection) -> Result<Self, SyncStateError> {
        let manager = Self {
            conn: std::sync::Mutex::new(conn),
        };
        manager.ensure_schema()?;
        Ok(manager)
    }

    /// Create an in-memory manager (for testing).
    pub fn new_in_memory() -> Result<Self, SyncStateError> {
        let conn = Connection::open_in_memory()?;
        let manager = Self {
            conn: std::sync::Mutex::new(conn),
        };
        manager.ensure_schema()?;
        Ok(manager)
    }

    fn ensure_schema(&self) -> Result<(), SyncStateError> {
        self.conn.lock().unwrap().execute_batch(
            "CREATE TABLE IF NOT EXISTS sync_peer_state (
                node_id       TEXT PRIMARY KEY,
                status        TEXT NOT NULL DEFAULT 'idle',
                last_sync_clock TEXT NOT NULL DEFAULT '{}',
                last_sync_at  TEXT,
                last_attempt_at TEXT,
                consecutive_failures INTEGER NOT NULL DEFAULT 0,
                last_error    TEXT,
                last_sync_file_count INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS sync_log (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                node_id       TEXT NOT NULL,
                direction     TEXT NOT NULL,
                status        TEXT NOT NULL,
                file_count    INTEGER NOT NULL DEFAULT 0,
                error_message TEXT,
                started_at    TEXT NOT NULL,
                completed_at  TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_sync_log_node ON sync_log(node_id);",
        )?;
        Ok(())
    }

    /// Get the sync state for a peer, creating a default if it doesn't exist.
    pub fn get_or_create(&self, node_id: &str) -> Result<PeerSyncState, SyncStateError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT node_id, status, last_sync_clock, last_sync_at,
                    last_attempt_at, consecutive_failures, last_error,
                    last_sync_file_count
             FROM sync_peer_state WHERE node_id = ?1",
        )?;

        let result = stmt.query_row(params![node_id], |row| {
            let status_str: String = row.get(1)?;
            let clock_json: String = row.get(2)?;
            let last_sync_at: Option<String> = row.get(3)?;
            let last_attempt_at: Option<String> = row.get(4)?;
            let consecutive_failures: u32 = row.get(5)?;
            let last_error: Option<String> = row.get(6)?;
            let last_sync_file_count: u64 = row.get(7)?;

            let status = match status_str.as_str() {
                "idle" => SyncStatus::Idle,
                "syncing" => SyncStatus::Syncing,
                "conflict" => SyncStatus::Conflict,
                "error" => SyncStatus::Error,
                _ => SyncStatus::Idle,
            };

            let clock: VectorClock = serde_json::from_str(&clock_json).unwrap_or_default();

            let last_sync_at = last_sync_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let last_attempt_at = last_attempt_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            Ok(PeerSyncState {
                node_id: node_id.to_string(),
                status,
                last_sync_clock: clock,
                last_sync_at,
                last_attempt_at,
                consecutive_failures,
                last_error,
                last_sync_file_count,
            })
        });

        match result {
            Ok(state) => Ok(state),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let state = PeerSyncState::new(node_id.to_string());
                self.save(&state)?;
                Ok(state)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Persist the sync state for a peer.
    pub fn save(&self, state: &PeerSyncState) -> Result<(), SyncStateError> {
        let status_str = match state.status {
            SyncStatus::Idle => "idle",
            SyncStatus::Syncing => "syncing",
            SyncStatus::Conflict => "conflict",
            SyncStatus::Error => "error",
        };
        let clock_json = serde_json::to_string(&state.last_sync_clock)?;
        let last_sync_at = state.last_sync_at.map(|dt| dt.to_rfc3339());
        let last_attempt_at = state.last_attempt_at.map(|dt| dt.to_rfc3339());

        self.conn.lock().unwrap().execute(
            "INSERT OR REPLACE INTO sync_peer_state
             (node_id, status, last_sync_clock, last_sync_at,
              last_attempt_at, consecutive_failures, last_error,
              last_sync_file_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                state.node_id,
                status_str,
                clock_json,
                last_sync_at,
                last_attempt_at,
                state.consecutive_failures,
                state.last_error,
                state.last_sync_file_count,
            ],
        )?;
        Ok(())
    }

    /// Transition a peer to a new status and persist.
    pub fn set_status(
        &self,
        node_id: &str,
        status: SyncStatus,
    ) -> Result<PeerSyncState, SyncStateError> {
        let mut state = self.get_or_create(node_id)?;
        state.status = status;
        if status == SyncStatus::Syncing {
            state.last_attempt_at = Some(Utc::now());
        }
        self.save(&state)?;
        Ok(state)
    }

    /// Record a successful sync and persist.
    pub fn record_success(
        &self,
        node_id: &str,
        clock: VectorClock,
        file_count: u64,
    ) -> Result<PeerSyncState, SyncStateError> {
        let mut state = self.get_or_create(node_id)?;
        state.status = SyncStatus::Idle;
        state.last_sync_clock = clock;
        state.last_sync_at = Some(Utc::now());
        state.consecutive_failures = 0;
        state.last_error = None;
        state.last_sync_file_count = file_count;
        self.save(&state)?;
        Ok(state)
    }

    /// Record a failed sync and persist.
    pub fn record_failure(
        &self,
        node_id: &str,
        error: &str,
    ) -> Result<PeerSyncState, SyncStateError> {
        let mut state = self.get_or_create(node_id)?;
        state.status = SyncStatus::Error;
        state.consecutive_failures += 1;
        state.last_error = Some(error.to_string());
        self.save(&state)?;
        Ok(state)
    }

    /// Log a sync event to the audit table.
    pub fn log_event(
        &self,
        node_id: &str,
        direction: &str,
        status: &str,
        file_count: u64,
        error_message: Option<&str>,
    ) -> Result<(), SyncStateError> {
        let now = Utc::now().to_rfc3339();
        self.conn.lock().unwrap().execute(
            "INSERT INTO sync_log
             (node_id, direction, status, file_count, error_message, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![node_id, direction, status, file_count, error_message, now],
        )?;
        Ok(())
    }

    /// List all known peer states.
    pub fn list_peers(&self) -> Result<Vec<PeerSyncState>, SyncStateError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT node_id, status, last_sync_clock, last_sync_at,
                    last_attempt_at, consecutive_failures, last_error,
                    last_sync_file_count
             FROM sync_peer_state ORDER BY node_id",
        )?;

        let rows = stmt.query_map([], |row| {
            let node_id: String = row.get(0)?;
            let status_str: String = row.get(1)?;
            let clock_json: String = row.get(2)?;
            let last_sync_at: Option<String> = row.get(3)?;
            let last_attempt_at: Option<String> = row.get(4)?;
            let consecutive_failures: u32 = row.get(5)?;
            let last_error: Option<String> = row.get(6)?;
            let last_sync_file_count: u64 = row.get(7)?;

            let status = match status_str.as_str() {
                "idle" => SyncStatus::Idle,
                "syncing" => SyncStatus::Syncing,
                "conflict" => SyncStatus::Conflict,
                "error" => SyncStatus::Error,
                _ => SyncStatus::Idle,
            };

            let clock: VectorClock = serde_json::from_str(&clock_json).unwrap_or_default();

            let last_sync_at = last_sync_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let last_attempt_at = last_attempt_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            Ok(PeerSyncState {
                node_id,
                status,
                last_sync_clock: clock,
                last_sync_at,
                last_attempt_at,
                consecutive_failures,
                last_error,
                last_sync_file_count,
            })
        })?;

        let mut peers = Vec::new();
        for row in rows {
            peers.push(row?);
        }
        Ok(peers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> SyncStateManager {
        SyncStateManager::new_in_memory().unwrap()
    }

    #[test]
    fn test_get_or_create_default() {
        let mgr = make_manager();
        let state = mgr.get_or_create("node-a").unwrap();
        assert_eq!(state.node_id, "node-a");
        assert_eq!(state.status, SyncStatus::Idle);
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn test_save_and_get() {
        let mgr = make_manager();
        let mut state = PeerSyncState::new("node-b".to_string());
        state.status = SyncStatus::Syncing;
        state.consecutive_failures = 3;
        mgr.save(&state).unwrap();

        let fetched = mgr.get_or_create("node-b").unwrap();
        assert_eq!(fetched.status, SyncStatus::Syncing);
        assert_eq!(fetched.consecutive_failures, 3);
    }

    #[test]
    fn test_record_success() {
        let mgr = make_manager();
        let mut clock = VectorClock::new();
        clock.increment("node-a");

        let state = mgr.record_success("node-a", clock.clone(), 42).unwrap();
        assert_eq!(state.status, SyncStatus::Idle);
        assert_eq!(state.last_sync_clock, clock);
        assert_eq!(state.last_sync_file_count, 42);
        assert_eq!(state.consecutive_failures, 0);
    }

    #[test]
    fn test_record_failure() {
        let mgr = make_manager();
        let state = mgr.record_failure("node-a", "connection refused").unwrap();
        assert_eq!(state.status, SyncStatus::Error);
        assert_eq!(state.consecutive_failures, 1);
        assert_eq!(state.last_error.as_deref(), Some("connection refused"));
    }

    #[test]
    fn test_list_peers() {
        let mgr = make_manager();
        mgr.get_or_create("node-a").unwrap();
        mgr.get_or_create("node-b").unwrap();
        mgr.get_or_create("node-c").unwrap();

        let peers = mgr.list_peers().unwrap();
        assert_eq!(peers.len(), 3);
        let ids: Vec<&str> = peers.iter().map(|p| p.node_id.as_str()).collect();
        assert!(ids.contains(&"node-a"));
        assert!(ids.contains(&"node-b"));
        assert!(ids.contains(&"node-c"));
    }
}
