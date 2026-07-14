use rusqlite::Connection;
use tracing::info;

pub use common::DbHandle;

#[cfg(test)]
const SCHEMA_VERSION: i64 = 14;

const MIGRATIONS: &[(&str, &str)] = &[
    ("001", include_str!("../../../migrations/001_initial_schema.sql")),
    ("002", include_str!("../../../migrations/002_totp_2fa.sql")),
    ("003", include_str!("../../../migrations/003_extended_features.sql")),
    ("004", include_str!("../../../migrations/004_retention_policies_v2.sql")),
    ("005", include_str!("../../../migrations/005_comments.sql")),
    ("006", include_str!("../../../migrations/006_event_triggers.sql")),
    ("007", include_str!("../../../migrations/007_worm_policies.sql")),
    ("008", include_str!("../../../migrations/008_remote_mounts.sql")),
    ("009", include_str!("../../../migrations/009_api_keys.sql")),
    ("011", include_str!("../../../migrations/011_push_notifications.sql")),
    ("012", include_str!("../../../migrations/012_notes_tasks.sql")),
    (
        "013",
        include_str!("../../../migrations/013_mail_analytics_watermark.sql"),
    ),
    (
        "014",
        include_str!("../../../migrations/014_devices_notification_prefs.sql"),
    ),
];

fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL
        );",
    )?;

    let current_version: i64 = conn
        .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    for &(version_str, sql) in MIGRATIONS {
        let version: i64 = version_str
            .parse()
            .expect("migration version constant must be a valid i64");
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
    let mut conn = Connection::open(&db_path)?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;")?;

    // Slow query logging: warn on queries taking longer than 100ms.
    conn.profile(Some(|sql: &str, duration: std::time::Duration| {
        let ms = duration.as_millis();
        if ms > 100 {
            tracing::warn!("Slow SQLite query ({}ms): {}", ms, sql.trim());
        }
    }));

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
            "comments",
            "worm_policies",
            "remote_mounts",
            "api_keys",
            "notes",
            "tasks",
            "devices",
            "notification_prefs",
        ];
        // SAFETY: `table` values come from a hardcoded constant array above, not user input.
        // This is the ONLY acceptable use of format! in SQL queries in this codebase.
        // CI enforces no format! in SQL outside this test function.
        for table in &tables {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {}", table), [], |row| row.get(0))
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
                .query_row("SELECT display_name FROM users WHERE id = 'user-1'", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(name, "Admin");
        }
    }

    #[test]
    fn test_wal_mode_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(dir.path().to_str().unwrap()).unwrap();
        let mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0)).unwrap();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn test_schema_version_tracked() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(dir.path().to_str().unwrap()).unwrap();

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_migration_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        let conn1 = open_db(path).unwrap();
        let version1: i64 = conn1
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();

        let conn2 = open_db(path).unwrap();
        let version2: i64 = conn2
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();

        assert_eq!(version1, version2);
        assert_eq!(version1, SCHEMA_VERSION);
    }
}
