use rusqlite::Connection;
use tracing::info;

pub type DbHandle = Arc<std::sync::Mutex<Connection>>;

use std::sync::Arc;

#[cfg(test)]
const SCHEMA_VERSION: i64 = 1;

const MIGRATIONS: &[(&str, &str)] = &[(
    "001",
    include_str!("../../../migrations/001_initial_schema.sql"),
)];

fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );",
    )?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for &(version_str, sql) in MIGRATIONS {
        let version: i64 = version_str.parse().unwrap();
        if version > current_version {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                rusqlite::params![version],
            )?;
            tx.commit()?;
            info!("applied migration {}", version_str);
        }
    }

    Ok(())
}

pub fn open_db(data_dir: &str) -> Result<Connection, rusqlite::Error> {
    let db_path = std::path::Path::new(data_dir).join("ferro.db");
    let conn = Connection::open(&db_path)?;

    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;",
    )?;

    run_migrations(&conn)?;

    info!("SQLite database opened: {}", db_path.display());
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    #[test]
    fn test_open_db_creates_tables() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(dir.path().to_str().unwrap()).unwrap();

        let tables = [
            "users",
            "shares",
            "favorites",
            "webhooks",
            "trash",
            "file_tags",
            "sync_ops",
            "fed_activities",
            "fed_followers",
            "fed_following",
            "preferences",
            "locks",
            "file_metadata",
        ];
        for table in &tables {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "Table {} should be empty", table);
        }
    }

    #[test]
    fn test_db_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        {
            let conn = open_db(path).unwrap();
            conn.execute(
                "INSERT INTO users (id, username, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                params!["user-1", "admin", "Admin", "2024-01-01T00:00:00+00:00"],
            )
            .unwrap();
        }

        {
            let conn = open_db(path).unwrap();
            let name: String = conn
                .query_row(
                    "SELECT display_name FROM users WHERE id = 'user-1'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(name, "Admin");
        }
    }

    #[test]
    fn test_wal_mode_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(dir.path().to_str().unwrap()).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn test_schema_version_tracked() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(dir.path().to_str().unwrap()).unwrap();

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_migration_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let conn1 = open_db(path).unwrap();
        let version1: i64 = conn1
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();

        let conn2 = open_db(path).unwrap();
        let version2: i64 = conn2
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(version1, version2);
        assert_eq!(version1, SCHEMA_VERSION);
    }
}
