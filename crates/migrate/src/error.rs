use std::fmt;

pub type Result<T> = std::result::Result<T, MigrationError>;

#[derive(Debug)]
pub struct MigrationError {
    pub kind: ErrorKind,
    pub message: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    Connection,
    Authentication,
    Database,
    WebDav,
    Api,
    Io,
    Mapping,
    Config,
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.kind, self.message)
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Connection => write!(f, "connection"),
            ErrorKind::Authentication => write!(f, "auth"),
            ErrorKind::Database => write!(f, "database"),
            ErrorKind::WebDav => write!(f, "webdav"),
            ErrorKind::Api => write!(f, "api"),
            ErrorKind::Io => write!(f, "io"),
            ErrorKind::Mapping => write!(f, "mapping"),
            ErrorKind::Config => write!(f, "config"),
        }
    }
}

impl MigrationError {
    pub fn connection(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Connection,
            message: msg.into(),
            source: None,
        }
    }

    pub fn authentication(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Authentication,
            message: msg.into(),
            source: None,
        }
    }

    pub fn database(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Database,
            message: msg.into(),
            source: None,
        }
    }

    pub fn webdav(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::WebDav,
            message: msg.into(),
            source: None,
        }
    }

    pub fn api(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Api,
            message: msg.into(),
            source: None,
        }
    }

    pub fn io(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Io,
            message: msg.into(),
            source: None,
        }
    }

    pub fn mapping(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Mapping,
            message: msg.into(),
            source: None,
        }
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Config,
            message: msg.into(),
            source: None,
        }
    }

    pub fn with_source(mut self, err: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(err));
        self
    }
}

impl From<rusqlite::Error> for MigrationError {
    fn from(err: rusqlite::Error) -> Self {
        MigrationError::database(err.to_string()).with_source(err)
    }
}

impl From<reqwest::Error> for MigrationError {
    fn from(err: reqwest::Error) -> Self {
        MigrationError::connection(err.to_string()).with_source(err)
    }
}
