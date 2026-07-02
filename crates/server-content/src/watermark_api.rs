use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use image::{GenericImageView, Rgba};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::warn;

use common::storage::StorageEngine;
use ferro_server_security::error::ApiError;

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

/// Trait for state needed by watermark handlers.
/// The server crate implements this for its `AppState`.
pub trait WatermarkState: Clone + Send + Sync + 'static {
    fn storage(&self) -> &Arc<dyn StorageEngine>;
    fn db(&self) -> &Option<DbHandle>;
}

/// Watermark policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkPolicy {
    pub id: String,
    pub name: String,
    pub text: String,
    pub position: String,
    pub opacity: f32,
    pub font_size: u32,
    pub color: String,
    pub scope: String,
    pub created_at: String,
}

/// Request body for creating a watermark policy.
#[derive(Debug, Deserialize)]
pub struct CreateWatermarkPolicyRequest {
    pub name: String,
    pub text: String,
    pub position: Option<String>,
    pub opacity: Option<f32>,
    pub font_size: Option<u32>,
    pub color: Option<String>,
    pub scope: Option<String>,
}

/// Watermark preview request.
#[derive(Debug, Deserialize)]
pub struct WatermarkPreviewRequest {
    pub text: String,
    pub position: Option<String>,
    pub opacity: Option<f32>,
    pub font_size: Option<u32>,
    pub color: Option<String>,
    /// Optional: path to a file to apply the watermark to.
    /// If not provided, a sample image is used.
    pub file_path: Option<String>,
}

/// Watermark result info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatermarkResult {
    pub success: bool,
    pub message: String,
    pub output_path: Option<String>,
}

// ---------------------------------------------------------------------------
// WatermarkDbStore
// ---------------------------------------------------------------------------

type WatermarkPolicyRow = (String, String, f32, u32, String);

#[derive(Clone)]
pub struct WatermarkDbStore {
    db: Option<DbHandle>,
}

impl Default for WatermarkDbStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WatermarkDbStore {
    pub fn new() -> Self {
        Self { db: None }
    }

    pub fn with_db(mut self, db: DbHandle) -> Self {
        self.db = Some(db);
        self
    }

    pub fn list_policies(&self) -> Result<Vec<WatermarkPolicy>, String> {
        let Some(db) = &self.db else {
            return Ok(Vec::new());
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn
            .prepare(
                "SELECT id, name, text, position, opacity, font_size, color, scope, created_at FROM watermark_policies ORDER BY created_at",
            )
            .map_err(|e| {
                warn!("Failed to prepare watermark_policies query: {}", e);
                format!("Failed to query watermark policies: {}", e)
            })?;

        let policies: Vec<WatermarkPolicy> = stmt
            .query_map([], |row| {
                Ok(WatermarkPolicy {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    text: row.get(2)?,
                    position: row.get(3)?,
                    opacity: row.get(4)?,
                    font_size: row.get(5)?,
                    color: row.get(6)?,
                    scope: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        Ok(policies)
    }

    pub fn get_default_policy(&self) -> Result<Option<WatermarkPolicyRow>, String> {
        let Some(db) = &self.db else {
            return Ok(None);
        };
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let result: Result<WatermarkPolicyRow, _> = conn.query_row(
            "SELECT text, position, opacity, font_size, color FROM watermark_policies LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        );
        Ok(result.ok())
    }

    pub fn create_policy(
        &self,
        req: &CreateWatermarkPolicyRequest,
    ) -> Result<WatermarkPolicy, String> {
        let Some(db) = &self.db else {
            return Err("Database not configured".to_string());
        };

        if req.name.is_empty() || req.text.is_empty() {
            return Err("Name and text are required".to_string());
        }

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let position = req.position.clone().unwrap_or_else(|| "center".to_string());
        let opacity = req.opacity.unwrap_or(0.3);
        let font_size = req.font_size.unwrap_or(48);
        let color = req.color.clone().unwrap_or_else(|| "#FFFFFF".to_string());
        let scope = req.scope.clone().unwrap_or_else(|| "all".to_string());

        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO watermark_policies (id, name, text, position, opacity, font_size, color, scope, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, req.name, req.text, position, opacity as f64, font_size as i32, color, scope, now],
        )
        .map_err(|e| {
            warn!("Failed to create watermark policy: {}", e);
            format!("Failed to create watermark policy: {}", e)
        })?;

        Ok(WatermarkPolicy {
            id,
            name: req.name.clone(),
            text: req.text.clone(),
            position,
            opacity,
            font_size,
            color,
            scope,
            created_at: now,
        })
    }
}

// ---------------------------------------------------------------------------
// Watermark image processing
// ---------------------------------------------------------------------------

fn parse_color(hex: &str) -> Rgba<u8> {
    let hex = hex.trim_start_matches('#');
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            Rgba([r, g, b, 255])
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            Rgba([r, g, b, a])
        }
        _ => Rgba([255, 255, 255, 255]),
    }
}

fn apply_text_watermark(
    img: &mut image::DynamicImage,
    text: &str,
    position: &str,
    opacity: f32,
    font_size: u32,
    color_hex: &str,
) {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let mut out = rgba.clone();

    let color = parse_color(color_hex);
    let alpha = (opacity * 255.0) as u8;

    // Simple pixel-based text rendering: draw text as a horizontal band
    // Using the image crate's built-in drawing primitives.
    // For a production system, you'd use rusttype or ab_glyph for real font rendering.
    // Here we do a basic watermark by tinting a region of the image.

    let text_width = (text.len() as u32) * (font_size / 2);
    let text_height = font_size;

    let (x_offset, y_offset) = match position {
        "top-left" => (10, 10),
        "top-right" => (w.saturating_sub(text_width + 10), 10),
        "bottom-left" => (10, h.saturating_sub(text_height + 10)),
        "bottom-right" => (
            w.saturating_sub(text_width + 10),
            h.saturating_sub(text_height + 10),
        ),
        "tiled" => {
            // For tiled, we mark every ~200px
            let step_x = 200;
            let step_y = 200;
            for y in (0..h).step_by(step_y as usize) {
                for x in (0..w).step_by(step_x as usize) {
                    blend_region(
                        &mut out,
                        x,
                        y,
                        text_width.min(w - x),
                        text_height.min(h - y),
                        alpha,
                        color,
                        w,
                        h,
                    );
                }
            }
            return;
        }
        _ => {
            // center
            (
                w.saturating_sub(text_width) / 2,
                h.saturating_sub(text_height) / 2,
            )
        }
    };

    blend_region(
        &mut out,
        x_offset,
        y_offset,
        text_width,
        text_height,
        alpha,
        color,
        w,
        h,
    );

    *img = image::DynamicImage::ImageRgba8(out);
}

#[allow(clippy::too_many_arguments)]
fn blend_region(
    img: &mut image::RgbaImage,
    x_start: u32,
    y_start: u32,
    width: u32,
    height: u32,
    alpha: u8,
    color: Rgba<u8>,
    img_w: u32,
    img_h: u32,
) {
    let x_end = (x_start + width).min(img_w);
    let y_end = (y_start + height).min(img_h);
    for y in y_start..y_end {
        for x in x_start..x_end {
            let pixel = img.get_pixel_mut(x, y);
            let blend_factor = alpha as f32 / 255.0;
            let inv = 1.0 - blend_factor;
            pixel[0] = (pixel[0] as f32 * inv + color[0] as f32 * blend_factor) as u8;
            pixel[1] = (pixel[1] as f32 * inv + color[1] as f32 * blend_factor) as u8;
            pixel[2] = (pixel[2] as f32 * inv + color[2] as f32 * blend_factor) as u8;
        }
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// POST /watermark/preview — preview a watermark on an image.
pub async fn preview_watermark<S: WatermarkState>(
    State(state): State<S>,
    Json(req): Json<WatermarkPreviewRequest>,
) -> Response {
    let position = req.position.unwrap_or_else(|| "center".to_string());
    let opacity = req.opacity.unwrap_or(0.3);
    let font_size = req.font_size.unwrap_or(48);
    let color = req.color.unwrap_or_else(|| "#FFFFFF".to_string());

    let file_path = match req.file_path {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "file_path is required for preview"
                })),
            )
                .into_response();
        }
    };

    let data = match state.storage().get(&file_path).await {
        Ok(d) => d,
        Err(_) => {
            return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found");
        }
    };

    let mut img = match image::load_from_memory(&data) {
        Ok(i) => i,
        Err(e) => {
            return ApiError::bad_request(
                ApiError::INVALID_INPUT,
                format!("Not a supported image format: {e}"),
            );
        }
    };

    apply_text_watermark(&mut img, &req.text, &position, opacity, font_size, &color);

    let mut output_buf = std::io::Cursor::new(Vec::new());
    if let Err(e) = img.write_to(&mut output_buf, image::ImageFormat::Png) {
        return ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to encode image: {e}"),
        );
    }

    let bytes = output_buf.into_inner();
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        bytes,
    )
        .into_response()
}

/// POST /watermark/apply/{path} — apply watermark to a file.
pub async fn apply_watermark<S: WatermarkState>(
    State(state): State<S>,
    Path(file_path): Path<String>,
) -> Response {
    // Get the default policy or use hardcoded defaults
    let store = match &state.db() {
        Some(db) => WatermarkDbStore::new().with_db(db.clone()),
        None => WatermarkDbStore::new(),
    };

    let (text, position, opacity, font_size, color) = match store.get_default_policy() {
        Ok(Some(r)) => r,
        _ => (
            "CONFIDENTIAL".to_string(),
            "center".to_string(),
            0.3,
            48,
            "#FFFFFF".to_string(),
        ),
    };

    let data = match state.storage().get(&file_path).await {
        Ok(d) => d,
        Err(_) => {
            return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found");
        }
    };

    let mut img = match image::load_from_memory(&data) {
        Ok(i) => i,
        Err(e) => {
            return ApiError::bad_request(
                ApiError::INVALID_INPUT,
                format!("Not a supported image format: {e}"),
            );
        }
    };

    apply_text_watermark(&mut img, &text, &position, opacity, font_size, &color);

    let mut output_buf = std::io::Cursor::new(Vec::new());
    if let Err(e) = img.write_to(&mut output_buf, image::ImageFormat::Png) {
        return ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to encode image: {e}"),
        );
    }

    let bytes = bytes::Bytes::from(output_buf.into_inner());
    if let Err(e) = state
        .storage()
        .put(&file_path, bytes.clone(), "watermark")
        .await
    {
        return ApiError::internal(
            ApiError::INTERNAL_ERROR,
            format!("Failed to write watermarked file: {e}"),
        );
    }

    (
        StatusCode::OK,
        Json(WatermarkResult {
            success: true,
            message: "Watermark applied successfully".to_string(),
            output_path: Some(file_path),
        }),
    )
        .into_response()
}

/// GET /watermark/policies — list watermark policies.
pub async fn list_policies<S: WatermarkState>(State(state): State<S>) -> Response {
    let store = match &state.db() {
        Some(db) => WatermarkDbStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
        }
    };

    match store.list_policies() {
        Ok(policies) => (
            StatusCode::OK,
            Json(serde_json::json!({ "policies": policies })),
        )
            .into_response(),
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, &e),
    }
}

/// POST /watermark/policies — create a watermark policy.
pub async fn create_policy<S: WatermarkState>(
    State(state): State<S>,
    Json(req): Json<CreateWatermarkPolicyRequest>,
) -> Response {
    let store = match &state.db() {
        Some(db) => WatermarkDbStore::new().with_db(db.clone()),
        None => {
            return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
        }
    };

    match store.create_policy(&req) {
        Ok(policy) => (StatusCode::CREATED, Json(serde_json::json!(policy))).into_response(),
        Err(e) => {
            if e.contains("Name and text are required") {
                ApiError::bad_request(ApiError::INVALID_INPUT, &e)
            } else {
                ApiError::internal(ApiError::INTERNAL_ERROR, &e)
            }
        }
    }
}
