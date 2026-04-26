use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Collection,
    Resource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockScope {
    Exclusive,
    Shared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockType {
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockDepth {
    Zero,
    One,
    Infinity,
}

impl LockDepth {
    pub fn from_header(depth: &str) -> Self {
        match depth.trim() {
            "0" => Self::Zero,
            "1" => Self::One,
            "infinity" => Self::Infinity,
            _ => Self::Infinity,
        }
    }

    pub fn to_header(&self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::One => "1",
            Self::Infinity => "infinity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LockToken(Uuid);

impl LockToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_str_custom(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }

    pub fn as_str(&self) -> String {
        format!("urn:uuid:{}", self.0)
    }

    pub fn as_opaque(&self) -> String {
        self.0.to_string()
    }
}

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
    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.created_at)
            .num_seconds();
        elapsed > self.timeout_seconds as i64
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.created_at
            + chrono::Duration::seconds(self.timeout_seconds as i64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavProperty {
    pub namespace: Option<String>,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStatusResponse {
    pub responses: Vec<MultiStatusItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStatusItem {
    pub href: String,
    pub status: u16,
    pub properties: Vec<WebDavProperty>,
    pub error_description: Option<String>,
}
