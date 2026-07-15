use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::AppState;

/// Generate a QR code SVG for a share link.
pub async fn share_qr_code(State(state): State<AppState>, Path(token): Path<String>) -> Response {
    // Verify the share exists
    let _link = match state.share_store.get(&token).await {
        Some(l) => l,
        None => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({"error": "share_not_found"})),
            )
                .into_response();
        }
    };

    let share_url = format!("{}/s/{}", state.external_url, token);

    match generate_qr_svg(&share_url) {
        Ok(svg) => (StatusCode::OK, [(header::CONTENT_TYPE, "image/svg+xml")], svg).into_response(),
        Err(e) => {
            tracing::warn!(error = %e, "failed to generate QR code");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": "qr_generation_failed"})),
            )
                .into_response()
        }
    }
}

/// Generate a QR code SVG string for the given data.
fn generate_qr_svg(data: &str) -> Result<String, Box<dyn std::error::Error>> {
    use qrcode::QrCode;
    use qrcode::render::svg;

    let code = QrCode::new(data.as_bytes())?;
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(200, 200)
        .max_dimensions(400, 400)
        .build();

    Ok(svg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_qr_svg() {
        let svg = generate_qr_svg("https://example.com/s/test-token");
        assert!(svg.is_ok());
        let svg_str = svg.unwrap();
        assert!(svg_str.contains("<svg"));
        assert!(svg_str.contains("image/svg+xml") || svg_str.contains("svg"));
    }

    #[test]
    fn test_generate_qr_svg_invalid_data() {
        // QR codes can encode any data, so this should succeed
        let svg = generate_qr_svg("");
        assert!(svg.is_ok());
    }
}
