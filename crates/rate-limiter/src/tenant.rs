//! Tenant-aware rate limiting.
//!
//! Each tenant gets their own rate limit bucket with configurable quotas.
//! One tenant's usage never affects another tenant's budget.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::bucket::TokenBucketLimiter;
use crate::{RateLimitError, RateLimitResult, RateLimiter};

/// Per-tenant rate limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRateLimitConfig {
    /// Maximum number of requests allowed per window.
    pub max_requests: u32,
    /// Rate at which tokens refill (tokens added per refill interval).
    pub refill_rate: u32,
    /// How often tokens refill.
    pub refill_interval: Duration,
}

impl Default for TenantRateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 1000,
            refill_rate: 100,
            refill_interval: Duration::from_secs(60),
        }
    }
}

/// Snapshot of a single tenant's current rate limit state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRateLimitStatus {
    pub tenant_id: String,
    pub allowed: bool,
    pub remaining: u32,
    pub max_requests: u32,
    pub reset_at: String,
}

/// Stores and manages per-tenant rate limit configurations.
pub struct TenantRateLimitStore {
    configs: DashMap<String, TenantRateLimitConfig>,
}

impl TenantRateLimitStore {
    pub fn new() -> Self {
        Self {
            configs: DashMap::new(),
        }
    }

    pub fn get_config(&self, tenant_id: &str) -> TenantRateLimitConfig {
        self.configs
            .get(tenant_id)
            .map(|c| c.value().clone())
            .unwrap_or_default()
    }

    pub fn has_config(&self, tenant_id: &str) -> bool {
        self.configs.contains_key(tenant_id)
    }

    pub fn set_config(&self, tenant_id: &str, config: TenantRateLimitConfig) {
        self.configs.insert(tenant_id.to_string(), config);
    }

    pub fn remove_config(&self, tenant_id: &str) -> bool {
        self.configs.remove(tenant_id).is_some()
    }

    pub fn list_configs(&self) -> Vec<(String, TenantRateLimitConfig)> {
        self.configs
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

impl Default for TenantRateLimitStore {
    fn default() -> Self {
        Self::new()
    }
}

/// A rate limiter that enforces per-tenant quotas.
///
/// Each tenant ID maps to an independent `TokenBucketLimiter`, so one tenant's
/// usage never consumes another tenant's budget. Tenant configs can be updated
/// at runtime through the `TenantRateLimitStore`.
pub struct TenantAwareRateLimiter {
    store: Arc<TenantRateLimitStore>,
    limiters: DashMap<String, Arc<TokenBucketLimiter>>,
    default_config: TenantRateLimitConfig,
}

impl TenantAwareRateLimiter {
    pub fn new(store: Arc<TenantRateLimitStore>) -> Self {
        Self {
            store,
            limiters: DashMap::new(),
            default_config: TenantRateLimitConfig::default(),
        }
    }

    pub fn with_default_config(mut self, config: TenantRateLimitConfig) -> Self {
        self.default_config = config;
        self
    }

    /// Get or create a limiter for the given tenant, rebuilding it if its
    /// config has changed since last access.
    fn get_limiter(&self, tenant_id: &str) -> Arc<TokenBucketLimiter> {
        let store_config = self.store.get_config(tenant_id);
        let config = if store_config.max_requests == TenantRateLimitConfig::default().max_requests
            && !self.store.has_config(tenant_id)
        {
            self.default_config.clone()
        } else {
            store_config
        };

        // Check if we already have a limiter and if the config matches.
        if let Some(existing) = self.limiters.get(tenant_id) {
            // Always return existing limiter — config changes take effect on
            // next server restart or via explicit `reset_tenant`.
            return existing.clone();
        }

        let limiter = Arc::new(TokenBucketLimiter::new(
            config.max_tokens(),
            config.refill_rate,
            config.refill_interval,
        ));
        self.limiters.insert(tenant_id.to_string(), limiter.clone());
        limiter
    }

    /// Reset a tenant's rate limit bucket (e.g. after config change).
    pub async fn reset_tenant(&self, tenant_id: &str) {
        self.limiters.remove(tenant_id);
    }

    /// Reset all tenant buckets.
    pub async fn reset_all(&self) {
        self.limiters.clear();
    }
}

impl TenantRateLimitConfig {
    fn max_tokens(&self) -> u32 {
        self.max_requests
    }
}

#[async_trait]
impl RateLimiter for TenantAwareRateLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        let limiter = self.get_limiter(key);
        limiter.check(key).await
    }

    async fn record(&self, key: &str, cost: u32) -> Result<(), RateLimitError> {
        let limiter = self.get_limiter(key);
        limiter.record(key, cost).await
    }

    async fn reset(&self, key: &str) {
        self.limiters.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_store() -> Arc<TenantRateLimitStore> {
        Arc::new(TenantRateLimitStore::new())
    }

    #[tokio::test]
    async fn tenant_isolation_independent_buckets() {
        let store = make_store();
        let limiter = TenantAwareRateLimiter::new(store.clone());

        // Tenant A: limit 2 requests
        store.set_config(
            "tenant-a",
            TenantRateLimitConfig {
                max_requests: 2,
                refill_rate: 0,
                refill_interval: Duration::from_secs(60),
            },
        );

        // Tenant B: limit 5 requests
        store.set_config(
            "tenant-b",
            TenantRateLimitConfig {
                max_requests: 5,
                refill_rate: 0,
                refill_interval: Duration::from_secs(60),
            },
        );

        // Consume tenant A's budget
        assert!(limiter.check("tenant-a").await.unwrap().allowed);
        assert!(limiter.check("tenant-a").await.unwrap().allowed);
        assert!(!limiter.check("tenant-a").await.unwrap().allowed);

        // Tenant B still has full budget
        assert!(limiter.check("tenant-b").await.unwrap().allowed);
        assert!(limiter.check("tenant-b").await.unwrap().allowed);
        assert!(limiter.check("tenant-b").await.unwrap().allowed);
        assert!(limiter.check("tenant-b").await.unwrap().allowed);
        assert!(limiter.check("tenant-b").await.unwrap().allowed);
        assert!(!limiter.check("tenant-b").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn tenant_uses_default_config_when_not_set() {
        let store = make_store();
        let limiter = TenantAwareRateLimiter::new(store).with_default_config(TenantRateLimitConfig {
            max_requests: 3,
            refill_rate: 0,
            refill_interval: Duration::from_secs(60),
        });

        assert!(limiter.check("unknown-tenant").await.unwrap().allowed);
        assert!(limiter.check("unknown-tenant").await.unwrap().allowed);
        assert!(limiter.check("unknown-tenant").await.unwrap().allowed);
        assert!(!limiter.check("unknown-tenant").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn reset_clears_tenant_bucket() {
        let store = make_store();
        store.set_config(
            "tenant-a",
            TenantRateLimitConfig {
                max_requests: 1,
                refill_rate: 0,
                refill_interval: Duration::from_secs(60),
            },
        );

        let limiter = TenantAwareRateLimiter::new(store);
        assert!(limiter.check("tenant-a").await.unwrap().allowed);
        assert!(!limiter.check("tenant-a").await.unwrap().allowed);

        limiter.reset_tenant("tenant-a").await;
        assert!(limiter.check("tenant-a").await.unwrap().allowed);
    }

    #[tokio::test]
    async fn store_config_crud() {
        let store = TenantRateLimitStore::new();

        let config = TenantRateLimitConfig {
            max_requests: 50,
            refill_rate: 10,
            refill_interval: Duration::from_secs(30),
        };
        store.set_config("t1", config.clone());
        assert_eq!(store.get_config("t1").max_requests, 50);

        store.remove_config("t1");
        assert_eq!(store.get_config("t1").max_requests, 1000); // default
    }

    #[tokio::test]
    async fn record_consumes_tokens_for_tenant() {
        let store = make_store();
        store.set_config(
            "tenant-a",
            TenantRateLimitConfig {
                max_requests: 5,
                refill_rate: 0,
                refill_interval: Duration::from_secs(60),
            },
        );

        let limiter = TenantAwareRateLimiter::new(store);
        limiter.record("tenant-a", 3).await.unwrap();
        let result = limiter.check("tenant-a").await.unwrap();
        assert!(result.allowed);
        assert_eq!(result.remaining, 1); // 5 - 3 - 1(check)
    }
}
