use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WebDAV resource type: collection (directory) or individual resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Collection,
    Resource,
}

/// Scope of a WebDAV lock: exclusive or shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockScope {
    Exclusive,
    Shared,
}

/// Type of a WebDAV lock (currently only write locks are supported).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockType {
    Write,
}

/// Depth of a WebDAV lock or operation.
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
    pub token: LockToken,
    pub path: String,
    pub principal: String,
    pub scope: LockScope,
    pub lock_type: LockType,
    pub depth: LockDepth,
    pub timeout_seconds: u32,
    pub created_at: DateTime<Utc>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavProperty {
    pub namespace: Option<String>,
    pub name: String,
    pub value: String,
}

/// A WebDAV multistatus response containing per-resource status items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStatusResponse {
    pub responses: Vec<MultiStatusItem>,
}

/// A single resource's status within a multistatus response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStatusItem {
    pub href: String,
    pub status: u16,
    pub properties: Vec<WebDavProperty>,
    pub error_description: Option<String>,
}
