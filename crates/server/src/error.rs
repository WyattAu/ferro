use axum::response::{IntoResponse, Response};
use common::error::FerroError;

/// Wrapper that converts [`FerroError`] into an HTTP response.
#[derive(Debug)]
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
            FerroError::WormProtected(_) => crate::api_error::ApiError::WORM_PROTECTED,
            _ => crate::api_error::ApiError::INTERNAL_ERROR,
        };

        crate::api_error::ApiError::with_details(status, error_code, self.0.to_string(), format!("{}", self.0))
    }
}

impl From<FerroError> for ServerError {
    fn from(e: FerroError) -> Self {
        ServerError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    async fn response_body(resp: Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_not_found_error() {
        let err = ServerError(FerroError::NotFound("missing.txt".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = response_body(resp).await;
        assert!(body.contains("FILE_NOT_FOUND"));
        assert!(body.contains("missing.txt"));
    }

    #[tokio::test]
    async fn test_already_exists_error() {
        let err = ServerError(FerroError::AlreadyExists("file.txt".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body = response_body(resp).await;
        assert!(body.contains("FILE_EXISTS"));
    }

    #[tokio::test]
    async fn test_permission_denied_error() {
        let err = ServerError(FerroError::PermissionDenied("no access".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body = response_body(resp).await;
        assert!(body.contains("POLICY_DENIED"));
    }

    #[tokio::test]
    async fn test_invalid_argument_error() {
        let err = ServerError(FerroError::InvalidArgument("bad path".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = response_body(resp).await;
        assert!(body.contains("PATH_INVALID"));
    }

    #[tokio::test]
    async fn test_lock_conflict_error() {
        let err = ServerError(FerroError::LockConflict("locked".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::LOCKED);
        let body = response_body(resp).await;
        assert!(body.contains("FILE_LOCKED"));
    }

    #[tokio::test]
    async fn test_lock_token_not_found_error() {
        let err = ServerError(FerroError::LockTokenNotFound("token-123".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body = response_body(resp).await;
        assert!(body.contains("CONFLICT"));
    }

    #[tokio::test]
    async fn test_precondition_failed_error() {
        let err = ServerError(FerroError::PreconditionFailed("etag mismatch".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::PRECONDITION_FAILED);
        let body = response_body(resp).await;
        assert!(body.contains("BAD_REQUEST"));
    }

    #[tokio::test]
    async fn test_unauthorized_error() {
        let err = ServerError(FerroError::Unauthorized);
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = response_body(resp).await;
        assert!(body.contains("AUTH_REQUIRED"));
    }

    #[tokio::test]
    async fn test_internal_error() {
        let err = ServerError(FerroError::Internal("something broke".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = response_body(resp).await;
        assert!(body.contains("INTERNAL_ERROR"));
    }

    #[tokio::test]
    async fn test_timeout_error() {
        let err = ServerError(FerroError::Timeout);
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::GATEWAY_TIMEOUT);
        let body = response_body(resp).await;
        assert!(body.contains("INTERNAL_ERROR"));
    }

    #[tokio::test]
    async fn test_storage_backend_error() {
        let err = ServerError(FerroError::StorageBackend("disk full".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        let body = response_body(resp).await;
        assert!(body.contains("INTERNAL_ERROR"));
    }

    #[tokio::test]
    async fn test_xml_error() {
        let err = ServerError(FerroError::XmlError("malformed".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = response_body(resp).await;
        assert!(body.contains("INTERNAL_ERROR"));
    }

    #[tokio::test]
    async fn test_unsupported_media_type_error() {
        let err = ServerError(FerroError::UnsupportedMediaType("audio/mp3".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        let body = response_body(resp).await;
        assert!(body.contains("INTERNAL_ERROR"));
    }

    #[tokio::test]
    async fn test_worm_protected_error() {
        let err = ServerError(FerroError::WormProtected("immutable.txt".into()));
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body = response_body(resp).await;
        assert!(body.contains("WORM_PROTECTED"));
    }

    #[test]
    fn test_from_ferro_error() {
        let fe = FerroError::NotFound("x".into());
        let se: ServerError = fe.into();
        assert_eq!(se.0.status_code(), 404);
    }
}
