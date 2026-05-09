use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebDAV resource type: collection (directory) or individual resource.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Collection,
    Resource,
}

/// Scope of a WebDAV lock: exclusive or shared.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockScope {
    Exclusive,
    Shared,
}

/// Type of a WebDAV lock (currently only write locks are supported).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockType {
    Write,
}

/// Depth of a WebDAV lock or operation.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockDepth {
    Zero,
    One,
    Infinity,
}

impl LockDepth {
    /// Parse a `Depth` header value into a [`LockDepth`].
    pub fn from_header(depth: &str) -> Self {
        match depth.trim() {
            "0" => Self::Zero,
            "1" => Self::One,
            "infinity" => Self::Infinity,
            _ => Self::Infinity,
        }
    }

    /// Convert to the string value used in a `Depth` header.
    pub fn to_header(&self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::One => "1",
            Self::Infinity => "infinity",
        }
    }
}

/// Opaque WebDAV lock token backed by a UUID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LockToken(Uuid);

impl LockToken {
    /// Generate a new random lock token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a lock token from a UUID string.
    pub fn from_str_custom(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }

    /// Return the token in `urn:uuid:…` format.
    pub fn as_str(&self) -> String {
        format!("urn:uuid:{}", self.0)
    }

    /// Return the raw UUID string without the `urn:uuid:` prefix.
    pub fn as_opaque(&self) -> String {
        self.0.to_string()
    }
}

/// Full information about an active WebDAV lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    /// Unique lock token.
    pub token: LockToken,
    /// Path of the locked resource.
    pub path: String,
    /// Principal (user) who holds the lock.
    pub principal: String,
    /// Whether the lock is exclusive or shared.
    pub scope: LockScope,
    /// Type of the lock (currently only write).
    pub lock_type: LockType,
    /// Depth of the lock (0, 1, or infinity).
    pub depth: LockDepth,
    /// Lock timeout in seconds.
    pub timeout_seconds: u32,
    /// When the lock was created.
    pub created_at: DateTime<Utc>,
    /// Number of times the lock has been refreshed.
    pub refresh_count: u32,
}

impl LockInfo {
    /// Check whether the lock has exceeded its timeout.
    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds();
        elapsed > self.timeout_seconds as i64
    }

    /// Compute the absolute expiration time of this lock.
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.created_at + chrono::Duration::seconds(self.timeout_seconds as i64)
    }
}

/// An arbitrary WebDAV property with an optional XML namespace.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavProperty {
    /// XML namespace of the property, if any.
    pub namespace: Option<String>,
    /// Local name of the property.
    pub name: String,
    /// Serialized value of the property.
    pub value: String,
}

/// A WebDAV multistatus response containing per-resource status items.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStatusResponse {
    pub responses: Vec<MultiStatusItem>,
}

/// A single resource's status within a multistatus response.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStatusItem {
    /// URI of the resource this status applies to.
    pub href: String,
    /// HTTP status code for this resource.
    pub status: u16,
    /// WebDAV properties included in the response.
    pub properties: Vec<WebDavProperty>,
    /// Optional error description.
    pub error_description: Option<String>,
}
