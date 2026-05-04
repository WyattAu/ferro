use axum::response::{IntoResponse, Response};
use common::error::FerroError;

/// Wrapper that converts [`FerroError`] into an HTTP response.
pub struct ServerError(pub FerroError);

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = axum::http::StatusCode::from_u16(self.0.status_code())
            .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);

        let error_code = match &self.0 {
            FerroError::NotFound(_) => crate::api_error::ApiError::FILE_NOT_FOUND,
            FerroError::AlreadyExists(_) => crate::api_error::ApiError::FILE_EXISTS,
            FerroError::PermissionDenied(_) => crate::api_error::ApiError::POLICY_DENIED,
            FerroError::InvalidArgument(_) => crate::api_error::ApiError::PATH_INVALID,
            FerroError::LockConflict(_) => crate::api_error::ApiError::FILE_LOCKED,
            FerroError::LockTokenNotFound(_) => crate::api_error::ApiError::CONFLICT,
            FerroError::PreconditionFailed(_) => crate::api_error::ApiError::BAD_REQUEST,
            FerroError::Unauthorized => crate::api_error::ApiError::AUTH_REQUIRED,
            FerroError::Timeout
            | FerroError::Internal(_)
            | FerroError::StorageBackend(_)
            | FerroError::XmlError(_)
            | FerroError::UnsupportedMediaType(_) => crate::api_error::ApiError::INTERNAL_ERROR,
        };

        crate::api_error::ApiError::with_details(
            status,
            error_code,
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
