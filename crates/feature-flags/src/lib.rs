//! Lightweight feature flag system for Ferro, backed by `flag-kit`.
//!
//! Provides a simple, thread-safe mechanism for evaluating feature flags
//! with support for percentage rollouts, tenant-scoped, and user-scoped flags.
//!
//! # Migration to `flag-kit` (v1)
//!
//! - **Validation**: `is_valid_flag_name` now delegates to `flag_kit::FlagName::try_from`
//!   which enforces `^[a-z][a-z0-9_]*$` (lowercase snake_case). See
//!   “Before/After invalid flag handling” below.
//! - **Storage**: the hot-path store wraps `flag_kit::MemoryFlagStore` (`DashMap`)
//!   via the `FlagStore` trait. The existing `FeatureFlag` enum is kept as a
//!   wrapper that delegates to `flag_kit::Flag { name, enabled, percentage }`.
//!   For v1 `MemoryFlagStore` is wrapped synchronously; a future `sqlite` feature
//!   will delegate to `flag_kit::SqliteFlagStore` (requires `flag-kit/sqlite`).
//! - **Rollout**: percentage evaluation now uses `flag_kit::bucket` /
//!   `Evaluator::enabled_for(&FlagName, user_id, org_id)` (SipHash of
//!   `flag_name + user_id % 100 < percentage`) instead of the previous custom
//!   FNV-1a `hash_in_range`.
//! - **DB persistence**: not yet wired; `MemoryFlagStore` is used directly.
//!   Persisted stores can be added via `FlagStore` and `reload` without API
//!   breakage; audit trail via `FlagChange` is available in `flag-kit`.
//!
//! # Before / After invalid flag handling
//!
//! - **Before**: no validation. Any string (including `"webdav-class3"`,
//!   `"Hello"`, `"123"`, `""`, `"a-b"`) was accepted as a flag name and stored
//!   in `HashMap<String, FeatureFlag>`. Percentage bucket used FNV-1a of the
//!   flag name alone.
//! - **After**: `is_valid_flag_name(name)` and `validate_flag_name(name)`
//!   call `FlagName::try_from(name)` → `Err(FlagError::InvalidName{ name, reason })`
//!   if `!^[a-z][a-z0-9_]*$`. Legacy hyphenated defaults (`"webdav-class3"` etc.)
//!   are still stored via `FlagName::new_unchecked` for backward compatibility
//!   but are considered *invalid* for new flags. Callers should validate before
//!   inserting. Percentage rollout uses `flag_kit::bucket(flag, user) % 100`.
//!
//! # Example
//!
//! ```rust
//! use ferro_feature_flags::{FeatureFlag, FeatureFlagConfig, FeatureFlags};
//!
//! let mut config = FeatureFlagConfig::default();
//! config.flags.insert("new_ui".into(), FeatureFlag::Enabled);
//!
//! let flags = FeatureFlags::from_config(&config);
//! assert!(flags.is_enabled("new_ui"));
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use flag_kit::{Flag, FlagName, FlagStore, MemoryFlagStore, bucket};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::trace;

// ---------------------------------------------------------------------------
// Validation helpers — delegate to flag_kit::FlagName::try_from
// ---------------------------------------------------------------------------

/// Returns `true` if `name` is a valid flag name per `flag_kit::FlagName`.
///
/// Valid names match `^[a-z][a-z0-9_]*$`.
pub fn is_valid_flag_name(name: &str) -> bool {
    FlagName::try_from(name).is_ok()
}

/// Validates `name` via `FlagName::try_from`.
///
/// Returns the validated `FlagName` or `FlagError::InvalidName`.
pub fn validate_flag_name(name: &str) -> Result<FlagName, flag_kit::FlagError> {
    FlagName::try_from(name)
}

// ---------------------------------------------------------------------------
// FeatureFlag enum — kept as wrapper delegating to flag_kit::Flag
// ---------------------------------------------------------------------------

/// Represents the state of a single feature flag.
///
/// This enum is kept for backward compatibility. For `Enabled`/`Disabled`/
/// `Percentage` it delegates to `flag_kit::Flag { name, enabled, percentage }`.
/// `TenantOnly`/`UserOnly` are legacy targeting variants that remain in the
/// wrapper and are evaluated against allow-lists; future targeting should use
/// `Evaluator::enabled_for` with `org_id` / `user_id`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum FeatureFlag {
    /// Flag is unconditionally enabled (`Flag { enabled: true, percentage: 100 }`).
    Enabled,
    /// Flag is unconditionally disabled (`Flag { enabled: false, percentage: 0 }`).
    Disabled,
    /// Flag is enabled for a deterministic percentage of users (0–100).
    /// Evaluated via `flag_kit::bucket(flag_name, user_id) < pct`.
    Percentage(u8),
    /// Flag is enabled only for the listed tenant IDs.
    TenantOnly(Vec<String>),
    /// Flag is enabled only for the listed user IDs.
    UserOnly(Vec<String>),
}

impl FeatureFlag {
    /// Convert this wrapper into a `flag_kit::Flag` when representable.
    ///
    /// `TenantOnly`/`UserOnly` have no direct `Flag` representation and return
    /// `None`; callers should keep those in the legacy map.
    pub fn to_flag(&self, name: FlagName) -> Option<Flag> {
        match self {
            FeatureFlag::Enabled => Flag::new(name, true, 100).ok(),
            FeatureFlag::Disabled => Flag::new(name, false, 0).ok(),
            FeatureFlag::Percentage(pct) => Flag::new(name, true, *pct).ok(),
            FeatureFlag::TenantOnly(_) | FeatureFlag::UserOnly(_) => None,
        }
    }

    /// Create a wrapper from a `flag_kit::Flag`.
    ///
    /// Maps `enabled == false` → `Disabled`, `percentage == 100` → `Enabled`,
    /// otherwise `Percentage(percentage)`. This is the inverse of `to_flag`.
    pub fn from_flag(flag: &Flag) -> Self {
        if !flag.enabled {
            FeatureFlag::Disabled
        } else if flag.percentage == 100 {
            FeatureFlag::Enabled
        } else if flag.percentage == 0 {
            // enabled true but 0% rollout is effectively disabled for generic checks;
            // keep as Percentage(0) to preserve rollout semantics.
            FeatureFlag::Percentage(0)
        } else {
            FeatureFlag::Percentage(flag.percentage)
        }
    }
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

// ---------------------------------------------------------------------------
// FeatureFlags — wraps MemoryFlagStore + FlagStore, delegates to Flag
// ---------------------------------------------------------------------------

/// Thread-safe feature flag evaluator backed by `flag_kit::MemoryFlagStore`.
///
/// Holds a snapshot of all flags behind a `RwLock` for legacy `TenantOnly`/
/// `UserOnly` plus an `Arc<MemoryFlagStore>` for `FlagStore` operations.
/// The store is wrapped synchronously via `pollster::block_on` for the
/// existing sync API; async callers can use `store()` / `evaluator()`
/// and `enabled_for_async`.
///
/// For v1 `MemoryFlagStore` is used directly. A future `sqlite` feature will
/// allow `flag_kit::SqliteFlagStore` behind the same `FlagStore` trait and
/// `FlagChange` audit trail — the `FeatureFlags` API will not need to change.
pub struct FeatureFlags {
    store: Arc<MemoryFlagStore>,
    inner: RwLock<FeatureFlagsInner>,
}

struct FeatureFlagsInner {
    flags: HashMap<String, FeatureFlag>,
}

impl FeatureFlags {
    /// Create a new `FeatureFlags` instance from the supplied config.
    ///
    /// Validates each flag name via `FlagName::try_from`. Invalid names are
    /// still stored via `FlagName::new_unchecked` for backward compatibility
    /// (legacy hyphenated defaults like `"webdav-class3"`), but a `tracing::trace`
    /// is emitted. New code should call `is_valid_flag_name` before insertion.
    pub fn from_config(config: &FeatureFlagConfig) -> Self {
        let store = Arc::new(MemoryFlagStore::new());
        let mut flags = HashMap::with_capacity(config.flags.len());

        for (k, v) in &config.flags {
            // Validate via FlagName::try_from; keep legacy hyphenated names via unchecked.
            let flag_name = match FlagName::try_from(k.as_str()) {
                Ok(n) => n,
                Err(e) => {
                    trace!(flag = k, error = %e, "invalid flag name, storing via new_unchecked for backward compat");
                    FlagName::new_unchecked(k.clone())
                }
            };

            // Populate MemoryFlagStore for representable variants.
            if let Some(flag) = v.to_flag(flag_name) {
                // `MemoryFlagStore::set` is async; block synchronously for sync constructor.
                let _ = pollster::block_on(store.set(flag));
            }
            // Keep full wrapper in HashMap for TenantOnly/UserOnly and sync reads.
            flags.insert(k.clone(), v.clone());
        }

        Self {
            store,
            inner: RwLock::new(FeatureFlagsInner { flags }),
        }
    }

    /// Returns the underlying `MemoryFlagStore` (for `FlagStore` trait usage).
    pub fn store(&self) -> Arc<MemoryFlagStore> {
        self.store.clone()
    }

    /// Returns an `Evaluator` wrapping the underlying store.
    ///
    /// Use `evaluator.enabled_for(&FlagName, user_id, org_id).await` for
    /// deterministic rollout checks.
    pub fn evaluator(&self) -> flag_kit::Evaluator {
        flag_kit::Evaluator::new(self.store.clone())
    }

    /// Check if a flag is enabled (unconditionally or via percentage).
    ///
    /// For `Percentage`, uses `flag_kit::bucket(flag_name, "") < pct` (generic
    /// rollout without user context). Returns `false` if the flag does not exist.
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

    /// Async variant that delegates to `flag_kit::Evaluator::enabled_for`.
    ///
    /// For `Percentage` flags this is per-user deterministic (`bucket % 100`).
    /// `TenantOnly`/`UserOnly` are handled via the legacy allow-lists.
    pub async fn enabled_for(&self, flag_name: &str, user_id: &str, org_id: Option<&str>) -> bool {
        let snapshot = self.inner.read();
        let flag = match snapshot.flags.get(flag_name) {
            Some(f) => f.clone(),
            None => return false,
        };
        drop(snapshot);

        match flag {
            FeatureFlag::Enabled => true,
            FeatureFlag::Disabled => false,
            FeatureFlag::TenantOnly(_) | FeatureFlag::UserOnly(_) => {
                // Legacy targeting not represented in FlagStore; use sync list check.
                // For generic enabled_for we treat them as disabled; call tenant/user helpers instead.
                false
            }
            FeatureFlag::Percentage(_) => {
                // Delegate to Evaluator via store lookup. Need validated FlagName.
                let validated =
                    FlagName::try_from(flag_name).unwrap_or_else(|_| FlagName::new_unchecked(flag_name.to_string()));
                // If flag not in store (should be), fallback to bucket directly.
                let pct = match &flag {
                    FeatureFlag::Percentage(p) => *p,
                    _ => 0,
                };
                if pct == 0 {
                    return false;
                }
                if pct == 100 {
                    return true;
                }
                // Use Evaluator path: lookup store, then bucket.
                // We already have pct, but verify store consistency.
                let stored = self.store.get(&validated).await;
                let effective_pct = stored.map(|f| f.percentage).unwrap_or(pct);
                let b = bucket(validated.as_str(), user_id);
                // org_id currently logged but not bucketed (flag-kit semantics).
                let _ = org_id;
                b < effective_pct
            }
        }
    }

    /// Check if a flag is enabled for a specific tenant.
    ///
    /// For `TenantOnly` flags the tenant is matched against the list.
    /// For `Percentage` flags, uses `bucket(flag_name, tenant_id)` for deterministic rollout.
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
    /// For `Percentage` flags, uses `flag_kit::bucket(flag_name, user_id)` for deterministic rollout
    /// (same as `Evaluator::enabled_for`).
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
    /// This replaces the entire flag set atomically under a write lock and
    /// repopulates the underlying `MemoryFlagStore` via `FlagStore::set`.
    /// Readers that already hold a read lock will continue to see the old
    /// set until they release it.
    pub fn reload(&mut self, config: FeatureFlagConfig) {
        // Clear store first (MemoryFlagStore has clear()).
        self.store.clear();
        let mut inner = self.inner.write();
        inner.flags.clear();
        for (k, v) in config.flags {
            let flag_name = match FlagName::try_from(k.as_str()) {
                Ok(n) => n,
                Err(e) => {
                    trace!(flag = k, error = %e, "invalid flag name on reload, storing via new_unchecked");
                    FlagName::new_unchecked(k.clone())
                }
            };
            if let Some(flag) = v.to_flag(flag_name) {
                let _ = pollster::block_on(self.store.set(flag));
            }
            inner.flags.insert(k, v);
        }
        trace!("feature flags reloaded");
    }

    /// Evaluate a generic flag (no tenant/user context).
    fn evaluate(&self, flag: &FeatureFlag, flag_name: &str, _tenant: &str, _user: &str) -> bool {
        match flag {
            FeatureFlag::Enabled => true,
            FeatureFlag::Disabled => false,
            FeatureFlag::Percentage(pct) => {
                if *pct == 0 {
                    return false;
                }
                if *pct == 100 {
                    return true;
                }
                // Deterministic hash via flag_kit::bucket (SipHash of flag+user).
                // Generic path has no user, use empty string to keep determinism.
                bucket(flag_name, "") < *pct
            }
            FeatureFlag::TenantOnly(_) => false,
            FeatureFlag::UserOnly(_) => false,
        }
    }

    /// Evaluate a flag with tenant context.
    fn evaluate_tenant(&self, flag: &FeatureFlag, flag_name: &str, tenant_id: &str) -> bool {
        match flag {
            FeatureFlag::Enabled => true,
            FeatureFlag::Disabled => false,
            FeatureFlag::Percentage(pct) => {
                if *pct == 0 {
                    return false;
                }
                if *pct == 100 {
                    return true;
                }
                bucket(flag_name, tenant_id) < *pct
            }
            FeatureFlag::TenantOnly(tenants) => tenants.contains(&tenant_id.to_string()),
            FeatureFlag::UserOnly(_) => false,
        }
    }

    /// Evaluate a flag with user context.
    fn evaluate_user(&self, flag: &FeatureFlag, flag_name: &str, user_id: &str) -> bool {
        match flag {
            FeatureFlag::Enabled => true,
            FeatureFlag::Disabled => false,
            FeatureFlag::Percentage(pct) => {
                if *pct == 0 {
                    return false;
                }
                if *pct == 100 {
                    return true;
                }
                bucket(flag_name, user_id) < *pct
            }
            FeatureFlag::TenantOnly(_) => false,
            FeatureFlag::UserOnly(users) => users.contains(&user_id.to_string()),
        }
    }
}

/// Returns the default feature flags for the Ferro project.
///
/// These are the flags that ship out of the box. Deployments can override
/// or extend them via configuration.
///
/// Note: legacy defaults use kebab-case (`"webdav-class3"`). New flags should
/// use `^[a-z][a-z0-9_]*$` and be validated via `is_valid_flag_name`. The
/// defaults are stored via `FlagName::new_unchecked` for backward compat.
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
        config.flags.insert("test_flag".into(), FeatureFlag::Enabled);
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled("test_flag"));
    }

    #[test]
    fn disabled_flag_is_always_false() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("test_flag".into(), FeatureFlag::Disabled);
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("test_flag"));
    }

    #[test]
    fn percentage_flag_is_deterministic() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("pct_flag".into(), FeatureFlag::Percentage(50));
        let flags = FeatureFlags::from_config(&config);

        let first = flags.is_enabled("pct_flag");
        let second = flags.is_enabled("pct_flag");
        assert_eq!(first, second);
    }

    #[test]
    fn percentage_0_is_never_enabled() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("pct_zero".into(), FeatureFlag::Percentage(0));
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("pct_zero"));
    }

    #[test]
    fn percentage_100_is_always_enabled() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("pct_full".into(), FeatureFlag::Percentage(100));
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled("pct_full"));
    }

    #[test]
    fn tenant_only_matches() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert(
            "tenant_flag".into(),
            FeatureFlag::TenantOnly(vec!["acme".into(), "globex".into()]),
        );
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled_for_tenant("tenant_flag", "acme"));
        assert!(flags.is_enabled_for_tenant("tenant_flag", "globex"));
        assert!(!flags.is_enabled_for_tenant("tenant_flag", "initech"));
    }

    #[test]
    fn tenant_only_ignores_generic_check() {
        let mut config = FeatureFlagConfig::default();
        config
            .flags
            .insert("tenant_flag".into(), FeatureFlag::TenantOnly(vec!["acme".into()]));
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("tenant_flag"));
    }

    #[test]
    fn user_only_matches() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert(
            "user_flag".into(),
            FeatureFlag::UserOnly(vec!["user-1".into(), "user-2".into()]),
        );
        let flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled_for_user("user_flag", "user-1"));
        assert!(flags.is_enabled_for_user("user_flag", "user-2"));
        assert!(!flags.is_enabled_for_user("user_flag", "user-999"));
    }

    #[test]
    fn user_only_ignores_generic_check() {
        let mut config = FeatureFlagConfig::default();
        config
            .flags
            .insert("user_flag".into(), FeatureFlag::UserOnly(vec!["user-1".into()]));
        let flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("user_flag"));
    }

    #[test]
    fn reload_updates_flags() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("reload_flag".into(), FeatureFlag::Disabled);
        let mut flags = FeatureFlags::from_config(&config);

        assert!(!flags.is_enabled("reload_flag"));

        let mut new_config = FeatureFlagConfig::default();
        new_config.flags.insert("reload_flag".into(), FeatureFlag::Enabled);
        flags.reload(new_config);

        assert!(flags.is_enabled("reload_flag"));
    }

    #[test]
    fn reload_removes_old_flags() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("old_flag".into(), FeatureFlag::Enabled);
        let mut flags = FeatureFlags::from_config(&config);

        assert!(flags.is_enabled("old_flag"));

        let new_config = FeatureFlagConfig::default();
        flags.reload(new_config);

        assert!(!flags.is_enabled("old_flag"));
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

    #[test]
    fn flag_name_validation_via_flag_kit() {
        // Valid snake_case
        assert!(is_valid_flag_name("new_checkout"));
        assert!(is_valid_flag_name("a"));
        assert!(is_valid_flag_name("my_flag_123"));
        // Invalid: hyphens, uppercase, starting digit, empty
        assert!(!is_valid_flag_name("new-checkout"));
        assert!(!is_valid_flag_name("NewFlag"));
        assert!(!is_valid_flag_name("1flag"));
        assert!(!is_valid_flag_name(""));
        assert!(!is_valid_flag_name("webdav-class3")); // legacy hyphenated → invalid per FlagName
        assert!(validate_flag_name("bad-name").is_err());
        assert!(validate_flag_name("good_name").is_ok());
    }

    #[test]
    fn to_flag_delegation() {
        let name = FlagName::new("test_flag").unwrap();
        let flag = FeatureFlag::Enabled.to_flag(name.clone()).unwrap();
        assert_eq!(flag.name, name);
        assert!(flag.enabled);
        assert_eq!(flag.percentage, 100);

        let name2 = FlagName::new("pct").unwrap();
        let pct = FeatureFlag::Percentage(42).to_flag(name2.clone()).unwrap();
        assert_eq!(pct.percentage, 42);

        // TenantOnly has no Flag representation
        assert!(
            FeatureFlag::TenantOnly(vec!["a".into()])
                .to_flag(FlagName::new("t").unwrap())
                .is_none()
        );
    }

    #[test]
    fn bucket_deterministic_for_percentage() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("bucket_flag".into(), FeatureFlag::Percentage(50));
        let flags = FeatureFlags::from_config(&config);
        // is_enabled_for_user should be deterministic and match bucket
        let uid = "user_123";
        let first = flags.is_enabled_for_user("bucket_flag", uid);
        let second = flags.is_enabled_for_user("bucket_flag", uid);
        assert_eq!(first, second);
        // Direct bucket check matches evaluation for non-trivial percentage
        let b = bucket("bucket_flag", uid);
        assert_eq!(first, b < 50);
    }

    #[tokio::test]
    async fn evaluator_enabled_for_matches_bucket() {
        let mut config = FeatureFlagConfig::default();
        config.flags.insert("eval_flag".into(), FeatureFlag::Percentage(50));
        let flags = FeatureFlags::from_config(&config);
        let name = FlagName::new("eval_flag").unwrap();
        // Seed store already has flag via from_config
        let eval = flags.evaluator();
        let uid = "alice";
        let via_eval = eval.enabled_for(&name, uid, None).await;
        let via_bucket = bucket(name.as_str(), uid) < 50;
        assert_eq!(via_eval, via_bucket);
    }

    #[tokio::test]
    async fn store_flagstore_trait() {
        // Verify MemoryFlagStore via FlagStore trait works
        let store = flags_store_helper().await;
        let name = FlagName::new("store_test").unwrap();
        let flag = Flag::new(name.clone(), true, 25).unwrap();
        store.set(flag.clone()).await.unwrap();
        let got = store.get(&name).await.unwrap();
        assert_eq!(got.percentage, 25);
    }

    async fn flags_store_helper() -> Arc<MemoryFlagStore> {
        let cfg = FeatureFlagConfig::default();
        let f = FeatureFlags::from_config(&cfg);
        f.store()
    }
}
