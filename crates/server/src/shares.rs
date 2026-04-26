use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{Duration, Utc};

use crate::api_error::ApiError;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    pub token: String,
    pub path: String,
    pub password: Option<String>,
    pub expires_at: chrono::DateTime<Utc>,
    pub max_downloads: Option<u32>,
    pub download_count: u32,
    pub created_by: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    pub path: String,
    pub password: Option<String>,
    pub expires_in_hours: Option<i64>,
    pub max_downloads: Option<u32>,
}

pub struct ShareStore {
    links: Arc<RwLock<Vec<ShareLink>>>,
}

impl ShareStore {
    pub fn new() -> Self {
        Self {
            links: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn create(&self, req: CreateShareRequest, created_by: String) -> ShareLink {
        let token = uuid::Uuid::new_v4().to_string();
        let expires_at = match req.expires_in_hours {
            Some(hours) => Utc::now() + Duration::hours(hours),
            None => Utc::now() + Duration::days(7),
        };
        let link = ShareLink {
            token: token.clone(),
            path: req.path,
            password: req.password,
            expires_at,
            max_downloads: req.max_downloads,
            download_count: 0,
            created_by,
        };
        self.links.write().await.push(link.clone());
        link
    }

    pub async fn get(&self, token: &str) -> Option<ShareLink> {
        let links = self.links.read().await;
        links.iter().find(|l| l.token == token).cloned()
    }

    pub async fn delete(&self, token: &str) -> bool {
        let mut links = self.links.write().await;
        if let Some(pos) = links.iter().position(|l| l.token == token) {
            links.remove(pos);
            true
        } else {
            false
        }
    }

    pub async fn list(&self) -> Vec<ShareLink> {
        let links = self.links.read().await;
        links.iter().filter(|l| l.expires_at > Utc::now()).cloned().collect()
    }

    pub async fn increment_download(&self, token: &str) -> bool {
        let mut links = self.links.write().await;
        if let Some(link) = links.iter_mut().find(|l| l.token == token) {
            link.download_count += 1;
            true
        } else {
            false
        }
    }
}

impl Default for ShareStore {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn create_share(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateShareRequest>,
) -> Response {
    let link = state.share_store.create(req, "anonymous".to_string()).await;
    (StatusCode::CREATED, axum::Json(serde_json::json!({
        "token": link.token,
        "url": format!("/s/{}", link.token),
        "path": link.path,
        "expires_at": link.expires_at.to_rfc3339(),
        "max_downloads": link.max_downloads,
    }))).into_response()
}

pub async fn list_shares(State(state): State<AppState>) -> Response {
    let links: Vec<ShareLink> = state.share_store.list().await;
    let items: Vec<serde_json::Value> = links.iter().map(|l| {
        serde_json::json!({
            "token": l.token,
            "url": format!("/s/{}", l.token),
            "path": l.path,
            "expires_at": l.expires_at.to_rfc3339(),
            "max_downloads": l.max_downloads,
            "download_count": l.download_count,
            "created_by": l.created_by,
        })
    }).collect();
    (StatusCode::OK, axum::Json(serde_json::json!({ "shares": items }))).into_response()
}

pub async fn delete_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Response {
    if state.share_store.delete(&token).await {
        (StatusCode::NO_CONTENT, "").into_response()
    } else {
        ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found")
    }
}

pub async fn serve_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    let link = match state.share_store.get(&token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    };

    if link.expires_at < Utc::now() {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Share expired");
    }

    if let Some(max) = link.max_downloads
        && link.download_count >= max
    {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Download limit reached");
    }

    // Check password if set
    if let Some(ref required_password) = link.password {
        let provided_password = params.get("password").map(|s| s.as_str());
        match provided_password {
            Some(pw) if constant_time_eq(pw, required_password) => {}
            Some(_) => {
                return ApiError::unauthorized(ApiError::SHARE_PASSWORD_INVALID, "Invalid password");
            }
            None => {
                return ApiError::with_details(
                    StatusCode::UNAUTHORIZED,
                    ApiError::SHARE_PASSWORD_REQUIRED,
                    "Password required",
                    "true",
                );
            }
        }
    }

    let content: Bytes = match state.storage.get(&link.path).await {
        Ok(c) => c,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    let meta: common::metadata::FileMetadata = match state.storage.head(&link.path).await {
        Ok(m) => m,
        Err(_) => common::metadata::FileMetadata::new(
            link.path.clone(),
            common::metadata::ContentHash::new("0".repeat(64)),
            content.len() as u64,
            "anonymous".to_string(),
        ),
    };

    state.share_store.increment_download(&token).await;

    let mut headers = axum::http::HeaderMap::new();
    headers.insert("Content-Type", axum::http::HeaderValue::from_str(&meta.mime_type).unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")));
    headers.insert("Content-Disposition", axum::http::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", link.path.rsplit('/').next().unwrap_or("download"))).unwrap());
    (StatusCode::OK, headers, axum::body::Body::from(content)).into_response()
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
