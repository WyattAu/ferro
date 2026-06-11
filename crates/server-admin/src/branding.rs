//! Server-side branding and theming configuration.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use ferro_server::AppState;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Branding configuration stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    /// Custom logo URL or data URI.
    #[serde(default)]
    pub logo_url: Option<String>,
    /// Primary color (hex, e.g., "#3b82f6").
    #[serde(default = "default_primary_color")]
    pub primary_color: String,
    /// Custom instance title (overrides default "Ferro").
    #[serde(default = "default_title")]
    pub title: String,
    /// Favicon URL or data URI.
    #[serde(default)]
    pub favicon_url: Option<String>,
    /// Custom CSS to inject into the web UI.
    #[serde(default)]
    pub custom_css: Option<String>,
}

fn default_primary_color() -> String {
    "#3b82f6".to_string()
}

fn default_title() -> String {
    "Ferro".to_string()
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            logo_url: None,
            primary_color: default_primary_color(),
            title: default_title(),
            favicon_url: None,
            custom_css: None,
        }
    }
}

/// Request body for updating branding configuration.
#[derive(Debug, Deserialize)]
pub struct UpdateBrandingRequest {
    pub logo_url: Option<String>,
    pub primary_color: Option<String>,
    pub title: Option<String>,
    pub favicon_url: Option<String>,
    pub custom_css: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/admin/branding`
///
/// Get the current branding configuration.
pub async fn get_branding(State(state): State<AppState>) -> Response {
    let branding = load_branding(&state);
    (StatusCode::OK, axum::Json(branding)).into_response()
}

/// `GET /api/branding`
///
/// Public endpoint for the web UI to fetch branding configuration.
/// No authentication required.
pub async fn get_public_branding(State(state): State<AppState>) -> Response {
    let branding = load_branding(&state);
    (StatusCode::OK, axum::Json(branding)).into_response()
}

/// `PUT /api/admin/branding`
///
/// Update the branding configuration.
pub async fn update_branding(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<UpdateBrandingRequest>,
) -> Response {
    let mut branding = load_branding(&state);

    if let Some(logo_url) = req.logo_url {
        branding.logo_url = Some(logo_url);
    }
    if let Some(primary_color) = req.primary_color {
        // Validate hex color format
        if !primary_color.starts_with('#')
            || !(primary_color.len() == 7 || primary_color.len() == 4)
        {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "primary_color must be a hex color string (e.g., '#3b82f6')"
                })),
            )
                .into_response();
        }
        branding.primary_color = primary_color;
    }
    if let Some(title) = req.title {
        if title.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "title must not be empty"
                })),
            )
                .into_response();
        }
        branding.title = title;
    }
    if let Some(favicon_url) = req.favicon_url {
        branding.favicon_url = Some(favicon_url);
    }
    if let Some(custom_css) = req.custom_css {
        branding.custom_css = Some(custom_css);
    }

    save_branding(&state, &branding);

    (StatusCode::OK, axum::Json(branding)).into_response()
}

/// `DELETE /api/admin/branding`
///
/// Reset branding to defaults.
pub async fn reset_branding(State(state): State<AppState>) -> Response {
    let default = BrandingConfig::default();
    save_branding(&state, &default);
    (StatusCode::OK, axum::Json(default)).into_response()
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

/// Load branding config from the preferences table.
pub fn load_branding(state: &AppState) -> BrandingConfig {
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(value) = conn.query_row(
            "SELECT value FROM preferences WHERE key = 'branding'",
            [],
            |row| row.get::<_, String>(0),
        ) && let Ok(config) = serde_json::from_str::<BrandingConfig>(&value)
        {
            return config;
        }
    }
    BrandingConfig::default()
}

/// Save branding config to the preferences table.
fn save_branding(state: &AppState, config: &BrandingConfig) {
    if let Some(ref db) = state.db {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let value = serde_json::to_string(config).unwrap_or_default();
        if let Err(e) = conn.execute(
            "INSERT OR REPLACE INTO preferences (key, value) VALUES ('branding', ?1)",
            params![value],
        ) {
            tracing::warn!(error = %e, "failed to save branding config");
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_branding() {
        let branding = BrandingConfig::default();
        assert_eq!(branding.primary_color, "#3b82f6");
        assert_eq!(branding.title, "Ferro");
        assert!(branding.logo_url.is_none());
        assert!(branding.favicon_url.is_none());
        assert!(branding.custom_css.is_none());
    }

    #[test]
    fn test_branding_serialization() {
        let branding = BrandingConfig {
            logo_url: Some("https://example.com/logo.svg".to_string()),
            primary_color: "#ef4444".to_string(),
            title: "My Cloud".to_string(),
            favicon_url: None,
            custom_css: Some("body { font-family: sans-serif; }".to_string()),
        };
        let json = serde_json::to_string(&branding).unwrap();
        let deserialized: BrandingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "My Cloud");
        assert_eq!(deserialized.primary_color, "#ef4444");
    }

    #[test]
    fn test_branding_roundtrip_empty() {
        let json = "{}";
        let branding: BrandingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(branding.primary_color, "#3b82f6");
        assert_eq!(branding.title, "Ferro");
    }
}
