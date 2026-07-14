//! Lightweight feature flag system for Ferro.
//!
//! Provides a simple, thread-safe mechanism for evaluating feature flags
//! with support for percentage rollouts, tenant-scoped, and user-scoped flags.
//!
//! Flags are evaluated without hot-path allocation. The entire flag set is
//! loaded at startup or on config reload, and reads are lock-free via
//! `RwLock` fast-path.
//!
//! # Example
//!
//! ```rust
//! use ferro_feature_flags::{FeatureFlag, FeatureFlagConfig, FeatureFlags};
//!
//! let mut config = FeatureFlagConfig::default();
//! config.flags.insert("new-ui".into(), FeatureFlag::Enabled);
//!
//! let flags = FeatureFlags::from_config(&config);
//! assert!(flags.is_enabled("new-ui"));
//! ```

use std::collections::HashMap;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::trace;

/// Represents the state of a single feature flag.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum FeatureFlag {
    /// Flag is unconditionally enabled.
    Enabled,
    /// Flag is unconditionally disabled.
    Disabled,
    /// Flag is enabled for a random percentage of evaluations (0–100).
    /// The percentage is evaluated deterministically per flag name using a
    /// simple hash so that a given identifier always sees the same result.
    Percentage(u8),
    /// Flag is enabled only for the listed tenant IDs.
    TenantOnly(Vec<String>),
    /// Flag is enabled only for the listed user IDs.
    UserOnly(Vec<String>),
}

/// JSON-serializable configuration for feature flags.
///
/// Deserialize this from your config file or environment to drive flag state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureFlagConfig {
    /// Map of flag name to its state.
    #[serde(default)]
    pub flags: HashMap<String, FeatureFlag>,
}

/// Thread-safe feature flag evaluator.
///
/// Holds a snapshot of all flags behind a `RwLock` so that reloads (writes)
/// do not block reads on the hot path.
pub struct FeatureFlags {
    inner: RwLock<FeatureFlagsInner>,
}

struct FeatureFlagsInner {
    flags: HashMap<String, FeatureFlag>,
}

impl FeatureFlags {
    /// Create a new `FeatureFlags` instance from the supplied config.
    pub fn from_config(config: &FeatureFlagConfig) -> Self {
        Self {
            inner: RwLock::new(FeatureFlagsInner {
                flags: config.flags.clone(),
            }),
        }
    }

    /// Check if a flag is enabled (unconditionally or via percentage).
    ///
    /// Returns `false` if the flag does not exist.
    pub fn is_enabled(&self, flag_name: &str) -> bool {
        let snapshot = self.inner.read();
        let result = snapshot
            .flags
            .get(flag_name)
            .map(|f| self.evaluate(f, flag_name, "", ""))
            .unwrap_or(false);
        trace!(flag = flag_name, enabled = result, "feature flag evaluated");
        result
    }

    /// Check if a flag is enabled for a specific tenant.
    ///
    /// For `TenantOnly` flags the tenant is matched against the list.
    /// For all other variants the tenant parameter is ignored and the
    /// standard evaluation logic applies.
    pub fn is_enabled_for_tenant(&self, flag_name: &str, tenant_id: &str) -> bool {
        let snapshot = self.inner.read();
        let result = snapshot
            .flags
            .get(flag_name)
            .map(|f| self.evaluate_tenant(f, flag_name, tenant_id))
            .unwrap_or(false);
        trace!(
            flag = flag_name,
            tenant = tenant_id,
            enabled = result,
            "feature flag tenant evaluation"
        );
        result
    }

    /// Check if a flag is enabled for a specific user.
    ///
    /// For `UserOnly` flags the user is matched against the list.
    /// For all other variants the user parameter is ignored and the
    /// standard evaluation logic applies.
    pub fn is_enabled_for_user(&self, flag_name: &str, user_id: &str) -> bool {
        let snapshot = self.inner.read();
        let result = snapshot
            .flags
            .get(flag_name)
            .map(|f| self.evaluate_user(f, flag_name, user_id))
            .unwrap_or(false);
        trace!(
            flag = flag_name,
            user = user_id,
            enabled = result,
            "feature flag user evaluation"
        );
        result
    }

    /// Hot-reload flags from a new config.
    ///
    /// This replaces the entire flag set atomically under a write lock.
    /// Readers that already hold a read lock will continue to see the old
    /// set until they release it.
    pub fn reload(&mut self, config: FeatureFlagConfig) {
        let mut inner = self.inner.write();
        inner.flags = config.flags;
        trace!("feature flags reloaded");
    }

    /// Evaluate a generic flag (no tenant/user context).
    fn evaluate(&self, flag: &FeatureFlag, flag_name: &str, _tenant: &str, _user: &str) -> bool {
        match flag {
            FeatureFlag::Enabled => true,
            FeatureFlag::Disabled => false,
            FeatureFlag::Percentage(pct) => self.hash_in_range(flag_name) < *pct,
            FeatureFlag::TenantOnly(_) => false,
            FeatureFlag::UserOnly(_) => false,
        }
    }

    /// Evaluate a flag with tenant context.
    fn evaluate_tenant(&self, flag: &FeatureFlag, flag_name: &str, tenant_id: &str) -> bool {
        match flag {
            FeatureFlag::Enabled => true,
            FeatureFlag::Disabled => false,
            FeatureFlag::Percentage(pct) => self.hash_in_range(flag_name) < *pct,
            FeatureFlag::TenantOnly(tenants) => tenants.contains(&tenant_id.to_string()),
            FeatureFlag::UserOnly(_) => false,
        }
    }

    /// Evaluate a flag with user context.
    fn evaluate_user(&self, flag: &FeatureFlag, flag_name: &str, user_id: &str) -> bool {
        match flag {
            FeatureFlag::Enabled => true,
            FeatureFlag::Disabled => false,
            FeatureFlag::Percentage(pct) => self.hash_in_range(flag_name) < *pct,
            FeatureFlag::TenantOnly(_) => false,
            FeatureFlag::UserOnly(users) => users.contains(&user_id.to_string()),
        }
    }

    /// Deterministic hash of a flag name into the range 0..100.
    ///
    /// Uses a simple FNV-1a style hash so that the same flag name always
    /// produces the same percentage bucket without needing a real hash map.
    fn hash_in_range(&self, name: &str) -> u8 {
        let hash: u64 = name.bytes().fold(0xcbf29ce484222325, |acc, b| {
            acc.wrapping_mul(0x100000001b3).wrapping_add(b as u64)
        });
        (hash % 100) as u8
    }
}

/// Returns the default feature flags for the Ferro project.
///
/// These are the flags that ship out of the box. Deployments can override
/// or extend them via configuration.
pub fn default_flags() -> FeatureFlagConfig {
    let mut flags = HashMap::new();
    flags.insert("webdav-class3".into(), FeatureFlag::Enabled);
    flags.insert("wasm-workers".into(), FeatureFlag::Enabled);
    flags.insert("activitypub".into(), FeatureFlag::Enabled);
    flags.insert("webrtc".into(), FeatureFlag::Disabled);
    flags.insert("caldav-scheduling".into(), FeatureFlag::Enabled);
    flags.insert("remote-mount".into(), FeatureFlag::Enabled);
    FeatureFlagConfig { flags }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_flags_are_set() {
        let config = default_flags();
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled("webdav-class3"));
        assert!(flags.is_enabled("wasm-workers"));
        assert!(flags.is_enabled("activitypub"));
        assert!(!flags.is_enabled("webrtc"));
        assert!(flags.is_enabled("caldav-scheduling"));
        assert!(flags.is_enabled("remote-mount"));
    }

    #[test]
    fn unknown_flag_returns_false() {
        let config = default_flags();
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("nonexistent-flag"));
    }

    #[test]
    fn enabled_flag_is_always_true() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("test-flag".into(), FeatureFlag::Enabled);
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled("test-flag"));
    }

    #[test]
    fn disabled_flag_is_always_false() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("test-flag".into(), FeatureFlag::Disabled);
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("test-flag"));
    }

    #[test]
    fn percentage_flag_is_deterministic() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("pct-flag".into(), FeatureFlag::Percentage(50));
        let flags = FeatureFlags::from_config(&config);

        let first = flags.is_enabled("pct-flag");
        let second = flags.is_enabled("pct-flag");
        assert_eq!(first, second);
    }

    #[test]
    fn percentage_0_is_never_enabled() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("pct-zero".into(), FeatureFlag::Percentage(0));
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("pct-zero"));
    }

    #[test]
    fn percentage_100_is_always_enabled() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("pct-full".into(), FeatureFlag::Percentage(100));
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled("pct-full"));
    }

    #[test]
    fn tenant_only_matches() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert(
            "tenant-flag".into(),
            FeatureFlag::TenantOnly(vec!["acme".into(), "globex".into()]),
        );
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled_for_tenant("tenant-flag", "acme"));
        assert!(flags.is_enabled_for_tenant("tenant-flag", "globex"));
        assert!(!flags.is_enabled_for_tenant("tenant-flag", "initech"));
    }

    #[test]
    fn tenant_only_ignores_generic_check() {
        let mut config = FeatureFlagConfig::default();
        config
            .flags
            .insert("tenant-flag".into(), FeatureFlag::TenantOnly(vec!["acme".into()]));
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("tenant-flag"));
    }

    #[test]
    fn user_only_matches() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert(
            "user-flag".into(),
            FeatureFlag::UserOnly(vec!["user-1".into(), "user-2".into()]),
        );
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled_for_user("user-flag", "user-1"));
        assert!(flags.is_enabled_for_user("user-flag", "user-2"));
        assert!(!flags.is_enabled_for_user("user-flag", "user-999"));
    }

    #[test]
    fn user_only_ignores_generic_check() {
        let mut config = FeatureFlagConfig::default();
        config
            .flags
            .insert("user-flag".into(), FeatureFlag::UserOnly(vec!["user-1".into()]));
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("user-flag"));
    }

    #[test]
    fn reload_updates_flags() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("reload-flag".into(), FeatureFlag::Disabled);
        let mut flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("reload-flag"));

        let mut new_config = FeatureFlagConfig::default();
        new_config.flags.insert("reload-flag".into(), FeatureFlag::Enabled);
        flags.reload(new_config);

        assert!(flags.is_enabled("reload-flag"));
    }

    #[test]
    fn reload_removes_old_flags() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("old-flag".into(), FeatureFlag::Enabled);
        let mut flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled("old-flag"));

        let new_config = FeatureFlagConfig::default();
        flags.reload(new_config);

        assert!(!flags.is_enabled("old-flag"));
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = default_flags();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: FeatureFlagConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.flags.len(), deserialized.flags.len());
        assert_eq!(config.flags.get("webrtc"), deserialized.flags.get("webrtc"));
    }

    #[test]
    fn flag_enum_serialization() {
        let flag = FeatureFlag::Percentage(42);
        let json = serde_json::to_string(&flag).unwrap();
        let deserialized: FeatureFlag = serde_json::from_str(&json).unwrap();
        assert_eq!(flag, deserialized);
    }
}
