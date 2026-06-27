use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub taken_at: Option<String>,
    pub modified_at: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub description: String,
    pub photo_paths: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExifData {
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub date_taken: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAlbumRequest {
    pub name: String,
    pub description: Option<String>,
    pub photo_paths: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PhotosQuery {
    pub start: Option<String>,
    pub end: Option<String>,
}

fn photos_dir(state: &AppState) -> std::path::PathBuf {
    let base = state.data_dir.as_deref().unwrap_or(".ferro");
    std::path::PathBuf::from(base).join("photos")
}

fn albums_file(state: &AppState) -> std::path::PathBuf {
    photos_dir(state).join("albums.json")
}

fn ensure_photos_dir(
    state: &AppState,
) -> Result<std::path::PathBuf, (StatusCode, Json<serde_json::Value>)> {
    let dir = photos_dir(state);
    std::fs::create_dir_all(&dir).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to create photos directory: {}", e)})),
        )
    })?;
    Ok(dir)
}

fn is_image_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
        || lower.ends_with(".bmp")
        || lower.ends_with(".tiff")
        || lower.ends_with(".tif")
        || lower.ends_with(".heic")
        || lower.ends_with(".heif")
}

fn get_mime_type(path: &str) -> String {
    let lower = path.to_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if lower.ends_with(".bmp") {
        "image/bmp".to_string()
    } else if lower.ends_with(".tiff") || lower.ends_with(".tif") {
        "image/tiff".to_string()
    } else if lower.ends_with(".heic") || lower.ends_with(".heif") {
        "image/heic".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

fn load_albums(state: &AppState) -> Vec<Album> {
    let path = albums_file(state);
    if !path.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

fn save_albums(state: &AppState, albums: &[Album]) -> Result<(), std::io::Error> {
    let _ = ensure_photos_dir(state);
    let path = albums_file(state);
    std::fs::write(
        path,
        serde_json::to_string_pretty(albums).unwrap_or_default(),
    )
}

pub async fn list_photos(
    State(state): State<AppState>,
    Query(params): Query<PhotosQuery>,
) -> impl IntoResponse {
    let storage = state.storage.clone();

    let entries = storage.list_all("/", 10000).await.unwrap_or_default();

    let mut photos: Vec<Photo> = entries
        .iter()
        .filter(|e| !e.is_collection && is_image_file(&e.path))
        .map(|e| {
            let name = e.path.rsplit('/').next().unwrap_or(&e.path).to_string();
            Photo {
                id: uuid::Uuid::new_v4().to_string(),
                path: e.path.clone(),
                name,
                size: e.size,
                mime_type: get_mime_type(&e.path),
                taken_at: None,
                modified_at: e.modified_at.to_rfc3339(),
                width: None,
                height: None,
            }
        })
        .collect();

    if let Some(ref start) = params.start {
        photos.retain(|p| p.modified_at.as_str() >= start.as_str());
    }
    if let Some(ref end) = params.end {
        photos.retain(|p| p.modified_at.as_str() <= end.as_str());
    }

    photos.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));

    Json(serde_json::json!({
        "photos": photos,
        "total": photos.len(),
    }))
    .into_response()
}

pub async fn list_albums(State(state): State<AppState>) -> impl IntoResponse {
    let albums = load_albums(&state);
    Json(serde_json::json!({
        "albums": albums,
        "total": albums.len(),
    }))
    .into_response()
}

pub async fn create_album(
    State(state): State<AppState>,
    Json(req): Json<CreateAlbumRequest>,
) -> impl IntoResponse {
    if let Err(e) = ensure_photos_dir(&state) {
        return e.into_response();
    }

    let mut albums = load_albums(&state);
    let album = Album {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        description: req.description.unwrap_or_default(),
        photo_paths: req.photo_paths.unwrap_or_default(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    albums.push(album.clone());
    if let Err(e) = save_albums(&state, &albums) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to save albums: {}", e)})),
        )
            .into_response();
    }

    (StatusCode::CREATED, Json(serde_json::json!(album))).into_response()
}

pub async fn get_thumbnail(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let storage = state.storage.clone();
    let clean_path = path.trim_start_matches('/');

    match storage.get(clean_path).await {
        Ok(data) => {
            let content_type = get_mime_type(clean_path);
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, content_type.as_str())],
                data,
            )
                .into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "File not found"})),
        )
            .into_response(),
    }
}

pub async fn get_exif(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> impl IntoResponse {
    let storage = state.storage.clone();
    let clean_path = path.trim_start_matches('/');

    match storage.get(clean_path).await {
        Ok(_data) => {
            let exif = ExifData {
                camera_make: None,
                camera_model: None,
                date_taken: None,
                latitude: None,
                longitude: None,
                width: None,
                height: None,
            };

            Json(serde_json::json!(exif)).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "File not found"})),
        )
            .into_response(),
    }
}
