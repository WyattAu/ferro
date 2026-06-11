use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use ferro_server::AppState;
use ferro_server::api_error::ApiError;

/// Manifest describing a backup's contents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub id: String,
    pub created_at: String,
    pub files: Vec<BackupEntry>,
    pub cas_blobs: Vec<CasBlobEntry>,
    pub metadata_snapshot: MetadataSnapshot,
    pub total_bytes: u64,
}

/// A single file entry within a backup manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub path: String,
    pub size: u64,
    pub etag: String,
    pub content_hash: String,
    /// SHA-256 checksum computed from the actual backup file bytes.
    pub sha256: String,
}

/// A CAS blob entry within a backup manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasBlobEntry {
    pub hash: String,
    pub size: u64,
}

/// Snapshot of server metadata at backup time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSnapshot {
    pub file_count: u64,
    pub total_bytes: u64,
    pub cas_blob_count: usize,
    pub db_checkpoint: bool,
    pub server_version: String,
}

/// Summary info returned when listing or creating backups.
#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub id: String,
    pub created_at: String,
    pub files: usize,
    pub bytes: u64,
}

/// Request body for restoring a backup.
#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub backup_id: String,
}

/// Report returned after restoring from a backup archive.
#[derive(Debug, Serialize)]
pub struct RestoreReport {
    pub backup_id: String,
    pub files_restored: usize,
    pub files_skipped: usize,
    pub files_failed: usize,
    pub total_files: usize,
    pub cas_blobs_restored: usize,
    pub integrity_verified: bool,
    pub errors: Vec<String>,
}

/// POST /api/admin/backup — create a new backup.
///
/// Performs a WAL checkpoint, lists all CAS blobs, creates a manifest with
/// SHA-256 checksums for each file, and optionally backs up the SQLite
/// database via VACUUM INTO.
pub async fn create_backup(State(state): State<AppState>) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return ApiError::bad_request(
                ApiError::BAD_REQUEST,
                "Backups require --data-dir to be set",
            );
        }
    };

    let entries = match state.storage.list_all("/", 10000).await {
        Ok(e) => e,
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                format!("Failed to list files: {}", e),
            );
        }
    };

    let now = chrono::Utc::now();
    let backup_id = format!("backup-{}", now.format("%Y%m%d-%H%M%S"));
    let backup_dir = std::path::Path::new(&data_dir)
        .join("backups")
        .join(&backup_id);

    if let Err(e) = std::fs::create_dir_all(&backup_dir) {
        return ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to create backup directory: {}", e),
        );
    }

    let mut db_checkpointed = false;

    if let Some(ref db) = state.db
        && let Ok(conn) = db.lock()
    {
        if conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .is_ok()
        {
            tracing::info!("SQLite WAL checkpoint completed before backup");
        }

        let db_backup_path = backup_dir.join("ferro.db");
        match conn.execute_batch(&format!(
            "VACUUM INTO '{}';",
            db_backup_path.to_string_lossy()
        )) {
            Ok(()) => {
                tracing::info!("SQLite database backed up to {}", db_backup_path.display());
                db_checkpointed = true;
            }
            Err(e) => {
                tracing::warn!("Failed to back up SQLite database: {}", e);
            }
        }
    }

    let mut cas_blobs = Vec::new();
    if let Some(ref cas_store) = state.cas_store {
        if let Ok(all_files) = state.storage.list_all("/", 10000).await {
            let mut seen_hashes = std::collections::HashSet::new();
            for meta in &all_files {
                if meta.is_collection {
                    continue;
                }
                let hash = meta.content_hash.as_str().to_string();
                if seen_hashes.insert(hash.clone()) {
                    cas_blobs.push(CasBlobEntry {
                        hash,
                        size: meta.size,
                    });
                }
            }
        }
        let cas_count = cas_store.content_count().await;
        tracing::info!(
            "CAS blob listing: {} unique hashes from files, {} total in CAS store",
            cas_blobs.len(),
            cas_count,
        );
    }

    let mut file_count = 0u64;
    let mut total_bytes = 0u64;
    for meta in &entries {
        if !meta.is_collection {
            file_count += 1;
            total_bytes += meta.size;
        }
    }

    let mut manifest = BackupManifest {
        id: backup_id.clone(),
        created_at: now.to_rfc3339(),
        files: Vec::new(),
        cas_blobs,
        metadata_snapshot: MetadataSnapshot {
            file_count,
            total_bytes,
            cas_blob_count: manifest_cas_count(&state).await,
            db_checkpoint: db_checkpointed,
            server_version: env!("CARGO_PKG_VERSION").to_string(),
        },
        total_bytes: 0,
    };

    for meta in &entries {
        if meta.is_collection {
            continue;
        }

        match state.storage.get(&meta.path).await {
            Ok(content) => {
                let safe_path = meta.path.trim_start_matches('/').replace('/', "_");
                let file_path = backup_dir.join(&safe_path);

                if let Err(e) = ferro_core::fs_util::atomic_write(&file_path, &content) {
                    tracing::warn!("Failed to backup file {}: {}", meta.path, e);
                    continue;
                }

                let sha256 = hex::encode(Sha256::digest(&content));
                manifest.total_bytes += meta.size;
                manifest.files.push(BackupEntry {
                    path: meta.path.clone(),
                    size: meta.size,
                    etag: meta.etag.clone(),
                    content_hash: meta.content_hash.as_str().to_string(),
                    sha256,
                });
            }
            Err(e) => {
                tracing::warn!("Failed to read file {} for backup: {}", meta.path, e);
            }
        }
    }

    let manifest_path = backup_dir.join("manifest.json");
    match serde_json::to_string_pretty(&manifest) {
        Ok(json) => {
            if let Err(e) = ferro_core::fs_util::atomic_write(&manifest_path, json.as_bytes()) {
                return ApiError::internal(
                    ApiError::INTERNAL_ERROR,
                    format!("Failed to write manifest: {}", e),
                );
            }
        }
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                format!("Failed to serialize manifest: {}", e),
            );
        }
    }

    let info = BackupInfo {
        id: backup_id,
        created_at: now.to_rfc3339(),
        files: manifest.files.len(),
        bytes: manifest.total_bytes,
    };

    (StatusCode::CREATED, axum::Json(info)).into_response()
}

async fn manifest_cas_count(state: &AppState) -> usize {
    match &state.cas_store {
        Some(cas) => cas.content_count().await,
        None => 0,
    }
}

/// GET /api/admin/backup/latest — get the latest backup manifest.
pub async fn get_latest_backup(State(state): State<AppState>) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return ApiError::not_found(ApiError::NOT_FOUND, "No backups available");
        }
    };

    let latest = match find_latest_manifest(&data_dir) {
        Some(m) => m,
        None => {
            return ApiError::not_found(ApiError::NOT_FOUND, "No backups found");
        }
    };

    (StatusCode::OK, axum::Json(latest)).into_response()
}

/// GET /api/admin/backup/download — download latest backup as a zip archive.
pub async fn download_backup(State(state): State<AppState>) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return ApiError::not_found(ApiError::NOT_FOUND, "No backups available");
        }
    };

    let manifest = match find_latest_manifest(&data_dir) {
        Some(m) => m,
        None => {
            return ApiError::not_found(ApiError::NOT_FOUND, "No backups found");
        }
    };

    let backup_dir = std::path::Path::new(&data_dir)
        .join("backups")
        .join(&manifest.id);

    match build_archive(&backup_dir, &manifest) {
        Ok(bytes) => (
            StatusCode::OK,
            [
                ("content-type", "application/zip"),
                (
                    "content-disposition",
                    &format!("attachment; filename=\"{}.zip\"", manifest.id),
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(e) => ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to create backup archive: {}", e),
        ),
    }
}

/// POST /api/admin/backup/restore — restore from an uploaded backup archive.
///
/// Accepts a zip archive containing a manifest and file data. Validates
/// SHA-256 checksums and restores files to storage.
pub async fn restore_from_archive(
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Response {
    let manifest: BackupManifest = match extract_manifest_from_archive(&body) {
        Ok(m) => m,
        Err(e) => {
            return ApiError::bad_request(
                ApiError::BAD_REQUEST,
                format!("Invalid backup archive: {}", e),
            );
        }
    };

    let mut report = RestoreReport {
        backup_id: manifest.id.clone(),
        files_restored: 0,
        files_skipped: 0,
        files_failed: 0,
        total_files: manifest.files.len(),
        cas_blobs_restored: 0,
        integrity_verified: true,
        errors: Vec::new(),
    };

    let archive_files = match extract_all_from_archive(&body) {
        Ok(f) => f,
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                format!("Failed to read backup archive: {}", e),
            );
        }
    };

    for entry in &manifest.files {
        let archive_key = entry.path.trim_start_matches('/').replace('/', "_");

        let already_exists = state.storage.exists(&entry.path).await.unwrap_or(false);
        if already_exists {
            report.files_skipped += 1;
            continue;
        }

        let Some(content) = archive_files.get(&archive_key) else {
            report.files_failed += 1;
            report
                .errors
                .push(format!("Missing file data for {}", entry.path));
            report.integrity_verified = false;
            continue;
        };

        let computed = hex::encode(Sha256::digest(content));
        if computed != entry.sha256 {
            report.files_failed += 1;
            report.integrity_verified = false;
            report.errors.push(format!(
                "Checksum mismatch for {}: expected {}, got {}",
                entry.path, entry.sha256, computed
            ));
            continue;
        }

        match state
            .storage
            .put(
                &entry.path,
                bytes::Bytes::from(content.clone()),
                "backup-restore",
            )
            .await
        {
            Ok(_) => report.files_restored += 1,
            Err(e) => {
                report.files_failed += 1;
                report
                    .errors
                    .push(format!("Failed to restore {}: {}", entry.path, e));
            }
        }
    }

    (StatusCode::OK, axum::Json(report)).into_response()
}

/// GET /api/admin/backups — list available backups.
pub async fn list_backups(State(state): State<AppState>) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return (StatusCode::OK, axum::Json(serde_json::json!([]))).into_response();
        }
    };

    let backups_dir = std::path::Path::new(&data_dir).join("backups");
    let mut backups: Vec<BackupInfo> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&backups_dir) {
        for entry in entries.flatten() {
            let manifest_path = entry.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            match std::fs::read_to_string(&manifest_path) {
                Ok(content) => match serde_json::from_str::<BackupManifest>(&content) {
                    Ok(manifest) => {
                        backups.push(BackupInfo {
                            id: manifest.id,
                            created_at: manifest.created_at,
                            files: manifest.files.len(),
                            bytes: manifest.total_bytes,
                        });
                    }
                    Err(_) => continue,
                },
                Err(_) => continue,
            }
        }
    }

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    (StatusCode::OK, axum::Json(backups)).into_response()
}

/// POST /api/admin/restore — restore from a backup (by backup_id).
pub async fn restore_backup(
    State(state): State<AppState>,
    axum::Json(input): axum::Json<RestoreRequest>,
) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return ApiError::bad_request(
                ApiError::BAD_REQUEST,
                "Restore requires --data-dir to be set",
            );
        }
    };

    let backup_dir = std::path::Path::new(&data_dir)
        .join("backups")
        .join(&input.backup_id);

    let manifest_path = backup_dir.join("manifest.json");
    let manifest_content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => {
            return ApiError::not_found(ApiError::NOT_FOUND, "Backup not found");
        }
    };

    let manifest: BackupManifest = match serde_json::from_str(&manifest_content) {
        Ok(m) => m,
        Err(_) => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to parse backup manifest");
        }
    };

    let mut restored_files = 0u64;

    for entry in &manifest.files {
        let already_exists = state.storage.exists(&entry.path).await.unwrap_or(false);
        if already_exists {
            restored_files += 1;
            continue;
        }

        let safe_path = entry.path.trim_start_matches('/').replace('/', "_");
        let file_path = backup_dir.join(&safe_path);

        match std::fs::read(&file_path) {
            Ok(content) => {
                let computed = hex::encode(Sha256::digest(&content));
                if computed != entry.sha256 {
                    tracing::warn!(
                        "Checksum mismatch for {}: expected {}, got {}",
                        entry.path,
                        entry.sha256,
                        computed
                    );
                    continue;
                }

                if let Err(e) = state
                    .storage
                    .put(&entry.path, bytes::Bytes::from(content), "backup-restore")
                    .await
                {
                    tracing::warn!("Failed to restore {}: {}", entry.path, e);
                } else {
                    restored_files += 1;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to read backup file for {}: {}", entry.path, e);
            }
        }
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "restored_files": restored_files,
            "total_files": manifest.files.len(),
            "backup_id": input.backup_id,
        })),
    )
        .into_response()
}

/// Result of checking a single file's integrity.
#[derive(Debug, Serialize)]
pub struct IntegrityCheckResult {
    pub path: String,
    pub status: IntegrityStatus,
    pub stored_hash: String,
    pub computed_hash: String,
}

/// Integrity status for a single file.
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityStatus {
    Ok,
    Mismatch,
    Unreadable,
    InvalidHash,
}

/// Summary of an integrity audit run.
#[derive(Debug, Serialize)]
pub struct IntegrityAuditReport {
    pub scanned_at: String,
    pub total_files: usize,
    pub ok: usize,
    pub mismatches: usize,
    pub unreadable: usize,
    pub invalid_hashes: usize,
    pub findings: Vec<IntegrityCheckResult>,
}

/// GET /api/admin/integrity — audit all stored files for hash integrity.
pub async fn audit_integrity(State(state): State<AppState>) -> Response {
    let entries = match state.storage.list_all("/", 10000).await {
        Ok(e) => e,
        Err(e) => {
            return ApiError::internal(
                ApiError::INTERNAL_ERROR,
                format!("Failed to list files for integrity audit: {}", e),
            );
        }
    };

    let mut report = IntegrityAuditReport {
        scanned_at: chrono::Utc::now().to_rfc3339(),
        total_files: 0,
        ok: 0,
        mismatches: 0,
        unreadable: 0,
        invalid_hashes: 0,
        findings: Vec::new(),
    };

    for meta in &entries {
        if meta.is_collection {
            continue;
        }

        report.total_files += 1;
        let stored_hash = meta.content_hash.as_str().to_string();

        if stored_hash.len() != 64 || stored_hash.chars().any(|c| !c.is_ascii_hexdigit()) {
            report.invalid_hashes += 1;
            report.findings.push(IntegrityCheckResult {
                path: meta.path.clone(),
                status: IntegrityStatus::InvalidHash,
                stored_hash,
                computed_hash: String::new(),
            });
            continue;
        }

        match state.storage.get(&meta.path).await {
            Ok(content) => {
                let computed = hex::encode(Sha256::digest(&content));

                if computed == stored_hash {
                    report.ok += 1;
                } else {
                    report.mismatches += 1;
                    report.findings.push(IntegrityCheckResult {
                        path: meta.path.clone(),
                        status: IntegrityStatus::Mismatch,
                        stored_hash,
                        computed_hash: computed,
                    });
                }
            }
            Err(_) => {
                report.unreadable += 1;
                report.findings.push(IntegrityCheckResult {
                    path: meta.path.clone(),
                    status: IntegrityStatus::Unreadable,
                    stored_hash,
                    computed_hash: String::new(),
                });
            }
        }
    }

    (StatusCode::OK, axum::Json(report)).into_response()
}

/// GET /api/admin/audit-chain — verify audit log chain hash integrity.
pub async fn audit_chain_verify(State(state): State<AppState>) -> Response {
    match state.audit_log.verify_chain().await {
        Some(report) => (StatusCode::OK, axum::Json(report)).into_response(),
        None => ApiError::internal(
            ApiError::INTERNAL_ERROR,
            "Audit chain verification requires SQLite persistence to be configured".to_string(),
        ),
    }
}

/// DELETE /api/admin/backup/:id — delete a backup.
pub async fn delete_backup(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let data_dir = match &state.data_dir {
        Some(d) => d.clone(),
        None => {
            return ApiError::bad_request(
                ApiError::BAD_REQUEST,
                "Delete requires --data-dir to be set",
            );
        }
    };

    let backup_dir = std::path::Path::new(&data_dir).join("backups").join(&id);

    if !backup_dir.exists() {
        return ApiError::not_found(ApiError::NOT_FOUND, "Backup not found");
    }

    if let Err(e) = std::fs::remove_dir_all(&backup_dir) {
        return ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to delete backup: {}", e),
        );
    }

    (StatusCode::NO_CONTENT, "").into_response()
}

fn find_latest_manifest(data_dir: &str) -> Option<BackupManifest> {
    let backups_dir = std::path::Path::new(data_dir).join("backups");
    let mut latest: Option<(String, BackupManifest)> = None;

    if let Ok(entries) = std::fs::read_dir(&backups_dir) {
        for entry in entries.flatten() {
            let manifest_path = entry.path().join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&manifest_path)
                && let Ok(manifest) = serde_json::from_str::<BackupManifest>(&content)
            {
                match &latest {
                    Some((current_ts, _)) if &manifest.created_at <= current_ts => {}
                    _ => {
                        latest = Some((manifest.created_at.clone(), manifest));
                    }
                }
            }
        }
    }

    latest.map(|(_, m)| m)
}

fn build_archive(
    backup_dir: &std::path::Path,
    manifest: &BackupManifest,
) -> std::io::Result<Vec<u8>> {
    use std::io::Write;

    let buf = std::io::Cursor::new(Vec::new());
    let mut writer = zip::ZipWriter::new(buf);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let manifest_json = serde_json::to_string_pretty(manifest)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    writer.start_file("manifest.json", options)?;
    writer.write_all(manifest_json.as_bytes())?;

    for entry in &manifest.files {
        let safe_path = entry.path.trim_start_matches('/').replace('/', "_");
        let file_path = backup_dir.join(&safe_path);
        if file_path.exists() {
            let data = std::fs::read(&file_path)?;
            writer.start_file(&safe_path, options)?;
            writer.write_all(&data)?;
        }
    }

    let db_path = backup_dir.join("ferro.db");
    if db_path.exists() {
        let data = std::fs::read(&db_path)?;
        writer.start_file("ferro.db", options)?;
        writer.write_all(&data)?;
    }

    let result = writer.finish()?;
    Ok(result.into_inner())
}

fn extract_manifest_from_archive(data: &[u8]) -> std::io::Result<BackupManifest> {
    use std::io::Read;

    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;

    let mut manifest_file = archive.by_name("manifest.json")?;
    let mut manifest_str = String::new();
    manifest_file.read_to_string(&mut manifest_str)?;

    serde_json::from_str(&manifest_str).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid manifest: {}", e),
        )
    })
}

fn extract_all_from_archive(
    data: &[u8],
) -> std::io::Result<std::collections::HashMap<String, Vec<u8>>> {
    use std::io::Read;

    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut files = std::collections::HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name == "manifest.json" || name == "ferro.db" {
            continue;
        }
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        files.insert(name, buf);
    }

    Ok(files)
}

pub fn compute_sha256(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_server::AppState;
    use ferro_server::build_router;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn backup_test_app() -> (axum::Router, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().to_string_lossy().to_string();
        let state = AppState::in_memory().with_data_dir(data_dir);
        (build_router(state), dir)
    }

    #[test]
    fn test_compute_sha256_known_input() {
        let data = b"hello world";
        let hash = compute_sha256(data);
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_sha256_empty() {
        let hash = compute_sha256(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_backup_manifest_serialization() {
        let manifest = BackupManifest {
            id: "backup-20260101-000000".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            files: vec![BackupEntry {
                path: "/test/file.txt".to_string(),
                size: 12,
                etag: "\"abc123\"".to_string(),
                content_hash: "a".repeat(64),
                sha256: "b".repeat(64),
            }],
            cas_blobs: vec![CasBlobEntry {
                hash: "c".repeat(64),
                size: 100,
            }],
            metadata_snapshot: MetadataSnapshot {
                file_count: 1,
                total_bytes: 12,
                cas_blob_count: 1,
                db_checkpoint: true,
                server_version: "1.0.0".to_string(),
            },
            total_bytes: 12,
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: BackupManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, manifest.id);
        assert_eq!(deserialized.files.len(), 1);
        assert_eq!(deserialized.files[0].sha256, "b".repeat(64));
        assert_eq!(deserialized.cas_blobs.len(), 1);
        assert_eq!(deserialized.metadata_snapshot.db_checkpoint, true);
    }

    #[test]
    fn test_backup_entry_sha256_field() {
        let entry = BackupEntry {
            path: "/foo/bar.txt".to_string(),
            size: 5,
            etag: "\"etag\"".to_string(),
            content_hash: "a".repeat(64),
            sha256: compute_sha256(b"hello"),
        };
        assert_eq!(
            entry.sha256,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_restore_report_serialization() {
        let report = RestoreReport {
            backup_id: "backup-test".to_string(),
            files_restored: 5,
            files_skipped: 2,
            files_failed: 1,
            total_files: 8,
            cas_blobs_restored: 3,
            integrity_verified: true,
            errors: vec!["file X read error".to_string()],
        };
        let json = serde_json::to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["files_restored"], 5);
        assert_eq!(parsed["files_failed"], 1);
        assert_eq!(parsed["integrity_verified"], true);
        assert_eq!(parsed["errors"][0], "file X read error");
    }

    #[test]
    fn test_metadata_snapshot_defaults() {
        let snapshot = MetadataSnapshot {
            file_count: 0,
            total_bytes: 0,
            cas_blob_count: 0,
            db_checkpoint: false,
            server_version: "0.1.0".to_string(),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["file_count"], 0);
        assert_eq!(parsed["db_checkpoint"], false);
    }

    #[test]
    fn test_cas_blob_entry_serialization() {
        let entry = CasBlobEntry {
            hash: "a".repeat(64),
            size: 4096,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: CasBlobEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.hash, "a".repeat(64));
        assert_eq!(parsed.size, 4096);
    }

    #[tokio::test]
    async fn test_backup_requires_data_dir() {
        let app = build_router(AppState::in_memory());
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_backup_and_restore_roundtrip() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/backup-test/file1.txt")
                    .body(axum::body::Body::from("hello backup"))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/backup-test/file2.txt")
                    .body(axum::body::Body::from("world backup"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(backup_resp.status(), StatusCode::CREATED);
        let backup_json = body_json(backup_resp).await;
        let backup_id = backup_json["id"].as_str().unwrap().to_string();
        assert_eq!(backup_json["files"], 2);

        let list_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/backups")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(list_resp.status(), StatusCode::OK);
        let list_json = body_json(list_resp).await;
        assert_eq!(list_json.as_array().unwrap().len(), 1);

        let restore_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/restore")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "backup_id": backup_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(restore_resp.status(), StatusCode::OK);
        let restore_json = body_json(restore_resp).await;
        assert_eq!(restore_json["restored_files"], 2);
        assert_eq!(restore_json["total_files"], 2);
    }

    #[tokio::test]
    async fn test_get_latest_backup() {
        let (app, _dir) = backup_test_app();

        let latest_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/backup/latest")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(latest_resp.status(), StatusCode::NOT_FOUND);

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/latest-test/file.txt")
                    .body(axum::body::Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let latest_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/backup/latest")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(latest_resp.status(), StatusCode::OK);
        let json = body_json(latest_resp).await;
        assert!(json["id"].as_str().unwrap().starts_with("backup-"));
        assert_eq!(json["files"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_download_backup() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/dl-test/file.txt")
                    .body(axum::body::Body::from("download me"))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let download_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/backup/download")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(download_resp.status(), StatusCode::OK);
        let ct = download_resp
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(ct, "application/zip");

        let body = download_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();
        let reader = std::io::Cursor::new(&body[..]);
        let archive = zip::ZipArchive::new(reader).unwrap();
        let names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();
        assert!(names.contains(&"manifest.json".to_string()));
    }

    #[tokio::test]
    async fn test_restore_from_archive() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/archive-test/file.txt")
                    .body(axum::body::Body::from("archive content"))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let download_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/backup/download")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let archive_bytes = download_resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri("/archive-test/file.txt")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let restore_resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup/restore")
                    .header("Content-Type", "application/zip")
                    .body(axum::body::Body::from(archive_bytes.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(restore_resp.status(), StatusCode::OK);
        let json = body_json(restore_resp).await;
        assert_eq!(json["files_restored"], 1);
        assert_eq!(json["integrity_verified"], true);
    }

    #[tokio::test]
    async fn test_restore_idempotent() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/idem-test/file.txt")
                    .body(axum::body::Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_id = body_json(backup_resp).await["id"]
            .as_str()
            .unwrap()
            .to_string();

        let restore1 = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/restore")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "backup_id": backup_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let restore1_json = body_json(restore1).await;
        assert_eq!(restore1_json["restored_files"], 1);

        let restore2 = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/restore")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::json!({ "backup_id": backup_id }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        let restore2_json = body_json(restore2).await;
        assert_eq!(restore2_json["restored_files"], 1);
    }

    #[tokio::test]
    async fn test_delete_backup() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/del-test/file.txt")
                    .body(axum::body::Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/admin/backup")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let backup_id = body_json(backup_resp).await["id"]
            .as_str()
            .unwrap()
            .to_string();

        let del_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/admin/backup/{}", backup_id))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

        let list_resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/backups")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let list_json = body_json(list_resp).await;
        assert_eq!(list_json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_integrity_audit_all_ok() {
        let (app, _dir) = backup_test_app();

        let files_to_store: Vec<(&str, &[u8])> = vec![
            ("/integrity-ok/a.txt", b"hello integrity"),
            ("/integrity-ok/b.bin", &[0xDE_u8, 0xAD, 0xBE, 0xEF]),
        ];
        for (path, content) in &files_to_store {
            app.clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("PUT")
                        .uri(*path)
                        .body(axum::body::Body::from(content.to_vec()))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/integrity")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["total_files"], 2);
        assert_eq!(json["ok"], 2);
        assert_eq!(json["mismatches"], 0);
        assert_eq!(json["unreadable"], 0);
        assert!(json["findings"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_integrity_audit_empty_storage() {
        let (app, _dir) = backup_test_app();

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/integrity")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["total_files"], 0);
        assert_eq!(json["ok"], 0);
    }

    #[tokio::test]
    async fn test_integrity_report_has_scanned_at() {
        let (app, _dir) = backup_test_app();

        app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri("/integrity-ts/file.txt")
                    .body(axum::body::Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/admin/integrity")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp).await;
        let scanned_at = json["scanned_at"].as_str().unwrap();
        assert!(scanned_at.parse::<chrono::DateTime<chrono::Utc>>().is_ok());
    }

    #[test]
    fn test_find_latest_manifest_empty() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().to_string_lossy().to_string();
        let result = find_latest_manifest(&data_dir);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_latest_manifest_picks_newest() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().to_string_lossy().to_string();
        let backups = dir.path().join("backups");

        let old_dir = backups.join("backup-old");
        std::fs::create_dir_all(&old_dir).unwrap();
        let old_manifest = BackupManifest {
            id: "backup-old".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            files: vec![],
            cas_blobs: vec![],
            metadata_snapshot: MetadataSnapshot {
                file_count: 0,
                total_bytes: 0,
                cas_blob_count: 0,
                db_checkpoint: false,
                server_version: "1.0.0".to_string(),
            },
            total_bytes: 0,
        };
        std::fs::write(
            old_dir.join("manifest.json"),
            serde_json::to_string(&old_manifest).unwrap(),
        )
        .unwrap();

        let new_dir = backups.join("backup-new");
        std::fs::create_dir_all(&new_dir).unwrap();
        let new_manifest = BackupManifest {
            id: "backup-new".to_string(),
            created_at: "2026-06-10T00:00:00Z".to_string(),
            files: vec![],
            cas_blobs: vec![],
            metadata_snapshot: MetadataSnapshot {
                file_count: 0,
                total_bytes: 0,
                cas_blob_count: 0,
                db_checkpoint: false,
                server_version: "1.0.0".to_string(),
            },
            total_bytes: 0,
        };
        std::fs::write(
            new_dir.join("manifest.json"),
            serde_json::to_string(&new_manifest).unwrap(),
        )
        .unwrap();

        let result = find_latest_manifest(&data_dir).unwrap();
        assert_eq!(result.id, "backup-new");
    }

    #[test]
    fn test_build_and_extract_archive() {
        let dir = tempfile::tempdir().unwrap();
        let backup_dir = dir.path().join("backup-test");
        std::fs::create_dir_all(&backup_dir).unwrap();

        let content = b"test file content";
        std::fs::write(backup_dir.join("test_file.txt"), content).unwrap();

        let sha = compute_sha256(content);
        let manifest = BackupManifest {
            id: "backup-test".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            files: vec![BackupEntry {
                path: "/test/file.txt".to_string(),
                size: content.len() as u64,
                etag: "\"etag\"".to_string(),
                content_hash: "a".repeat(64),
                sha256: sha.clone(),
            }],
            cas_blobs: vec![],
            metadata_snapshot: MetadataSnapshot {
                file_count: 1,
                total_bytes: content.len() as u64,
                cas_blob_count: 0,
                db_checkpoint: false,
                server_version: "1.0.0".to_string(),
            },
            total_bytes: content.len() as u64,
        };

        let archive = build_archive(&backup_dir, &manifest).unwrap();

        let extracted_manifest = extract_manifest_from_archive(&archive).unwrap();
        assert_eq!(extracted_manifest.id, "backup-test");
        assert_eq!(extracted_manifest.files[0].sha256, sha);

        let files = extract_all_from_archive(&archive).unwrap();
        assert_eq!(files.get("test_file.txt").unwrap(), content);
    }
}
