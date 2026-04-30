use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP error: {status} {body}")]
    Http { status: u16, body: String },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("XML parse error: {0}")]
    XmlParse(String),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("Authentication failed")]
    AuthFailed,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl ClientError {
    pub fn status_code(&self) -> Option<u16> {
        match self {
            ClientError::Http { status, .. } => Some(*status),
            ClientError::AuthFailed => Some(401),
            ClientError::NotFound(_) => Some(404),
            _ => None,
        }
    }
}
