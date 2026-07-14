use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::AppState;
use crate::api_error::ApiError;

/// Link analytics entry for a single access event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkAnalyticsEntry {
    pub id: String,
    pub share_token: String,
    pub timestamp: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub file_path: Option<String>,
    pub event_type: String,
}

/// Summary stats for a share link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkStats {
    pub token: String,
    pub path: String,
    pub total_views: u64,
    pub total_downloads: u64,
    pub unique_visitors: u64,
    pub top_referrers: Vec<ReferrerCount>,
    pub daily_breakdown: Vec<DailyCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferrerCount {
    pub referrer: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCount {
    pub date: String,
    pub views: u64,
    pub downloads: u64,
}

/// Global analytics overview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsOverview {
    pub total_views: u64,
    pub total_downloads: u64,
    pub total_shares: u64,
    pub top_links: Vec<TopLink>,
    pub storage_used_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLink {
    pub token: String,
    pub path: String,
    pub views: u64,
}

// ---------------------------------------------------------------------------
// Middleware: track share link access
// ---------------------------------------------------------------------------

/// Record a share link access in the analytics table.
pub fn track_link_access(
    db: &rusqlite::Connection,
    share_token: &str,
    file_path: &str,
    event_type: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    referrer: Option<&str>,
) {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    if let Err(e) = db.execute(
        "INSERT INTO link_analytics (id, share_token, timestamp, ip_address, user_agent, referrer, file_path, event_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, share_token, timestamp, ip_address, user_agent, referrer, file_path, event_type],
    ) {
        warn!("Failed to record link analytics: {}", e);
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// GET /analytics/links — list all share links with stats.
pub async fn list_link_analytics(State(state): State<AppState>) -> Response {
    let links = state.share_store.list().await;
    let mut results = Vec::new();

    // Collect stats while holding the lock (no await needed here)
    let stats_data: Vec<(u64, u64)> = if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        links
            .iter()
            .map(|link| {
                let views: u64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM link_analytics WHERE share_token = ?1 AND event_type = 'view'",
                        params![link.token],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                let downloads: u64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM link_analytics WHERE share_token = ?1 AND event_type = 'download'",
                        params![link.token],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                (views, downloads)
            })
            .collect()
    } else {
        links.iter().map(|_| (0, 0)).collect()
    };

    for (link, (total_views, total_downloads)) in links.iter().zip(stats_data) {
        results.push(serde_json::json!({
            "token": link.token,
            "path": link.path,
            "total_views": total_views,
            "total_downloads": total_downloads,
            "download_count": link.download_count,
            "expires_at": link.expires_at.to_rfc3339(),
            "created_by": link.created_by,
        }));
    }

    (StatusCode::OK, Json(serde_json::json!({ "links": results }))).into_response()
}

/// GET /analytics/links/{id}/stats — detailed stats for a link.
pub async fn analytics_link_stats(Path(token): Path<String>, State(state): State<AppState>) -> Response {
    let link = match state.share_store.get(&token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::NOT_FOUND, "Share link not found"),
    };

    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };

    // All DB queries happen in a single lock scope, no await needed
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());

    let total_views: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM link_analytics WHERE share_token = ?1 AND event_type = 'view'",
            params![token],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_downloads: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM link_analytics WHERE share_token = ?1 AND event_type = 'download'",
            params![token],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let unique_visitors: u64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT ip_address) FROM link_analytics WHERE share_token = ?1 AND ip_address IS NOT NULL",
            params![token],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let mut referrer_stmt = match conn.prepare(
        "SELECT COALESCE(referrer, 'direct') as ref, COUNT(*) as cnt FROM link_analytics WHERE share_token = ?1 GROUP BY ref ORDER BY cnt DESC LIMIT 10",
    ) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to prepare referrer query: {}", e);
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database query failed");
        }
    };
    let top_referrers: Vec<ReferrerCount> = referrer_stmt
        .query_map(params![token], |row| {
            Ok(ReferrerCount {
                referrer: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut daily_stmt = match conn.prepare(
        "SELECT DATE(timestamp) as day, SUM(CASE WHEN event_type='view' THEN 1 ELSE 0 END) as views, SUM(CASE WHEN event_type='download' THEN 1 ELSE 0 END) as downloads FROM link_analytics WHERE share_token = ?1 AND timestamp >= datetime('now', '-30 days') GROUP BY day ORDER BY day",
    ) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to prepare daily breakdown query: {}", e);
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database query failed");
        }
    };
    let daily_breakdown: Vec<DailyCount> = daily_stmt
        .query_map(params![token], |row| {
            Ok(DailyCount {
                date: row.get(0)?,
                views: row.get(1)?,
                downloads: row.get(2)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    (
        StatusCode::OK,
        Json(serde_json::json!(LinkStats {
            token,
            path: link.path,
            total_views,
            total_downloads,
            unique_visitors,
            top_referrers,
            daily_breakdown,
        })),
    )
        .into_response()
}

/// GET /analytics/overview — global analytics overview.
pub async fn analytics_overview(State(state): State<AppState>) -> Response {
    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };

    let conn = db.lock().unwrap_or_else(|e| e.into_inner());

    let total_views: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM link_analytics WHERE event_type = 'view'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_downloads: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM link_analytics WHERE event_type = 'download'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_shares: u64 = conn
        .query_row("SELECT COUNT(*) FROM shares", [], |row| row.get(0))
        .unwrap_or(0);

    let mut top_stmt = match conn.prepare(
        "SELECT a.share_token, s.file_path, COUNT(*) as cnt FROM link_analytics a LEFT JOIN shares s ON a.share_token = s.token WHERE a.event_type = 'view' GROUP BY a.share_token ORDER BY cnt DESC LIMIT 10",
    ) {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to prepare top links query: {}", e);
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database query failed");
        }
    };
    let top_links: Vec<TopLink> = top_stmt
        .query_map([], |row| {
            Ok(TopLink {
                token: row.get(0)?,
                path: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                views: row.get(2)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let storage_used_bytes = state.used_bytes.load(std::sync::atomic::Ordering::Relaxed);

    (
        StatusCode::OK,
        Json(serde_json::json!(AnalyticsOverview {
            total_views,
            total_downloads,
            total_shares,
            top_links,
            storage_used_bytes,
        })),
    )
        .into_response()
}
