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
