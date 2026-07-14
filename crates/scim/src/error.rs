use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ScimError {
    #[error("Not found")]
    NotFound,
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ScimErrorResponse {
    schemas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scim_type: Option<String>,
    detail: String,
    status: u16,
}

impl IntoResponse for ScimError {
    fn into_response(self) -> Response {
        let (status, detail, scim_type) = match &self {
            ScimError::NotFound => (StatusCode::NOT_FOUND, self.to_string(), None),
            ScimError::Conflict(d) => (StatusCode::CONFLICT, d.clone(), Some("invalidValue".into())),
            ScimError::BadRequest(d) => (StatusCode::BAD_REQUEST, d.clone(), None),
            ScimError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string(), None),
            ScimError::Internal(d) => (StatusCode::INTERNAL_SERVER_ERROR, d.clone(), None),
        };
        let body = ScimErrorResponse {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".into()],
            scim_type,
            detail,
            status: status.as_u16(),
        };
        (status, axum::Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scim_error_not_found() {
        let err = ScimError::NotFound;
        assert_eq!(format!("{}", err), "Not found");
    }

    #[test]
    fn test_scim_error_conflict() {
        let err = ScimError::Conflict("exists".to_string());
        assert!(format!("{}", err).contains("exists"));
    }

    #[test]
    fn test_scim_error_bad_request() {
        let err = ScimError::BadRequest("invalid".to_string());
        assert!(format!("{}", err).contains("invalid"));
    }

    #[test]
    fn test_scim_error_unauthorized() {
        let err = ScimError::Unauthorized;
        assert_eq!(format!("{}", err), "Unauthorized");
    }

    #[test]
    fn test_scim_error_internal() {
        let err = ScimError::Internal("oops".to_string());
        assert!(format!("{}", err).contains("oops"));
    }

    #[test]
    fn test_scim_error_debug() {
        let debug = format!("{:?}", ScimError::NotFound);
        assert!(debug.contains("NotFound"));
    }
}
