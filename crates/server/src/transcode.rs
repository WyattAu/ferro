use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::AppState;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TranscodeFormat {
    #[serde(rename = "mp4")]
    Mp4,
    #[serde(rename = "webm")]
    Webm,
    #[serde(rename = "mov")]
    Mov,
    #[serde(rename = "avi")]
    Avi,
}

impl TranscodeFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            TranscodeFormat::Mp4 => "mp4",
            TranscodeFormat::Webm => "webm",
            TranscodeFormat::Mov => "mov",
            TranscodeFormat::Avi => "avi",
        }
    }

    pub fn ffmpeg_codec(&self) -> &'static str {
        match self {
            TranscodeFormat::Mp4 => "libx264",
            TranscodeFormat::Webm => "libvpx-vp9",
            TranscodeFormat::Mov => "libx264",
            TranscodeFormat::Avi => "libx264",
        }
    }

    pub fn ffmpeg_format(&self) -> &'static str {
        match self {
            TranscodeFormat::Mp4 => "mp4",
            TranscodeFormat::Webm => "webm",
            TranscodeFormat::Mov => "mov",
            TranscodeFormat::Avi => "avi",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TranscodeQuality {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

impl TranscodeQuality {
    pub fn scale_filter(&self) -> &'static str {
        match self {
            TranscodeQuality::Low => "scale=-2:480",
            TranscodeQuality::Medium => "scale=-2:720",
            TranscodeQuality::High => "scale=-2:1080",
        }
    }

    pub fn crf_value(&self) -> &'static str {
        match self {
            TranscodeQuality::Low => "28",
            TranscodeQuality::Medium => "23",
            TranscodeQuality::High => "18",
        }
    }

    pub fn bitrate(&self) -> &'static str {
        match self {
            TranscodeQuality::Low => "500k",
            TranscodeQuality::Medium => "1500k",
            TranscodeQuality::High => "4000k",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TranscodeRequest {
    pub source_path: String,
    pub target_format: TranscodeFormat,
    pub quality: TranscodeQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TranscodeStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "processing")]
    Processing,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeJob {
    pub id: String,
    pub source_path: String,
    pub target_format: TranscodeFormat,
    pub quality: TranscodeQuality,
    pub output_path: String,
    pub status: TranscodeStatus,
    pub progress: f64,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TranscodeResponse {
    pub id: String,
    pub status: TranscodeStatus,
    pub output_path: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Transcode Store
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TranscodeStore {
    jobs: Arc<RwLock<HashMap<String, TranscodeJob>>>,
}

impl Default for TranscodeStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TranscodeStore {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_job(&self, job: TranscodeJob) {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.id.clone(), job);
    }

    pub async fn get_job(&self, id: &str) -> Option<TranscodeJob> {
        let jobs = self.jobs.read().await;
        jobs.get(id).cloned()
    }

    pub async fn update_job(&self, id: &str, status: TranscodeStatus, progress: f64, error: Option<String>) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(id) {
            job.status = status;
            job.progress = progress;
            if error.is_some() {
                job.error = error;
            }
            if matches!(job.status, TranscodeStatus::Completed | TranscodeStatus::Failed) {
                job.completed_at = Some(chrono::Utc::now().to_rfc3339());
            }
        }
    }

    pub async fn list_jobs(&self) -> Vec<TranscodeJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }

    pub async fn delete_job(&self, id: &str) -> bool {
        let mut jobs = self.jobs.write().await;
        jobs.remove(id).is_some()
    }
}

fn transcode_store() -> &'static TranscodeStore {
    use std::sync::OnceLock;
    static STORE: OnceLock<TranscodeStore> = OnceLock::new();
    STORE.get_or_init(TranscodeStore::new)
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// POST /api/v1/transcode — Initiate a transcoding job.
pub async fn start_transcode(
    State(_state): State<AppState>,
    Json(req): Json<TranscodeRequest>,
) -> Response {
    let source = req.source_path.trim_start_matches('/').to_string();
    let source_path = std::path::PathBuf::from(&source);

    if !source_path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Source file not found",
                "path": req.source_path,
            })),
        )
            .into_response();
    }

    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let supported_source = ["mp4", "webm", "mov", "avi", "mkv", "flv", "wmv", "m4v"];
    if !supported_source.contains(&ext.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("Unsupported source format: .{}", ext),
                "supported": supported_source,
            })),
        )
            .into_response();
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let output_name = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let output_dir = source_path.parent().unwrap_or(std::path::Path::new("/"));
    let output_path = output_dir
        .join(format!("{}.{}", output_name, req.target_format.extension()))
        .to_string_lossy()
        .to_string();

    let job = TranscodeJob {
        id: job_id.clone(),
        source_path: req.source_path.clone(),
        target_format: req.target_format.clone(),
        quality: req.quality.clone(),
        output_path: output_path.clone(),
        status: TranscodeStatus::Pending,
        progress: 0.0,
        created_at: chrono::Utc::now().to_rfc3339(),
        completed_at: None,
        error: None,
    };

    transcode_store().create_job(job).await;

    let store = transcode_store().clone();
    let job_id_clone = job_id.clone();
    let source_clone = source.clone();
    let output_clone = output_path.clone();
    let format_clone = req.target_format.clone();
    let quality_clone = req.quality.clone();
    tokio::spawn(async move {
        execute_transcode(
            &store,
            &job_id_clone,
            &source_clone,
            &output_clone,
            &format_clone,
            &quality_clone,
        )
        .await;
    });

    (
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "id": job_id,
            "status": "pending",
            "output_path": output_path,
            "message": "Transcoding job started",
        })),
    )
        .into_response()
}

/// GET /api/v1/transcode/:id/status — Check transcoding progress.
pub async fn transcode_status(Path(id): Path<String>) -> Response {
    match transcode_store().get_job(&id).await {
        Some(job) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": job.id,
                "status": job.status,
                "progress": job.progress,
                "output_path": job.output_path,
                "source_path": job.source_path,
                "target_format": job.target_format,
                "quality": job.quality,
                "created_at": job.created_at,
                "completed_at": job.completed_at,
                "error": job.error,
            })),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Transcode job not found",
            })),
        )
            .into_response(),
    }
}

/// GET /api/v1/transcode — List all transcode jobs.
pub async fn list_transcode_jobs() -> Response {
    let jobs = transcode_store().list_jobs().await;
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "jobs": jobs,
            "total": jobs.len(),
        })),
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// FFmpeg execution
// ---------------------------------------------------------------------------

async fn execute_transcode(
    store: &TranscodeStore,
    job_id: &str,
    source: &str,
    output: &str,
    format: &TranscodeFormat,
    quality: &TranscodeQuality,
) {
    store
        .update_job(job_id, TranscodeStatus::Processing, 0.0, None)
        .await;

    let ffmpeg_path = which_ffmpeg().await;
    let ffmpeg = match ffmpeg_path {
        Some(path) => path,
        None => {
            store
                .update_job(
                    job_id,
                    TranscodeStatus::Failed,
                    0.0,
                    Some("ffmpeg not found in PATH".to_string()),
                )
                .await;
            return;
        }
    };

    let mut cmd = tokio::process::Command::new(&ffmpeg);
    cmd.arg("-i")
        .arg(source)
        .arg("-vf")
        .arg(quality.scale_filter())
        .arg("-c:v")
        .arg(format.ffmpeg_codec())
        .arg("-crf")
        .arg(quality.crf_value())
        .arg("-preset")
        .arg("medium")
        .arg("-b:v")
        .arg(quality.bitrate())
        .arg("-y")
        .arg(output);

    if matches!(format, TranscodeFormat::Mp4 | TranscodeFormat::Mov) {
        cmd.arg("-c:a").arg("aac").arg("-b:a").arg("128k");
    } else if matches!(format, TranscodeFormat::Webm) {
        cmd.arg("-c:a").arg("libopus").arg("-b:a").arg("128k");
    } else {
        cmd.arg("-c:a").arg("aac").arg("-b:a").arg("128k");
    }

    store
        .update_job(job_id, TranscodeStatus::Processing, 10.0, None)
        .await;

    match cmd.output().await {
        Ok(output) => {
            if output.status.success() {
                store
                    .update_job(job_id, TranscodeStatus::Completed, 100.0, None)
                    .await;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let error_msg = if stderr.len() > 500 {
                    format!("{}...", &stderr[..500])
                } else {
                    stderr.to_string()
                };
                store
                    .update_job(
                        job_id,
                        TranscodeStatus::Failed,
                        0.0,
                        Some(error_msg),
                    )
                    .await;
            }
        }
        Err(e) => {
            store
                .update_job(
                    job_id,
                    TranscodeStatus::Failed,
                    0.0,
                    Some(format!("Failed to execute ffmpeg: {}", e)),
                )
                .await;
        }
    }
}

async fn which_ffmpeg() -> Option<String> {
    match tokio::process::Command::new("which")
        .arg("ffmpeg")
        .output()
        .await
    {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path.is_empty() {
                None
            } else {
                Some(path)
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcode_format_extension() {
        assert_eq!(TranscodeFormat::Mp4.extension(), "mp4");
        assert_eq!(TranscodeFormat::Webm.extension(), "webm");
        assert_eq!(TranscodeFormat::Mov.extension(), "mov");
        assert_eq!(TranscodeFormat::Avi.extension(), "avi");
    }

    #[test]
    fn test_transcode_format_codec() {
        assert_eq!(TranscodeFormat::Mp4.ffmpeg_codec(), "libx264");
        assert_eq!(TranscodeFormat::Webm.ffmpeg_codec(), "libvpx-vp9");
    }

    #[test]
    fn test_transcode_quality_scale() {
        assert_eq!(TranscodeQuality::Low.scale_filter(), "scale=-2:480");
        assert_eq!(TranscodeQuality::Medium.scale_filter(), "scale=-2:720");
        assert_eq!(TranscodeQuality::High.scale_filter(), "scale=-2:1080");
    }

    #[test]
    fn test_transcode_quality_crf() {
        assert_eq!(TranscodeQuality::Low.crf_value(), "28");
        assert_eq!(TranscodeQuality::Medium.crf_value(), "23");
        assert_eq!(TranscodeQuality::High.crf_value(), "18");
    }

    #[test]
    fn test_transcode_store_new() {
        let store = TranscodeStore::new();
        assert!(store.jobs.blocking_read().is_empty());
    }

    #[tokio::test]
    async fn test_transcode_store_crud() {
        let store = TranscodeStore::new();
        let job = TranscodeJob {
            id: "test-1".to_string(),
            source_path: "/videos/test.mp4".to_string(),
            target_format: TranscodeFormat::Webm,
            quality: TranscodeQuality::Medium,
            output_path: "/videos/test.webm".to_string(),
            status: TranscodeStatus::Pending,
            progress: 0.0,
            created_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            error: None,
        };

        store.create_job(job.clone()).await;
        assert!(store.get_job("test-1").await.is_some());

        store
            .update_job("test-1", TranscodeStatus::Processing, 50.0, None)
            .await;
        let updated = store.get_job("test-1").await.unwrap();
        assert_eq!(updated.status, TranscodeStatus::Processing);
        assert_eq!(updated.progress, 50.0);

        store
            .update_job("test-1", TranscodeStatus::Completed, 100.0, None)
            .await;
        let completed = store.get_job("test-1").await.unwrap();
        assert!(completed.completed_at.is_some());

        assert!(store.delete_job("test-1").await);
        assert!(store.get_job("test-1").await.is_none());
    }
}
