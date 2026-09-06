use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ProductivityState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteboardState {
    pub id: String,
    pub name: String,
    pub elements: Vec<WhiteboardElement>,
    pub viewport: Viewport,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteboardElement {
    pub id: String,
    pub element_type: String,
    pub points: Vec<Point>,
    pub style: ElementStyle,
    pub text: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementStyle {
    pub color: String,
    pub stroke_width: f64,
    pub fill: Option<String>,
    pub opacity: Option<f64>,
}

impl Default for ElementStyle {
    fn default() -> Self {
        Self {
            color: "#000000".to_string(),
            stroke_width: 2.0,
            fill: None,
            opacity: Some(1.0),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateWhiteboardRequest {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveWhiteboardRequest {
    pub elements: Vec<WhiteboardElement>,
    pub viewport: Option<Viewport>,
}

pub async fn list_whiteboards<S: ProductivityState>(state: State<S>) -> Response {
    let mut whiteboards: Vec<serde_json::Value> = vec![];

    if let Some(data_dir) = state.data_dir().map(|d| d.to_string()) {
        let dir = std::path::Path::new(&data_dir).join("whiteboards");
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("json")
                    && let Ok(content) = std::fs::read_to_string(entry.path())
                    && let Ok(wb) = serde_json::from_str::<WhiteboardState>(&content)
                {
                    whiteboards.push(serde_json::json!({
                        "id": wb.id,
                        "name": wb.name,
                        "elements_count": wb.elements.len(),
                        "created_at": wb.created_at,
                        "updated_at": wb.updated_at,
                    }));
                }
            }
        }
    }
    whiteboards.sort_by(|a, b| {
        let ka = a.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
        let kb = b.get("updated_at").and_then(|v| v.as_str()).unwrap_or("");
        kb.cmp(ka)
    });

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "whiteboards": whiteboards,
            "total": whiteboards.len(),
        })),
    )
        .into_response()
}

pub async fn create_whiteboard<S: ProductivityState>(
    _state: State<S>,
    Json(req): Json<CreateWhiteboardRequest>,
) -> Response {
    let id = Uuid::new_v4().to_string();
    let name = req.name.unwrap_or_else(|| format!("Whiteboard {}", &id[..8]));
    let now = chrono::Utc::now().to_rfc3339();

    let whiteboard = WhiteboardState {
        id,
        name,
        elements: vec![],
        viewport: Viewport::default(),
        created_at: now.clone(),
        updated_at: now,
    };

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": whiteboard.id,
            "name": whiteboard.name,
            "created_at": whiteboard.created_at,
        })),
    )
        .into_response()
}

pub async fn get_whiteboard<S: ProductivityState>(state: State<S>, Path(id): Path<String>) -> Response {
    let Some(data_dir) = state.data_dir().map(|d| d.to_string()) else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not_found", "message": "Whiteboard not found"})),
        )
            .into_response();
    };
    // Sanitize: ids are UUIDs; reject anything else to prevent path traversal.
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "bad_request", "message": "Invalid whiteboard id"})),
        )
            .into_response();
    }
    let path = std::path::Path::new(&data_dir)
        .join("whiteboards")
        .join(format!("{}.json", id));
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<WhiteboardState>(&content) {
            Ok(wb) => (StatusCode::OK, Json(serde_json::json!(wb))).into_response(),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal", "message": "Corrupt whiteboard"})),
            )
                .into_response(),
        },
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "not_found", "message": format!("Whiteboard {} not found", id)})),
        )
            .into_response(),
    }
}

pub async fn save_whiteboard<S: ProductivityState>(
    state: State<S>,
    Path(id): Path<String>,
    Json(req): Json<SaveWhiteboardRequest>,
) -> Response {
    let now = chrono::Utc::now().to_rfc3339();

    // Persist to {data_dir}/whiteboards/{id}.json (file-backed, survives restart).
    if let Some(data_dir) = state.data_dir().map(|d| d.to_string()) {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "bad_request", "message": "Invalid whiteboard id"})),
            )
                .into_response();
        }
        let dir = std::path::Path::new(&data_dir).join("whiteboards");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal", "message": format!("Failed to create whiteboards dir: {}", e)})),
            )
                .into_response();
        }
        // Preserve the original name/created_at if the whiteboard already exists.
        let path = dir.join(format!("{}.json", id));
        let (name, created_at) = match std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str::<WhiteboardState>(&c).ok())
        {
            Some(existing) => (existing.name, existing.created_at),
            None => (format!("Whiteboard {}", id), now.clone()),
        };
        let state_to_save = WhiteboardState {
            id: id.clone(),
            name,
            elements: req.elements.clone(),
            viewport: req.viewport.clone().unwrap_or_default(),
            created_at,
            updated_at: now.clone(),
        };
        if let Err(e) = std::fs::write(&path, serde_json::to_string(&state_to_save).unwrap_or_default()) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "internal", "message": format!("Failed to write whiteboard: {}", e)})),
            )
                .into_response();
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": id,
            "elements_count": req.elements.len(),
            "updated_at": now,
        })),
    )
        .into_response()
}

pub async fn export_whiteboard_image<S: ProductivityState>(_state: State<S>, Path(_id): Path<String>) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "not_implemented",
            "message": "PNG export is not yet implemented",
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whiteboard_state_serialization() {
        let state = WhiteboardState {
            id: "test-id".to_string(),
            name: "Test".to_string(),
            elements: vec![],
            viewport: Viewport::default(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: WhiteboardState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test-id");
    }

    #[test]
    fn test_element_style_defaults() {
        let style = ElementStyle::default();
        assert_eq!(style.color, "#000000");
        assert_eq!(style.stroke_width, 2.0);
    }
}
