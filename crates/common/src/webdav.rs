use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `WebDAV` resource type: collection (directory) or individual resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Collection,
    Resource,
}

/// Scope of a `WebDAV` lock: exclusive or shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockScope {
    Exclusive,
    Shared,
}

/// Type of a `WebDAV` lock (currently only write locks are supported).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockType {
    Write,
}

/// Depth of a `WebDAV` lock or operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LockDepth {
    Zero,
    One,
    Infinity,
}

impl LockDepth {
    /// Parse a `Depth` header value into a [`LockDepth`].
    #[must_use]
    pub fn from_header(depth: &str) -> Self {
        match depth.trim() {
            "0" => Self::Zero,
            "1" => Self::One,
            "infinity" => Self::Infinity,
            _ => Self::Infinity,
        }
    }

    /// Convert to the string value used in a `Depth` header.
    #[must_use]
    pub fn to_header(&self) -> &'static str {
        match self {
            Self::Zero => "0",
            Self::One => "1",
            Self::Infinity => "infinity",
        }
    }
}

/// Opaque `WebDAV` lock token backed by a UUID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LockToken(Uuid);

impl LockToken {
    /// Generate a new random lock token.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a lock token from a UUID string.
    pub fn from_str_custom(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }

    /// Return the token in `urn:uuid:…` format.
    #[must_use]
    pub fn as_str(&self) -> String {
        format!("urn:uuid:{}", self.0)
    }

    /// Return the raw UUID string without the `urn:uuid:` prefix.
    #[must_use]
    pub fn as_opaque(&self) -> String {
        self.0.to_string()
    }
}

/// Full information about an active `WebDAV` lock.
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
    #[must_use]
    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.created_at).num_seconds();
        elapsed > i64::from(self.timeout_seconds)
    }

    /// Compute the absolute expiration time of this lock.
    #[must_use]
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.created_at + chrono::Duration::seconds(i64::from(self.timeout_seconds))
    }
}

/// An arbitrary `WebDAV` property with an optional XML namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavProperty {
    /// XML namespace of the property, if any.
    pub namespace: Option<String>,
    /// Local name of the property.
    pub name: String,
    /// Serialized value of the property.
    pub value: String,
}

/// A `WebDAV` multistatus response containing per-resource status items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStatusResponse {
    pub responses: Vec<MultiStatusItem>,
}

/// A single resource's status within a multistatus response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiStatusItem {
    /// URI of the resource this status applies to.
    pub href: String,
    /// HTTP status code for this resource.
    pub status: u16,
    /// `WebDAV` properties included in the response.
    pub properties: Vec<WebDavProperty>,
    /// Optional error description.
    pub error_description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_depth_from_header() {
        assert_eq!(LockDepth::from_header("0"), LockDepth::Zero);
        assert_eq!(LockDepth::from_header("1"), LockDepth::One);
        assert_eq!(LockDepth::from_header("infinity"), LockDepth::Infinity);
        assert_eq!(LockDepth::from_header(" Infinity "), LockDepth::Infinity);
        assert_eq!(LockDepth::from_header("invalid"), LockDepth::Infinity);
        assert_eq!(LockDepth::from_header(""), LockDepth::Infinity);
    }

    #[test]
    fn test_lock_depth_to_header() {
        assert_eq!(LockDepth::Zero.to_header(), "0");
        assert_eq!(LockDepth::One.to_header(), "1");
        assert_eq!(LockDepth::Infinity.to_header(), "infinity");
    }

    #[test]
    fn test_lock_depth_roundtrip() {
        for depth in [LockDepth::Zero, LockDepth::One, LockDepth::Infinity] {
            assert_eq!(LockDepth::from_header(depth.to_header()), depth);
        }
    }

    #[test]
    fn test_lock_token_new() {
        let token = LockToken::new();
        let s = token.as_str();
        assert!(s.starts_with("urn:uuid:"));
    }

    #[test]
    fn test_lock_token_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let token = LockToken::from_str_custom(uuid_str);
        assert!(token.is_some());
        assert!(token.unwrap().as_str().contains(uuid_str));
    }

    #[test]
    fn test_lock_token_from_str_invalid() {
        assert!(LockToken::from_str_custom("not-a-uuid").is_none());
        assert!(LockToken::from_str_custom("").is_none());
    }

    #[test]
    fn test_lock_info_not_expired() {
        let lock = LockInfo {
            token: LockToken::new(),
            path: "/file.txt".into(),
            principal: "alice".into(),
            scope: LockScope::Exclusive,
            lock_type: LockType::Write,
            depth: LockDepth::Zero,
            timeout_seconds: 3600,
            created_at: Utc::now(),
            refresh_count: 0,
        };
        assert!(!lock.is_expired());
    }

    #[test]
    fn test_lock_info_expired() {
        let lock = LockInfo {
            token: LockToken::new(),
            path: "/file.txt".into(),
            principal: "alice".into(),
            scope: LockScope::Exclusive,
            lock_type: LockType::Write,
            depth: LockDepth::Zero,
            timeout_seconds: 1,
            created_at: Utc::now() - chrono::Duration::seconds(10),
            refresh_count: 0,
        };
        assert!(lock.is_expired());
    }

    #[test]
    fn test_lock_info_expires_at() {
        let now = Utc::now();
        let lock = LockInfo {
            token: LockToken::new(),
            path: "/file.txt".into(),
            principal: "alice".into(),
            scope: LockScope::Exclusive,
            lock_type: LockType::Write,
            depth: LockDepth::Zero,
            timeout_seconds: 60,
            created_at: now,
            refresh_count: 0,
        };
        let expires = lock.expires_at();
        let diff = expires.signed_duration_since(now).num_seconds();
        assert!((59..=61).contains(&diff));
    }

    #[test]
    fn test_webdav_property() {
        let prop = WebDavProperty {
            namespace: Some("DAV:".into()),
            name: "displayname".into(),
            value: "Test File".into(),
        };
        assert_eq!(prop.name, "displayname");
        assert!(prop.namespace.is_some());
    }

    #[test]
    fn test_multi_status_response() {
        let resp = MultiStatusResponse {
            responses: vec![MultiStatusItem {
                href: "/file.txt".into(),
                status: 200,
                properties: vec![],
                error_description: None,
            }],
        };
        assert_eq!(resp.responses.len(), 1);
        assert_eq!(resp.responses[0].status, 200);
    }

    #[test]
    fn test_lock_token_default() {
        let token = LockToken::default();
        let s = token.as_str();
        assert!(s.starts_with("urn:uuid:"));
    }

    #[test]
    fn test_lock_token_as_opaque() {
        let token = LockToken::new();
        let opaque = token.as_opaque();
        assert_eq!(opaque.len(), 36); // UUID format
        assert!(!opaque.contains("urn:uuid:"));
    }

    #[test]
    fn test_lock_token_eq() {
        let token1 = LockToken::new();
        let token2 = token1.clone();
        assert_eq!(token1, token2);
    }

    #[test]
    fn test_lock_depth_debug() {
        let depths = [LockDepth::Zero, LockDepth::One, LockDepth::Infinity];
        for depth in depths {
            let debug = format!("{:?}", depth);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_lock_scope_debug() {
        let scopes = [LockScope::Exclusive, LockScope::Shared];
        for scope in scopes {
            let debug = format!("{:?}", scope);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_lock_type_debug() {
        let types = [LockType::Write];
        for lt in types {
            let debug = format!("{:?}", lt);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_lock_info_debug() {
        let lock = LockInfo {
            token: LockToken::new(),
            path: "/file.txt".into(),
            principal: "alice".into(),
            scope: LockScope::Exclusive,
            lock_type: LockType::Write,
            depth: LockDepth::Zero,
            timeout_seconds: 3600,
            created_at: Utc::now(),
            refresh_count: 0,
        };
        let debug = format!("{:?}", lock);
        assert!(debug.contains("LockInfo"));
    }

    #[test]
    fn test_lock_info_clone() {
        let lock1 = LockInfo {
            token: LockToken::new(),
            path: "/file.txt".into(),
            principal: "alice".into(),
            scope: LockScope::Exclusive,
            lock_type: LockType::Write,
            depth: LockDepth::Zero,
            timeout_seconds: 3600,
            created_at: Utc::now(),
            refresh_count: 0,
        };
        let lock2 = lock1.clone();
        assert_eq!(lock1.path, lock2.path);
        assert_eq!(lock1.principal, lock2.principal);
    }

    #[test]
    fn test_lock_info_serialize_deserialize() {
        let lock = LockInfo {
            token: LockToken::new(),
            path: "/file.txt".into(),
            principal: "alice".into(),
            scope: LockScope::Exclusive,
            lock_type: LockType::Write,
            depth: LockDepth::Zero,
            timeout_seconds: 3600,
            created_at: Utc::now(),
            refresh_count: 0,
        };
        let json = serde_json::to_string(&lock).unwrap();
        let deserialized: LockInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(lock.path, deserialized.path);
        assert_eq!(lock.principal, deserialized.principal);
    }

    #[test]
    fn test_webdav_property_debug() {
        let prop = WebDavProperty {
            namespace: Some("DAV:".into()),
            name: "displayname".into(),
            value: "Test File".into(),
        };
        let debug = format!("{:?}", prop);
        assert!(debug.contains("WebDavProperty"));
    }

    #[test]
    fn test_webdav_property_clone() {
        let prop1 = WebDavProperty {
            namespace: Some("DAV:".into()),
            name: "displayname".into(),
            value: "Test File".into(),
        };
        let prop2 = prop1.clone();
        assert_eq!(prop1.name, prop2.name);
        assert_eq!(prop1.value, prop2.value);
    }

    #[test]
    fn test_webdav_property_serialize_deserialize() {
        let prop = WebDavProperty {
            namespace: Some("DAV:".into()),
            name: "displayname".into(),
            value: "Test File".into(),
        };
        let json = serde_json::to_string(&prop).unwrap();
        let deserialized: WebDavProperty = serde_json::from_str(&json).unwrap();
        assert_eq!(prop.name, deserialized.name);
    }

    #[test]
    fn test_multi_status_item_debug() {
        let item = MultiStatusItem {
            href: "/file.txt".into(),
            status: 200,
            properties: vec![],
            error_description: None,
        };
        let debug = format!("{:?}", item);
        assert!(debug.contains("MultiStatusItem"));
    }

    #[test]
    fn test_multi_status_item_clone() {
        let item1 = MultiStatusItem {
            href: "/file.txt".into(),
            status: 200,
            properties: vec![],
            error_description: None,
        };
        let item2 = item1.clone();
        assert_eq!(item1.href, item2.href);
        assert_eq!(item1.status, item2.status);
    }

    #[test]
    fn test_multi_status_item_serialize_deserialize() {
        let item = MultiStatusItem {
            href: "/file.txt".into(),
            status: 200,
            properties: vec![],
            error_description: None,
        };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: MultiStatusItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item.href, deserialized.href);
    }

    #[test]
    fn test_multi_status_response_debug() {
        let resp = MultiStatusResponse { responses: vec![] };
        let debug = format!("{:?}", resp);
        assert!(debug.contains("MultiStatusResponse"));
    }

    #[test]
    fn test_multi_status_response_clone() {
        let resp1 = MultiStatusResponse {
            responses: vec![MultiStatusItem {
                href: "/file.txt".into(),
                status: 200,
                properties: vec![],
                error_description: None,
            }],
        };
        let resp2 = resp1.clone();
        assert_eq!(resp1.responses.len(), resp2.responses.len());
    }

    #[test]
    fn test_multi_status_response_serialize_deserialize() {
        let resp = MultiStatusResponse {
            responses: vec![MultiStatusItem {
                href: "/file.txt".into(),
                status: 200,
                properties: vec![],
                error_description: None,
            }],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: MultiStatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp.responses.len(), deserialized.responses.len());
    }

    #[test]
    fn test_resource_type_debug() {
        let types = [ResourceType::Collection, ResourceType::Resource];
        for rt in types {
            let debug = format!("{:?}", rt);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_resource_type_clone() {
        let rt1 = ResourceType::Collection;
        let rt2 = rt1.clone();
        assert!(matches!(rt2, ResourceType::Collection));
    }

    #[test]
    fn test_resource_type_serialize_deserialize() {
        let rt = ResourceType::Resource;
        let json = serde_json::to_string(&rt).unwrap();
        let deserialized: ResourceType = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, ResourceType::Resource));
    }
}
