use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::chain::{self, ChainVerificationResult};
use crate::error::AuditError;
use crate::export::{self, ExportFormat};
use crate::retention::RetentionPolicy;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditAction {
    Login,
    Logout,
    FileCreate,
    FileRead,
    FileUpdate,
    FileDelete,
    FileShare,
    FileDownload,
    UserCreate,
    UserUpdate,
    UserDelete,
    PermissionChange,
    ConfigChange,
    Custom(String),
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(s) => write!(f, "Custom({s})"),
            _ => write!(f, "{self:?}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceType {
    User,
    File,
    Folder,
    Share,
    Permission,
    Config,
    ApiKey,
    Custom(String),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub action: AuditAction,
    pub actor_id: String,
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub details: HashMap<String, serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub previous_hash: String,
    pub hash: String,
}

pub struct AuditFilter {
    pub action: Option<AuditAction>,
    pub actor_id: Option<String>,
    pub resource_type: Option<ResourceType>,
    pub resource_id: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: usize,
}

impl Default for AuditFilter {
    fn default() -> Self {
        Self {
            action: None,
            actor_id: None,
            resource_type: None,
            resource_id: None,
            since: None,
            until: None,
            limit: 100,
        }
    }
}

pub struct AuditLog {
    conn: Mutex<Connection>,
    retention: RetentionPolicy,
}

impl AuditLog {
    pub fn new(db_path: &str) -> Result<Self, AuditError> {
        let conn = Connection::open(db_path)?;
        Self::init_db(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            retention: RetentionPolicy::new(),
        })
    }

    pub fn new_in_memory() -> Result<Self, AuditError> {
        let conn = Connection::open_in_memory()?;
        Self::init_db(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            retention: RetentionPolicy::new(),
        })
    }

    fn init_db(conn: &Connection) -> Result<(), AuditError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_entries (
                id              TEXT PRIMARY KEY,
                timestamp       TEXT NOT NULL,
                action          TEXT NOT NULL,
                actor_id        TEXT NOT NULL,
                resource_type   TEXT NOT NULL,
                resource_id     TEXT NOT NULL,
                details         TEXT NOT NULL DEFAULT '{}',
                ip_address      TEXT,
                user_agent      TEXT,
                previous_hash   TEXT NOT NULL,
                hash            TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_entries(action);
            CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_entries(actor_id);
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_entries(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_resource ON audit_entries(resource_type, resource_id);",
        )?;
        Ok(())
    }

    pub fn record(&self, entry: &mut AuditEntry) -> Result<(), AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::LockPoisoned)?;

        let last_hash: String = conn
            .query_row(
                "SELECT hash FROM audit_entries ORDER BY rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();

        entry.previous_hash = last_hash;
        entry.hash = chain::compute_hash(entry);

        let details_json = serde_json::to_string(&entry.details)
            .map_err(|e| AuditError::Export(e.to_string()))?;

        conn.execute(
            "INSERT INTO audit_entries (id, timestamp, action, actor_id, resource_type, resource_id, details, ip_address, user_agent, previous_hash, hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                entry.id,
                entry.timestamp.to_rfc3339(),
                format!("{:?}", entry.action),
                entry.actor_id,
                format!("{:?}", entry.resource_type),
                entry.resource_id,
                details_json,
                entry.ip_address,
                entry.user_agent,
                entry.previous_hash,
                entry.hash,
            ],
        )?;

        Ok(())
    }

    pub fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEntry>, AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::LockPoisoned)?;

        let mut sql = String::from("SELECT id, timestamp, action, actor_id, resource_type, resource_id, details, ip_address, user_agent, previous_hash, hash FROM audit_entries WHERE 1=1");
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref action) = filter.action {
            sql.push_str(" AND action = ?");
            params.push(Box::new(format!("{:?}", action)));
        }
        if let Some(ref actor_id) = filter.actor_id {
            sql.push_str(" AND actor_id = ?");
            params.push(Box::new(actor_id.clone()));
        }
        if let Some(ref rt) = filter.resource_type {
            sql.push_str(" AND resource_type = ?");
            params.push(Box::new(format!("{:?}", rt)));
        }
        if let Some(ref rid) = filter.resource_id {
            sql.push_str(" AND resource_id = ?");
            params.push(Box::new(rid.clone()));
        }
        if let Some(since) = filter.since {
            sql.push_str(" AND timestamp >= ?");
            params.push(Box::new(since.to_rfc3339()));
        }
        if let Some(until) = filter.until {
            sql.push_str(" AND timestamp <= ?");
            params.push(Box::new(until.to_rfc3339()));
        }

        sql.push_str(" ORDER BY rowid ASC LIMIT ?");
        params.push(Box::new(filter.limit as i64));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let details_str: String = row.get(6)?;
            let details: HashMap<String, serde_json::Value> =
                serde_json::from_str(&details_str).unwrap_or_default();

            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                action: parse_action(&row.get::<_, String>(2)?),
                actor_id: row.get(3)?,
                resource_type: parse_resource_type(&row.get::<_, String>(4)?),
                resource_id: row.get(5)?,
                details,
                ip_address: row.get(7)?,
                user_agent: row.get(8)?,
                previous_hash: row.get(9)?,
                hash: row.get(10)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    pub fn verify_chain(&self) -> Result<ChainVerificationResult, AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, action, actor_id, resource_type, resource_id, details, ip_address, user_agent, previous_hash, hash FROM audit_entries ORDER BY rowid ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let details_str: String = row.get(6)?;
            let details: HashMap<String, serde_json::Value> =
                serde_json::from_str(&details_str).unwrap_or_default();

            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                action: parse_action(&row.get::<_, String>(2)?),
                actor_id: row.get(3)?,
                resource_type: parse_resource_type(&row.get::<_, String>(4)?),
                resource_id: row.get(5)?,
                details,
                ip_address: row.get(7)?,
                user_agent: row.get(8)?,
                previous_hash: row.get(9)?,
                hash: row.get(10)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(chain::verify_chain(&entries))
    }

    pub fn export(&self, format: ExportFormat) -> Result<String, AuditError> {
        let entries = self.query(&AuditFilter {
            limit: usize::MAX,
            ..Default::default()
        })?;
        match format {
            ExportFormat::Json => export::export_json(&entries),
            ExportFormat::Csv => export::export_csv(&entries),
        }
    }

    pub fn prune(&self) -> Result<usize, AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::LockPoisoned)?;

        let mut stmt = conn.prepare(
            "SELECT id, timestamp, action, actor_id, resource_type, resource_id, details, ip_address, user_agent, previous_hash, hash FROM audit_entries ORDER BY rowid ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let details_str: String = row.get(6)?;
            let details: HashMap<String, serde_json::Value> =
                serde_json::from_str(&details_str).unwrap_or_default();

            Ok(AuditEntry {
                id: row.get(0)?,
                timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                action: parse_action(&row.get::<_, String>(2)?),
                actor_id: row.get(3)?,
                resource_type: parse_resource_type(&row.get::<_, String>(4)?),
                resource_id: row.get(5)?,
                details,
                ip_address: row.get(7)?,
                user_agent: row.get(8)?,
                previous_hash: row.get(9)?,
                hash: row.get(10)?,
            })
        })?;

        let mut entries: Vec<AuditEntry> = rows.flatten().collect();
        let count = self.retention.apply(&mut entries);

        let ids_to_delete: Vec<String> = entries
            .iter()
            .filter(|e| e.previous_hash.is_empty() && e.hash.is_empty())
            .map(|e| e.id.clone())
            .collect();

        for id in &ids_to_delete {
            conn.execute("DELETE FROM audit_entries WHERE id = ?1", rusqlite::params![id])?;
        }

        Ok(count)
    }

    pub fn count(&self) -> Result<usize, AuditError> {
        let conn = self.conn.lock().map_err(|_| AuditError::LockPoisoned)?;
        let count: usize = conn.query_row(
            "SELECT COUNT(*) FROM audit_entries",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

fn parse_action(s: &str) -> AuditAction {
    match s {
        "Login" => AuditAction::Login,
        "Logout" => AuditAction::Logout,
        "FileCreate" => AuditAction::FileCreate,
        "FileRead" => AuditAction::FileRead,
        "FileUpdate" => AuditAction::FileUpdate,
        "FileDelete" => AuditAction::FileDelete,
        "FileShare" => AuditAction::FileShare,
        "FileDownload" => AuditAction::FileDownload,
        "UserCreate" => AuditAction::UserCreate,
        "UserUpdate" => AuditAction::UserUpdate,
        "UserDelete" => AuditAction::UserDelete,
        "PermissionChange" => AuditAction::PermissionChange,
        "ConfigChange" => AuditAction::ConfigChange,
        _ if s.starts_with("Custom(") && s.ends_with(')') => {
            let inner = &s[7..s.len() - 1];
            AuditAction::Custom(inner.trim_matches('"').to_string())
        }
        _ => AuditAction::Custom(s.to_string()),
    }
}

fn parse_resource_type(s: &str) -> ResourceType {
    match s {
        "User" => ResourceType::User,
        "File" => ResourceType::File,
        "Folder" => ResourceType::Folder,
        "Share" => ResourceType::Share,
        "Permission" => ResourceType::Permission,
        "Config" => ResourceType::Config,
        "ApiKey" => ResourceType::ApiKey,
        _ if s.starts_with("Custom(") && s.ends_with(')') => {
            let inner = &s[7..s.len() - 1];
            ResourceType::Custom(inner.trim_matches('"').to_string())
        }
        _ => ResourceType::Custom(s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_entry(id: &str, action: AuditAction) -> AuditEntry {
        AuditEntry {
            id: id.to_string(),
            timestamp: Utc::now(),
            action,
            actor_id: "actor-1".to_string(),
            resource_type: ResourceType::File,
            resource_id: format!("res-{id}"),
            details: HashMap::new(),
            ip_address: Some("10.0.0.1".to_string()),
            user_agent: None,
            previous_hash: String::new(),
            hash: String::new(),
        }
    }

    #[test]
    fn test_record_and_retrieve() {
        let log = AuditLog::new_in_memory().unwrap();
        let mut e1 = make_entry("e1", AuditAction::FileCreate);
        log.record(&mut e1).unwrap();
        let entries = log.query(&AuditFilter::default()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "e1");
        assert_eq!(entries[0].action, AuditAction::FileCreate);
    }

    #[test]
    fn test_multiple_records() {
        let log = AuditLog::new_in_memory().unwrap();
        for i in 0..5 {
            let mut e = make_entry(&format!("e{i}"), AuditAction::FileRead);
            log.record(&mut e).unwrap();
        }
        assert_eq!(log.count().unwrap(), 5);
    }

    #[test]
    fn test_chain_hashes_linked() {
        let log = AuditLog::new_in_memory().unwrap();
        let mut e1 = make_entry("e1", AuditAction::Login);
        log.record(&mut e1).unwrap();
        let mut e2 = make_entry("e2", AuditAction::Logout);
        log.record(&mut e2).unwrap();

        let entries = log.query(&AuditFilter::default()).unwrap();
        assert_eq!(entries[0].previous_hash, "");
        assert_eq!(entries[1].previous_hash, entries[0].hash);
    }

    #[test]
    fn test_verify_chain_valid() {
        let log = AuditLog::new_in_memory().unwrap();
        for i in 0..3 {
            let mut e = make_entry(&format!("e{i}"), AuditAction::FileCreate);
            log.record(&mut e).unwrap();
        }
        let result = log.verify_chain().unwrap();
        assert!(result.valid);
        assert_eq!(result.total, 3);
    }

    #[test]
    fn test_filter_by_action() {
        let log = AuditLog::new_in_memory().unwrap();
        let mut e1 = make_entry("e1", AuditAction::FileCreate);
        let mut e2 = make_entry("e2", AuditAction::FileDelete);
        log.record(&mut e1).unwrap();
        log.record(&mut e2).unwrap();

        let results = log
            .query(&AuditFilter {
                action: Some(AuditAction::FileDelete),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, AuditAction::FileDelete);
    }

    #[test]
    fn test_filter_by_actor() {
        let log = AuditLog::new_in_memory().unwrap();
        let mut e = make_entry("e1", AuditAction::Login);
        e.actor_id = "alice".to_string();
        log.record(&mut e).unwrap();
        let mut e2 = make_entry("e2", AuditAction::Login);
        e2.actor_id = "bob".to_string();
        log.record(&mut e2).unwrap();

        let results = log
            .query(&AuditFilter {
                actor_id: Some("alice".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actor_id, "alice");
    }

    #[test]
    fn test_filter_limit() {
        let log = AuditLog::new_in_memory().unwrap();
        for i in 0..10 {
            let mut e = make_entry(&format!("e{i}"), AuditAction::FileRead);
            log.record(&mut e).unwrap();
        }
        let results = log
            .query(&AuditFilter {
                limit: 3,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_empty_log() {
        let log = AuditLog::new_in_memory().unwrap();
        assert_eq!(log.count().unwrap(), 0);
        let entries = log.query(&AuditFilter::default()).unwrap();
        assert!(entries.is_empty());
        let result = log.verify_chain().unwrap();
        assert!(result.valid);
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_custom_action_roundtrip() {
        let log = AuditLog::new_in_memory().unwrap();
        let mut e = make_entry("e1", AuditAction::Custom("SpecialAction".to_string()));
        log.record(&mut e).unwrap();
        let entries = log.query(&AuditFilter::default()).unwrap();
        assert_eq!(entries[0].action, AuditAction::Custom("SpecialAction".to_string()));
    }

    #[test]
    fn test_export_json() {
        let log = AuditLog::new_in_memory().unwrap();
        let mut e = make_entry("e1", AuditAction::Login);
        log.record(&mut e).unwrap();
        let json = log.export(ExportFormat::Json).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_export_csv() {
        let log = AuditLog::new_in_memory().unwrap();
        let mut e = make_entry("e1", AuditAction::Login);
        log.record(&mut e).unwrap();
        let csv = log.export(ExportFormat::Csv).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_sequential_recording() {
        let log = AuditLog::new_in_memory().unwrap();
        for i in 0..10 {
            let mut e = AuditEntry {
                id: format!("sequential-{i}"),
                timestamp: Utc::now(),
                action: AuditAction::FileCreate,
                actor_id: "sequential-actor".to_string(),
                resource_type: ResourceType::File,
                resource_id: format!("res-{i}"),
                details: HashMap::new(),
                ip_address: None,
                user_agent: None,
                previous_hash: String::new(),
                hash: String::new(),
            };
            log.record(&mut e).unwrap();
        }
        assert_eq!(log.count().unwrap(), 10);
    }

    #[tokio::test]
    async fn test_concurrent_recording_safe() {
        use std::sync::Arc;
        let log = Arc::new(AuditLog::new_in_memory().unwrap());
        let mut handles = Vec::new();

        for i in 0..10 {
            let log = Arc::clone(&log);
            handles.push(tokio::spawn(async move {
                let mut e = AuditEntry {
                    id: format!("safe-{i}"),
                    timestamp: Utc::now(),
                    action: AuditAction::FileCreate,
                    actor_id: "safe-actor".to_string(),
                    resource_type: ResourceType::File,
                    resource_id: format!("res-{i}"),
                    details: HashMap::new(),
                    ip_address: None,
                    user_agent: None,
                    previous_hash: String::new(),
                    hash: String::new(),
                };
                log.record(&mut e).unwrap();
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(log.count().unwrap(), 10);
    }
}
