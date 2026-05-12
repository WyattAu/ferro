//! File versioning, diff, and LCS implementation for Ferro server.
//!
//! Provides axum route handlers for version CRUD operations and line-level diffs,
//! plus an [`auto_version`] helper for call-from-PUT-hook scenarios.

use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use common::storage::StorageEngine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Subset of server state required by versioning handlers.
#[derive(Clone)]
pub struct VersioningState {
    /// Root data directory (versions stored under `<data_dir>/versions/`).
    pub data_dir: Option<String>,
    /// Default author when none is provided.
    pub admin_user: Option<String>,
    /// Backing storage engine (used to read current file content).
    pub storage: Arc<dyn StorageEngine>,
    /// Maximum retained versions per file (0 = disabled).
    pub max_file_versions: u64,
}

// ---------------------------------------------------------------------------
// Error helper (mirrors server api_error for crate independence)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ApiError {
    error: String,
    error_code: String,
}

fn error_response(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    let body = axum::Json(ApiError {
        error: message.into(),
        error_code: code.to_string(),
    });
    (status, body).into_response()
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A stored file version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub id: u64,
    pub path: String,
    pub size: u64,
    pub content_hash: String,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub author: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionMeta {
    id: u64,
    path: String,
    size: u64,
    content_hash: String,
    modified_at: chrono::DateTime<chrono::Utc>,
    author: String,
    note: Option<String>,
    file_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct VersionResponse {
    pub id: u64,
    pub path: String,
    pub size: u64,
    pub content_hash: String,
    pub modified_at: String,
    pub author: String,
    pub note: Option<String>,
}

impl From<&VersionMeta> for VersionResponse {
    fn from(m: &VersionMeta) -> Self {
        Self {
            id: m.id,
            path: m.path.clone(),
            size: m.size,
            content_hash: m.content_hash.clone(),
            modified_at: m.modified_at.to_rfc3339(),
            author: m.author.clone(),
            note: m.note.clone(),
        }
    }
}

#[derive(Deserialize)]
pub struct DiffParams {
    pub from: String,
    pub to: String,
}

#[derive(Serialize)]
pub struct DiffResult {
    pub from_version: String,
    pub to_version: String,
    pub is_binary: bool,
    pub lines: Vec<DiffLine>,
    pub stats: DiffStats,
}

#[derive(Serialize)]
pub struct DiffLine {
    #[serde(rename = "type")]
    pub type_: String,
    pub content: String,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
}

#[derive(Serialize)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub unchanged: usize,
}

impl DiffResult {
    pub fn binary(from: String, to: String) -> Self {
        Self {
            from_version: from,
            to_version: to,
            is_binary: true,
            lines: vec![],
            stats: DiffStats {
                additions: 0,
                deletions: 0,
                unchanged: 0,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (no state dependency)
// ---------------------------------------------------------------------------

fn path_hash(path: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hex::encode(hasher.finalize())
}

fn compute_content_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn versions_dir_for(data_dir: &str, path: &str) -> PathBuf {
    let hash = path_hash(path);
    PathBuf::from(data_dir).join("versions").join(hash)
}

fn version_file_path(data_dir: &str, path: &str, version_id: u64) -> PathBuf {
    versions_dir_for(data_dir, path).join(format!("v{}", version_id))
}

fn meta_file_path(data_dir: &str, path: &str) -> PathBuf {
    versions_dir_for(data_dir, path).join("_meta.json")
}

fn read_meta(data_dir: &str, path: &str) -> Vec<VersionMeta> {
    let meta_path = meta_file_path(data_dir, path);
    let content = match std::fs::read_to_string(&meta_path) {
        Ok(c) => c,
        Err(e) => {
            warn!(
                "Failed to read versions metadata file {}: {}",
                meta_path.display(),
                e
            );
            return Vec::new();
        }
    };
    let metas: Vec<VersionMeta> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            warn!(
                "Failed to parse versions metadata for {}: {}",
                meta_path.display(),
                e
            );
            return Vec::new();
        }
    };
    metas
}

fn write_meta(data_dir: &str, path: &str, metas: &[VersionMeta]) -> Result<(), std::io::Error> {
    let dir = versions_dir_for(data_dir, path);
    std::fs::create_dir_all(&dir)?;
    let meta_path = meta_file_path(data_dir, path);
    let content =
        serde_json::to_string_pretty(metas).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
    std::fs::write(&meta_path, content)
}

fn next_version_id(metas: &[VersionMeta]) -> u64 {
    metas.iter().map(|m| m.id).max().unwrap_or(0) + 1
}

fn require_data_dir(state: &VersioningState) -> Result<String, axum::http::StatusCode> {
    match &state.data_dir {
        Some(d) => Ok(d.clone()),
        None => Err(StatusCode::BAD_REQUEST),
    }
}

fn author_or_anon(state: &VersioningState) -> String {
    state
        .admin_user
        .clone()
        .unwrap_or_else(|| "anonymous".to_string())
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/files/:path/versions — list file versions.
pub async fn list_versions(
    Extension(state): Extension<VersioningState>,
    Path(path): Path<String>,
) -> Response {
    let normalized = common::path::normalize_path(&path);
    let data_dir = match require_data_dir(&state) {
        Ok(d) => d,
        Err(status) => return error_response(status, "NO_DATA_DIR", "Versioning requires --data-dir"),
    };

    let metas = tokio::task::spawn_blocking(move || read_meta(&data_dir, &normalized))
        .await
        .unwrap_or_else(|_| Vec::new());

    let versions: Vec<VersionResponse> = metas.iter().map(VersionResponse::from).collect();
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "versions": versions })),
    )
        .into_response()
}

/// GET /api/files/:path/versions/:version_id — download a specific version.
pub async fn get_version(
    Extension(state): Extension<VersioningState>,
    Path((path, version_id)): Path<(String, u64)>,
) -> Response {
    let normalized = common::path::normalize_path(&path);
    let data_dir = match require_data_dir(&state) {
        Ok(d) => d,
        Err(status) => return error_response(status, "NO_DATA_DIR", "Versioning requires --data-dir"),
    };

    let result = tokio::task::spawn_blocking(move || {
        let metas = read_meta(&data_dir, &normalized);
        let meta = metas.iter().find(|m| m.id == version_id);
        match meta {
            Some(m) => {
                let content = std::fs::read(&m.file_path)?;
                Ok::<_, std::io::Error>((m.clone(), content))
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "version not found",
            )),
        }
    })
    .await;

    match result {
        Ok(Ok((_meta, content))) => {
            let body = axum::body::Body::from(content);
            (StatusCode::OK, body).into_response()
        }
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => error_response(
            StatusCode::NOT_FOUND,
            "VERSION_NOT_FOUND",
            "Version not found",
        ),
        Ok(Err(e)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "VERSION_READ_ERROR",
            format!("Failed to read version: {}", e),
        ),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "VERSION_ERROR",
            "Internal error",
        ),
    }
}

/// POST /api/files/:path/versions — create a new file version.
pub async fn create_version(
    Extension(state): Extension<VersioningState>,
    Path(path): Path<String>,
) -> Response {
    let normalized = common::path::normalize_path(&path);
    let data_dir = match require_data_dir(&state) {
        Ok(d) => d,
        Err(status) => return error_response(status, "NO_DATA_DIR", "Versioning requires --data-dir"),
    };

    let author = author_or_anon(&state);

    let content = match state.storage.get(&normalized).await {
        Ok(c) => c,
        Err(_) => return error_response(StatusCode::NOT_FOUND, "FILE_NOT_FOUND", "File not found"),
    };

    let max_versions = state.max_file_versions;
    let norm = normalized.clone();
    let dd = data_dir.clone();

    let result = tokio::task::spawn_blocking(move || {
        let content_hash = compute_content_hash(&content);
        let mut metas = read_meta(&dd, &norm);

        if let Some(existing) = metas.iter().find(|m| m.content_hash == content_hash) {
            return Ok::<(u64, String), String>((existing.id, content_hash));
        }

        let new_id = next_version_id(&metas);
        let file_path = version_file_path(&dd, &norm, new_id);
        let dir = file_path
            .parent()
            .ok_or_else(|| "file path has no parent".to_string())?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::write(&file_path, &content).map_err(|e| e.to_string())?;

        let meta = VersionMeta {
            id: new_id,
            path: norm.clone(),
            size: content.len() as u64,
            content_hash: content_hash.clone(),
            modified_at: chrono::Utc::now(),
            author: author.clone(),
            note: None,
            file_path,
        };

        metas.push(meta);
        metas.sort_by_key(|m| m.id);

        while metas.len() > max_versions as usize {
            if let Some(oldest) = metas.first() {
                let _ = std::fs::remove_file(&oldest.file_path);
            }
            metas.remove(0);
        }

        write_meta(&dd, &norm, &metas).map_err(|e| e.to_string())?;
        Ok((new_id, content_hash))
    })
    .await;

    match result {
        Ok(Ok((id, hash))) => (
            StatusCode::CREATED,
            axum::Json(serde_json::json!({
                "id": id,
                "content_hash": hash,
            })),
        )
            .into_response(),
        Ok(Err(e)) => error_response(StatusCode::INTERNAL_SERVER_ERROR, "VERSION_CREATE_ERROR", e),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "VERSION_ERROR",
            "Internal error",
        ),
    }
}

/// DELETE /api/files/:path/versions/:version_id — delete a specific version.
pub async fn delete_version(
    Extension(state): Extension<VersioningState>,
    Path((path, version_id)): Path<(String, u64)>,
) -> Response {
    let normalized = common::path::normalize_path(&path);
    let data_dir = match require_data_dir(&state) {
        Ok(d) => d,
        Err(status) => return error_response(status, "NO_DATA_DIR", "Versioning requires --data-dir"),
    };

    let result = tokio::task::spawn_blocking(move || {
        let mut metas = read_meta(&data_dir, &normalized);
        let idx = metas.iter().position(|m| m.id == version_id);
        match idx {
            Some(i) => {
                let removed = metas.remove(i);
                let _ = std::fs::remove_file(&removed.file_path);
                write_meta(&data_dir, &normalized, &metas)?;
                Ok::<_, std::io::Error>(())
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "version not found",
            )),
        }
    })
    .await;

    match result {
        Ok(Ok(())) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "ok": true })),
        )
            .into_response(),
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::NotFound => error_response(
            StatusCode::NOT_FOUND,
            "VERSION_NOT_FOUND",
            "Version not found",
        ),
        Ok(Err(e)) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "VERSION_DELETE_ERROR",
            format!("Failed to delete version: {}", e),
        ),
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "VERSION_ERROR",
            "Internal error",
        ),
    }
}

/// Automatically version a file before an overwrite (called by PUT handler).
pub async fn auto_version(state: &VersioningState, path: &str, previous_content: bytes::Bytes) {
    if state.max_file_versions == 0 {
        return;
    }
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => return,
    };
    let normalized = path.to_string();
    let author = author_or_anon(state);
    let max_versions = state.max_file_versions;

    let result = tokio::task::spawn_blocking(move || {
        let content_hash = compute_content_hash(&previous_content);
        let mut metas = read_meta(&data_dir, &normalized);

        if metas.iter().any(|m| m.content_hash == content_hash) {
            return;
        }

        let new_id = next_version_id(&metas);
        let file_path = version_file_path(&data_dir, &normalized, new_id);
        let Some(dir) = file_path.parent() else {
            warn!("Version file path has no parent: {:?}", file_path);
            return;
        };
        if let Err(e) = std::fs::create_dir_all(dir) {
            warn!("Failed to create version dir: {}", e);
            return;
        }
        if let Err(e) = std::fs::write(&file_path, &previous_content) {
            warn!("Failed to write version file: {}", e);
            return;
        }

        let meta = VersionMeta {
            id: new_id,
            path: normalized.clone(),
            size: previous_content.len() as u64,
            content_hash,
            modified_at: chrono::Utc::now(),
            author,
            note: None,
            file_path,
        };

        metas.push(meta);
        metas.sort_by_key(|m| m.id);

        while metas.len() > max_versions as usize {
            if let Some(oldest) = metas.first() {
                let _ = std::fs::remove_file(&oldest.file_path);
            }
            metas.remove(0);
        }

        if let Err(e) = write_meta(&data_dir, &normalized, &metas) {
            warn!("Failed to write version meta: {}", e);
        }
    })
    .await;

    if let Err(e) = result {
        warn!("Auto-versioning failed for {}: {:?}", path, e);
    }
}

/// GET /api/files/:path/diff — diff two versions of a file.
pub async fn diff_versions(
    Extension(state): Extension<VersioningState>,
    Path(path): Path<String>,
    Query(params): Query<DiffParams>,
) -> Response {
    let normalized = common::path::normalize_path(&path);
    let data_dir = match require_data_dir(&state) {
        Ok(d) => d,
        Err(status) => return error_response(status, "NO_DATA_DIR", "Versioning requires --data-dir"),
    };

    let from_id: u64 = match params.from.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_VERSION",
                "Invalid 'from' version ID",
            );
        }
    };
    let to_id: u64 = match params.to.parse() {
        Ok(id) => id,
        Err(_) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                "INVALID_VERSION",
                "Invalid 'to' version ID",
            );
        }
    };

    let norm = normalized.clone();
    let dd = data_dir.clone();
    let result = tokio::task::spawn_blocking(move || {
        let metas = read_meta(&dd, &norm);
        let v1 = metas.iter().find(|m| m.id == from_id).cloned();
        let v2 = metas.iter().find(|m| m.id == to_id).cloned();

        match (v1, v2) {
            (Some(m1), Some(m2)) => {
                let content1 = std::fs::read(&m1.file_path).unwrap_or_default();
                let content2 = std::fs::read(&m2.file_path).unwrap_or_default();
                Ok::<_, String>((content1, content2))
            }
            (None, _) => Err(format!("Version {} not found", from_id)),
            (_, None) => Err(format!("Version {} not found", to_id)),
        }
    })
    .await;

    match result {
        Ok(Ok((content1, content2))) => {
            let mime = mime_guess::from_path(&normalized)
                .first_or_octet_stream()
                .to_string();
            let is_text = mime.starts_with("text/")
                || mime == "application/json"
                || mime == "application/xml";

            let diff = if is_text {
                compute_line_diff(
                    &String::from_utf8_lossy(&content1),
                    &String::from_utf8_lossy(&content2),
                    from_id.to_string(),
                    to_id.to_string(),
                )
            } else {
                DiffResult::binary(from_id.to_string(), to_id.to_string())
            };

            (StatusCode::OK, axum::Json(diff)).into_response()
        }
        Ok(Err(msg)) => {
            if msg.contains("not found") {
                error_response(StatusCode::NOT_FOUND, "VERSION_NOT_FOUND", msg)
            } else {
                error_response(StatusCode::INTERNAL_SERVER_ERROR, "DIFF_ERROR", msg)
            }
        }
        Err(_) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DIFF_ERROR",
            "Internal error",
        ),
    }
}

// ---------------------------------------------------------------------------
// Diff algorithm
// ---------------------------------------------------------------------------

fn compute_line_diff(old: &str, new: &str, from_version: String, to_version: String) -> DiffResult {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let lcs = longest_common_subsequence(&old_lines, &new_lines);
    let mut diff_lines = Vec::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;
    let mut unchanged = 0usize;

    let mut oi = 0usize;
    let mut ni = 0usize;

    for (o_line, n_line) in &lcs {
        while oi < old_lines.len() && old_lines[oi] != *o_line {
            diff_lines.push(DiffLine {
                type_: "removed".to_string(),
                content: old_lines[oi].to_string(),
                old_line: Some(oi + 1),
                new_line: None,
            });
            deletions += 1;
            oi += 1;
        }
        while ni < new_lines.len() && new_lines[ni] != *n_line {
            diff_lines.push(DiffLine {
                type_: "added".to_string(),
                content: new_lines[ni].to_string(),
                old_line: None,
                new_line: Some(ni + 1),
            });
            additions += 1;
            ni += 1;
        }
        diff_lines.push(DiffLine {
            type_: "same".to_string(),
            content: o_line.to_string(),
            old_line: Some(oi + 1),
            new_line: Some(ni + 1),
        });
        unchanged += 1;
        oi += 1;
        ni += 1;
    }

    while oi < old_lines.len() {
        diff_lines.push(DiffLine {
            type_: "removed".to_string(),
            content: old_lines[oi].to_string(),
            old_line: Some(oi + 1),
            new_line: None,
        });
        deletions += 1;
        oi += 1;
    }
    while ni < new_lines.len() {
        diff_lines.push(DiffLine {
            type_: "added".to_string(),
            content: new_lines[ni].to_string(),
            old_line: None,
            new_line: Some(ni + 1),
        });
        additions += 1;
        ni += 1;
    }

    DiffResult {
        from_version,
        to_version,
        is_binary: false,
        lines: diff_lines,
        stats: DiffStats {
            additions,
            deletions,
            unchanged,
        },
    }
}

fn longest_common_subsequence<'a>(a: &'a [&'a str], b: &'a [&'a str]) -> Vec<(&'a str, &'a str)> {
    let m = a.len();
    let n = b.len();
    if m == 0 || n == 0 {
        return vec![];
    }

    let limit = 10000;
    let a_lim = &a[..m.min(limit)];
    let b_lim = &b[..n.min(limit)];
    let rows = a_lim.len();
    let cols = b_lim.len();

    let mut dp = vec![vec![0usize; cols + 1]; rows + 1];
    for i in 1..=rows {
        for j in 1..=cols {
            if a_lim[i - 1] == b_lim[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut result = Vec::new();
    let (mut i, mut j) = (rows, cols);
    while i > 0 && j > 0 {
        if a_lim[i - 1] == b_lim[j - 1] {
            result.push((a_lim[i - 1], b_lim[j - 1]));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }

    result.reverse();
    result
}

// ---------------------------------------------------------------------------
// Route builder
// ---------------------------------------------------------------------------

/// Build a router with all versioning routes.
/// Requires `Extension<VersioningState>` to be provided by the caller.
pub fn routes<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    axum::Router::new()
        .route(
            "/files/{path}/versions",
            axum::routing::get(list_versions).post(create_version),
        )
        .route(
            "/files/{path}/versions/{version_id}",
            axum::routing::get(get_version).delete(delete_version),
        )
        .route("/files/{path}/diff", axum::routing::get(diff_versions))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use ferro_core::storage::InMemoryStorageEngine;
    use http_body_util::BodyExt;
    use std::sync::Arc;

    fn versioned_state() -> (VersioningState, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().to_string_lossy().to_string();
        let state = VersioningState {
            data_dir: Some(data_dir.clone()),
            admin_user: Some("admin".to_string()),
            storage: Arc::new(InMemoryStorageEngine::new()),
            max_file_versions: 10,
        };
        (state, tmp)
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_create_version() {
        let (state, _tmp) = versioned_state();
        state
            .storage
            .put("/test.txt", bytes::Bytes::from("hello"), "test")
            .await
            .unwrap();

        let resp = create_version(Extension(state), Path("test.txt".to_string())).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp).await;
        assert!(json.get("id").is_some());
        assert!(json.get("content_hash").is_some());
    }

    #[tokio::test]
    async fn test_create_version_idempotent() {
        let (state, _tmp) = versioned_state();
        state
            .storage
            .put("/idem.txt", bytes::Bytes::from("same content"), "test")
            .await
            .unwrap();

        let resp1 = create_version(Extension(state.clone()), Path("idem.txt".to_string())).await;
        assert_eq!(resp1.status(), StatusCode::CREATED);
        let json1 = body_json(resp1).await;
        let id1 = json1["id"].as_u64().unwrap();

        let resp2 = create_version(Extension(state.clone()), Path("idem.txt".to_string())).await;
        assert_eq!(resp2.status(), StatusCode::CREATED);
        let json2 = body_json(resp2).await;
        let id2 = json2["id"].as_u64().unwrap();

        assert_eq!(id1, id2, "Same content should return same version id");
    }

    #[tokio::test]
    async fn test_list_versions() {
        let (state, _tmp) = versioned_state();
        state
            .storage
            .put("/list.txt", bytes::Bytes::from("v1"), "test")
            .await
            .unwrap();
        create_version(Extension(state.clone()), Path("list.txt".to_string())).await;

        state
            .storage
            .put("/list.txt", bytes::Bytes::from("v2"), "test")
            .await
            .unwrap();
        create_version(Extension(state.clone()), Path("list.txt".to_string())).await;

        let resp = list_versions(Extension(state.clone()), Path("list.txt".to_string())).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["versions"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_max_versions_eviction() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().to_string_lossy().to_string();
        let state = VersioningState {
            data_dir: Some(data_dir.clone()),
            admin_user: Some("admin".to_string()),
            storage: Arc::new(InMemoryStorageEngine::new()),
            max_file_versions: 2,
        };

        state
            .storage
            .put("/evict.txt", bytes::Bytes::from("v1"), "test")
            .await
            .unwrap();
        create_version(Extension(state.clone()), Path("evict.txt".to_string())).await;

        state
            .storage
            .put("/evict.txt", bytes::Bytes::from("v2"), "test")
            .await
            .unwrap();
        create_version(Extension(state.clone()), Path("evict.txt".to_string())).await;

        state
            .storage
            .put("/evict.txt", bytes::Bytes::from("v3"), "test")
            .await
            .unwrap();
        create_version(Extension(state.clone()), Path("evict.txt".to_string())).await;

        let resp = list_versions(Extension(state.clone()), Path("evict.txt".to_string())).await;
        let json = body_json(resp).await;
        assert_eq!(
            json["versions"].as_array().unwrap().len(),
            2,
            "Should evict oldest version"
        );
    }

    #[tokio::test]
    async fn test_delete_version() {
        let (state, _tmp) = versioned_state();
        state
            .storage
            .put("/del.txt", bytes::Bytes::from("content"), "test")
            .await
            .unwrap();
        let resp = create_version(Extension(state.clone()), Path("del.txt".to_string())).await;
        let json = body_json(resp).await;
        let id = json["id"].as_u64().unwrap();

        let resp =
            delete_version(Extension(state.clone()), Path(("del.txt".to_string(), id))).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = list_versions(Extension(state.clone()), Path("del.txt".to_string())).await;
        let json = body_json(resp).await;
        assert_eq!(json["versions"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_version_nonexistent_file() {
        let (state, _tmp) = versioned_state();
        let resp = create_version(Extension(state), Path("nope.txt".to_string())).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_auto_version_called() {
        let (state, _tmp) = versioned_state();
        state
            .storage
            .put("/auto.txt", bytes::Bytes::from("original"), "test")
            .await
            .unwrap();

        let prev = bytes::Bytes::from("original");
        auto_version(&state, "/auto.txt", prev).await;

        let resp = list_versions(Extension(state.clone()), Path("auto.txt".to_string())).await;
        let json = body_json(resp).await;
        assert_eq!(json["versions"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_version_no_data_dir() {
        let state = VersioningState {
            data_dir: None,
            admin_user: Some("admin".to_string()),
            storage: Arc::new(InMemoryStorageEngine::new()),
            max_file_versions: 10,
        };
        state
            .storage
            .put("/no-dir.txt", bytes::Bytes::from("x"), "test")
            .await
            .unwrap();

        let resp = create_version(Extension(state), Path("no-dir.txt".to_string())).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_diff_versions() {
        let (state, _tmp) = versioned_state();
        state
            .storage
            .put("/diff.txt", bytes::Bytes::from("hello\nworld\n"), "test")
            .await
            .unwrap();

        let resp = create_version(Extension(state.clone()), Path("diff.txt".to_string())).await;
        let json = body_json(resp).await;
        let id1 = json["id"].as_u64().unwrap();

        state
            .storage
            .put(
                "/diff.txt",
                bytes::Bytes::from("hello\nferro\nworld\n"),
                "test",
            )
            .await
            .unwrap();
        let resp = create_version(Extension(state.clone()), Path("diff.txt".to_string())).await;
        let json = body_json(resp).await;
        let id2 = json["id"].as_u64().unwrap();

        let resp = diff_versions(
            Extension(state.clone()),
            Path("diff.txt".to_string()),
            Query(DiffParams {
                from: id1.to_string(),
                to: id2.to_string(),
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["from_version"], id1.to_string());
        assert_eq!(json["to_version"], id2.to_string());
        assert!(!json["is_binary"].as_bool().unwrap());
        assert!(json["stats"]["additions"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_diff_version_not_found() {
        let (state, _tmp) = versioned_state();

        let resp = diff_versions(
            Extension(state.clone()),
            Path("nope.txt".to_string()),
            Query(DiffParams {
                from: "1".to_string(),
                to: "2".to_string(),
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_diff_binary_file() {
        let (state, _tmp) = versioned_state();
        state
            .storage
            .put(
                "/img.png",
                bytes::Bytes::from_static(b"\x89PNG\r\n\x1a\n"),
                "test",
            )
            .await
            .unwrap();

        let resp = create_version(Extension(state.clone()), Path("img.png".to_string())).await;
        let json = body_json(resp).await;
        let id1 = json["id"].as_u64().unwrap();

        state
            .storage
            .put(
                "/img.png",
                bytes::Bytes::from_static(b"\x89PNG\r\n\x1a\n\x00\x00"),
                "test",
            )
            .await
            .unwrap();
        let resp = create_version(Extension(state.clone()), Path("img.png".to_string())).await;
        let json = body_json(resp).await;
        let id2 = json["id"].as_u64().unwrap();

        let resp = diff_versions(
            Extension(state.clone()),
            Path("img.png".to_string()),
            Query(DiffParams {
                from: id1.to_string(),
                to: id2.to_string(),
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert!(json["is_binary"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_diff_invalid_version_id() {
        let (state, _tmp) = versioned_state();

        let resp = diff_versions(
            Extension(state.clone()),
            Path("test.txt".to_string()),
            Query(DiffParams {
                from: "abc".to_string(),
                to: "2".to_string(),
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_diff_identical_versions() {
        let (state, _tmp) = versioned_state();
        state
            .storage
            .put("/same.txt", bytes::Bytes::from("unchanged"), "test")
            .await
            .unwrap();

        let resp = create_version(Extension(state.clone()), Path("same.txt".to_string())).await;
        let json = body_json(resp).await;
        let id = json["id"].as_u64().unwrap();

        let resp = diff_versions(
            Extension(state.clone()),
            Path("same.txt".to_string()),
            Query(DiffParams {
                from: id.to_string(),
                to: id.to_string(),
            }),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["stats"]["additions"], 0);
        assert_eq!(json["stats"]["deletions"], 0);
        assert!(json["stats"]["unchanged"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_compute_line_diff_additions_and_removals() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline2.5\nline3\n";
        let result = compute_line_diff(old, new, "1".to_string(), "2".to_string());

        assert_eq!(result.stats.additions, 1);
        assert_eq!(result.stats.deletions, 1);
        assert_eq!(result.stats.unchanged, 2);

        let added: Vec<_> = result.lines.iter().filter(|l| l.type_ == "added").collect();
        assert_eq!(added.len(), 1);
        assert_eq!(added[0].content, "line2.5");

        let removed: Vec<_> = result
            .lines
            .iter()
            .filter(|l| l.type_ == "removed")
            .collect();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].content, "line2");
    }

    #[test]
    fn test_compute_line_diff_empty_files() {
        let result = compute_line_diff("", "", "1".to_string(), "2".to_string());
        assert_eq!(result.stats.additions, 0);
        assert_eq!(result.stats.deletions, 0);
        assert_eq!(result.stats.unchanged, 0);
        assert!(result.lines.is_empty());
    }

    #[test]
    fn test_lcs_basic() {
        let a: Vec<&str> = vec!["a", "b", "c"];
        let b: Vec<&str> = vec!["a", "b", "c"];
        let lcs = longest_common_subsequence(&a, &b);
        assert_eq!(lcs.len(), 3);
    }

    #[test]
    fn test_lcs_with_changes() {
        let a: Vec<&str> = vec!["a", "b", "c", "d"];
        let b: Vec<&str> = vec!["a", "x", "c", "d"];
        let lcs = longest_common_subsequence(&a, &b);
        assert_eq!(lcs.len(), 3);
    }
}
