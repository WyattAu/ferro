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

pub async fn list_whiteboards<S: ProductivityState>(_state: State<S>) -> Response {
    let whiteboards: Vec<serde_json::Value> = vec![];

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
    let name = req
        .name
        .unwrap_or_else(|| format!("Whiteboard {}", &id[..8]));
    let now = chrono::Utc::now().to_rfc3339();

    let whiteboard = WhiteboardState {
        id: id.clone(),
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

pub async fn get_whiteboard<S: ProductivityState>(
    _state: State<S>,
    Path(id): Path<String>,
) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "not_found",
            "message": format!("Whiteboard {} not found", id),
        })),
    )
        .into_response()
}

pub async fn save_whiteboard<S: ProductivityState>(
    _state: State<S>,
    Path(id): Path<String>,
    Json(req): Json<SaveWhiteboardRequest>,
) -> Response {
    let now = chrono::Utc::now().to_rfc3339();

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

pub async fn export_whiteboard_image<S: ProductivityState>(
    _state: State<S>,
    Path(_id): Path<String>,
) -> Response {
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
