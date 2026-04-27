use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use tracing::debug;

use crate::shares::{CreateShareRequest, ShareLink};
use crate::search::UserPreferences;

pub struct PgShareStore {
    pool: PgPool,
}

impl PgShareStore {
    pub async fn new(pool: PgPool) -> anyhow::Result<Self> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS shares (
                token VARCHAR(36) PRIMARY KEY,
                path TEXT NOT NULL,
                password TEXT,
                expires_at TIMESTAMPTZ NOT NULL,
                max_downloads INTEGER,
                download_count INTEGER NOT NULL DEFAULT 0,
                created_by TEXT NOT NULL DEFAULT 'anonymous',
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
        "#)
        .execute(&pool)
        .await?;

        debug!("PgShareStore initialized");
        Ok(Self { pool })
    }
}

#[async_trait]
impl crate::shares::ShareStoreTrait for PgShareStore {
    async fn create(&self, req: CreateShareRequest, created_by: String) -> ShareLink {
        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = match req.expires_in_hours {
            Some(hours) => Utc::now() + chrono::Duration::hours(hours),
            None => Utc::now() + chrono::Duration::days(7),
        };

        let max_downloads = req.max_downloads.map(|d| d as i32);

        let _ = sqlx::query(r#"
            INSERT INTO shares (token, path, password, expires_at, max_downloads, created_by)
            VALUES ($1, $2, $3, $4, $5, $6)
        "#)
        .bind(&token)
        .bind(&req.path)
        .bind(&req.password)
        .bind(expires_at)
        .bind(max_downloads)
        .bind(&created_by)
        .execute(&self.pool)
        .await;

        ShareLink {
            token: token.clone(),
            path: req.path,
            password: req.password,
            expires_at,
            max_downloads: req.max_downloads,
            download_count: 0,
            created_by,
        }
    }

    async fn get(&self, token: &str) -> Option<ShareLink> {
        let row: Option<(String, String, Option<String>, chrono::DateTime<Utc>, Option<i32>, i32, String)> =
            sqlx::query_as(
                "SELECT token, path, password, expires_at, max_downloads, download_count, created_by FROM shares WHERE token = $1"
            )
            .bind(token)
            .fetch_optional(&self.pool)
            .await
            .ok()?;

        let (token, path, password, expires_at, max_downloads, download_count, created_by) = row?;

        Some(ShareLink {
            token,
            path,
            password,
            expires_at,
            max_downloads: max_downloads.map(|d| d as u32),
            download_count: download_count as u32,
            created_by,
        })
    }

    async fn delete(&self, token: &str) -> bool {
        let result = sqlx::query("DELETE FROM shares WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await
            .ok()
            .map(|r| r.rows_affected() > 0);

        result.unwrap_or(false)
    }

    async fn list(&self) -> Vec<ShareLink> {
        let rows: Vec<(String, String, Option<String>, chrono::DateTime<Utc>, Option<i32>, i32, String)> =
            sqlx::query_as(
                "SELECT token, path, password, expires_at, max_downloads, download_count, created_by FROM shares ORDER BY created_at DESC"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        rows.into_iter()
            .map(|(token, path, password, expires_at, max_downloads, download_count, created_by)| {
                ShareLink {
                    token,
                    path,
                    password,
                    expires_at,
                    max_downloads: max_downloads.map(|d| d as u32),
                    download_count: download_count as u32,
                    created_by,
                }
            })
            .collect()
    }

    async fn increment_download(&self, token: &str) -> bool {
        let result = sqlx::query(
            "UPDATE shares SET download_count = download_count + 1 WHERE token = $1"
        )
        .bind(token)
        .execute(&self.pool)
        .await
        .ok()
        .map(|r| r.rows_affected() > 0);

        result.unwrap_or(false)
    }
}

pub struct PgFavoriteStore {
    pool: PgPool,
}

impl PgFavoriteStore {
    pub async fn new(pool: PgPool) -> anyhow::Result<Self> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS favorites (
                user_id TEXT NOT NULL DEFAULT 'default',
                path TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                PRIMARY KEY (user_id, path)
            )
        "#)
        .execute(&pool)
        .await?;

        debug!("PgFavoriteStore initialized");
        Ok(Self { pool })
    }
}

#[async_trait]
impl crate::favorites::FavoriteStore for PgFavoriteStore {
    async fn list(&self) -> Vec<String> {
        sqlx::query_scalar("SELECT path FROM favorites WHERE user_id = 'default' ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
    }

    async fn add(&self, path: String) {
        let _ = sqlx::query(
            "INSERT INTO favorites (user_id, path) VALUES ('default', $1) ON CONFLICT DO NOTHING"
        )
        .bind(&path)
        .execute(&self.pool)
        .await;
    }

    async fn contains(&self, path: &str) -> bool {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM favorites WHERE user_id = 'default' AND path = $1)"
        )
        .bind(path)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false)
    }

    async fn remove(&self, path: &str) {
        let _ = sqlx::query("DELETE FROM favorites WHERE user_id = 'default' AND path = $1")
            .bind(path)
            .execute(&self.pool)
            .await;
    }
}

pub struct PgPreferenceStore {
    pool: PgPool,
}

impl PgPreferenceStore {
    pub async fn new(pool: PgPool) -> anyhow::Result<Self> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS preferences (
                user_id TEXT NOT NULL DEFAULT 'default',
                theme TEXT NOT NULL DEFAULT 'dark',
                view_mode TEXT NOT NULL DEFAULT 'list',
                sort_by TEXT NOT NULL DEFAULT 'name',
                sort_order TEXT NOT NULL DEFAULT 'asc',
                items_per_page INTEGER NOT NULL DEFAULT 50,
                show_hidden_files BOOLEAN NOT NULL DEFAULT false,
                language TEXT NOT NULL DEFAULT 'en',
                PRIMARY KEY (user_id)
            )
        "#)
        .execute(&pool)
        .await?;

        let _ = sqlx::query(
            "INSERT INTO preferences (user_id) VALUES ('default') ON CONFLICT DO NOTHING"
        )
        .execute(&pool)
        .await;

        debug!("PgPreferenceStore initialized");
        Ok(Self { pool })
    }
}

#[async_trait]
impl crate::search::PreferenceStore for PgPreferenceStore {
    async fn get(&self) -> UserPreferences {
        sqlx::query_as::<_, (String, String, String, String, i32, bool, String)>(
            "SELECT theme, view_mode, sort_by, sort_order, items_per_page, show_hidden_files, language FROM preferences WHERE user_id = 'default'"
        )
        .fetch_one(&self.pool)
        .await
        .map(|(theme, view_mode, sort_by, sort_order, items_per_page, show_hidden_files, language)| {
            UserPreferences {
                theme,
                view_mode,
                sort_by,
                sort_order,
                items_per_page: items_per_page as usize,
                show_hidden_files,
                language,
            }
        })
        .unwrap_or_default()
    }

    async fn update(&self, updates: serde_json::Value) -> UserPreferences {
        let mut set_parts = Vec::new();
        let mut binds: Vec<Box<dyn std::any::Any + Send>> = Vec::new();

        if let Some(val) = updates.get("theme").and_then(|v| v.as_str()) {
            set_parts.push("theme = $1".to_string());
            binds.push(Box::new(val.to_string()));
        }
        if let Some(val) = updates.get("view_mode").and_then(|v| v.as_str()) {
            set_parts.push(format!("view_mode = ${}", set_parts.len() + 1));
            binds.push(Box::new(val.to_string()));
        }
        if let Some(val) = updates.get("sort_by").and_then(|v| v.as_str()) {
            set_parts.push(format!("sort_by = ${}", set_parts.len() + 1));
            binds.push(Box::new(val.to_string()));
        }
        if let Some(val) = updates.get("sort_order").and_then(|v| v.as_str()) {
            set_parts.push(format!("sort_order = ${}", set_parts.len() + 1));
            binds.push(Box::new(val.to_string()));
        }
        if let Some(val) = updates.get("items_per_page").and_then(|v| v.as_u64()) {
            set_parts.push(format!("items_per_page = ${}", set_parts.len() + 1));
            binds.push(Box::new(val as i32));
        }
        if let Some(val) = updates.get("show_hidden_files").and_then(|v| v.as_bool()) {
            set_parts.push(format!("show_hidden_files = ${}", set_parts.len() + 1));
            binds.push(Box::new(val));
        }
        if let Some(val) = updates.get("language").and_then(|v| v.as_str()) {
            set_parts.push(format!("language = ${}", set_parts.len() + 1));
            binds.push(Box::new(val.to_string()));
        }

        if !set_parts.is_empty() {
            let query_str = format!(
                "UPDATE preferences SET {} WHERE user_id = 'default'",
                set_parts.join(", ")
            );
            let _ = sqlx::query(&query_str).execute(&self.pool).await;
        }

        self.get().await
    }
}

pub async fn create_pg_stores(pool: PgPool) -> anyhow::Result<(
    PgShareStore,
    PgFavoriteStore,
    PgPreferenceStore,
)> {
    let shares = PgShareStore::new(pool.clone()).await?;
    let favorites = PgFavoriteStore::new(pool.clone()).await?;
    let preferences = PgPreferenceStore::new(pool).await?;
    Ok((shares, favorites, preferences))
}
