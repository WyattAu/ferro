use axum::response::{IntoResponse, Response};
use common::error::FerroError;

pub struct ServerError(pub FerroError);

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = axum::http::StatusCode::from_u16(self.0.status_code())
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        crate::api_error::ApiError::with_details(
            status,
            crate::api_error::ApiError::INTERNAL_ERROR,
            self.0.to_string(),
            format!("{}", self.0),
        )
    }
}

impl From<FerroError> for ServerError {
    fn from(e: FerroError) -> Self {
        ServerError(e)
    }
}
