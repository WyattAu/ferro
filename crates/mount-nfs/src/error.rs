use std::fmt;

#[derive(Debug)]
pub enum MountError {
    ConnectionFailed {
        source: String,
        mount_point: String,
    },
    NotMounted {
        mount_point: String,
    },
    PermissionDenied {
        path: String,
    },
    NotFound {
        path: String,
    },
    Timeout {
        operation: String,
        elapsed: std::time::Duration,
    },
    Io {
        source: std::io::Error,
        context: String,
    },
    Unsupported {
        feature: String,
    },
}

impl fmt::Display for MountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed {
                source,
                mount_point,
            } => {
                write!(f, "connection to '{}' failed: {}", mount_point, source)
            }
            Self::NotMounted { mount_point } => {
                write!(f, "'{}' is not mounted", mount_point)
            }
            Self::PermissionDenied { path } => {
                write!(f, "permission denied: {}", path)
            }
            Self::NotFound { path } => {
                write!(f, "not found: {}", path)
            }
            Self::Timeout { operation, elapsed } => {
                write!(f, "operation '{}' timed out after {:?}", operation, elapsed)
            }
            Self::Io { source, context } => {
                write!(f, "{}: {}", context, source)
            }
            Self::Unsupported { feature } => {
                write!(f, "unsupported: {}", feature)
            }
        }
    }
}

impl std::error::Error for MountError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for MountError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            source: err,
            context: "IO error".to_string(),
        }
    }
}
