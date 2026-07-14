use axum::http::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum CalDavError {
    #[error("Calendar not found: {0}")]
    NotFound(String),

    #[error("Calendar already exists: {0}")]
    Conflict(String),

    #[error("Invalid iCalendar data: {0}")]
    InvalidData(String),

    #[error("Permission denied")]
    Forbidden,

    #[error("Invalid XML: {0}")]
    XmlError(String),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),
}

impl CalDavError {
    pub fn status_code(&self) -> u16 {
        match self {
            CalDavError::NotFound(_) => 404,
            CalDavError::Conflict(_) => 409,
            CalDavError::InvalidData(_) => 400,
            CalDavError::Forbidden => 403,
            CalDavError::XmlError(_) => 400,
            CalDavError::Store(_) => 500,
            CalDavError::BadRequest(_) => 400,
        }
    }

    pub fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub type Result<T> = std::result::Result<T, CalDavError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cal_dav_error_not_found() {
        let err = CalDavError::NotFound("missing".to_string());
        assert_eq!(err.status_code(), 404);
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        assert!(format!("{}", err).contains("missing"));
    }

    #[test]
    fn test_cal_dav_error_conflict() {
        let err = CalDavError::Conflict("exists".to_string());
        assert_eq!(err.status_code(), 409);
        assert_eq!(err.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn test_cal_dav_error_invalid_data() {
        let err = CalDavError::InvalidData("bad ical".to_string());
        assert_eq!(err.status_code(), 400);
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_cal_dav_error_forbidden() {
        let err = CalDavError::Forbidden;
        assert_eq!(err.status_code(), 403);
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_cal_dav_error_xml_error() {
        let err = CalDavError::XmlError("bad xml".to_string());
        assert_eq!(err.status_code(), 400);
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_cal_dav_error_store() {
        let err = CalDavError::Store("db error".to_string());
        assert_eq!(err.status_code(), 500);
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_cal_dav_error_bad_request() {
        let err = CalDavError::BadRequest("missing param".to_string());
        assert_eq!(err.status_code(), 400);
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_cal_dav_error_debug() {
        let err = CalDavError::NotFound("debug".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("NotFound"));
    }

    #[test]
    fn test_cal_dav_error_is_error_trait() {
        let err = CalDavError::Forbidden;
        let err_trait: &dyn std::error::Error = &err;
        assert!(!err_trait.to_string().is_empty());
    }
}
