//! Composite traits for `AppState` decomposition.
//!
//! These traits allow extracted crates to depend on `ferro-common` instead of
//! `ferro-server`, breaking the circular dependency that prevents crate extraction.
//!
//! # Usage
//!
//! Handler functions can be generic over these traits:
//!
//! ```ignore
//! async fn my_handler<S: HasStorage + HasAudit>(
//!     State(state): State<S>,
//! ) -> impl IntoResponse { ... }
//! ```
//!
//! `AppState` implements all of these traits, so existing code continues to work
//! unchanged during the incremental migration.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::storage::{LockManagerTrait, StorageEngine};

/// Provides access to the server start time for uptime calculation.
pub trait HasUptime: Send + Sync {
    fn started_at(&self) -> std::time::Instant;
}

/// Provides access to the favorites store (list/add/remove).
pub trait HasFavorites: Send + Sync {
    fn list_favorites(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Vec<String>> + Send + '_>>;
    fn add_favorite(&self, path: String) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>>;
    fn remove_favorite(&self, path: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>>;
}

/// Provides access to the storage engine.
pub trait HasStorage: Send + Sync {
    fn storage(&self) -> &Arc<dyn StorageEngine>;
}

/// Provides access to the lock manager.
pub trait HasLockManager: Send + Sync {
    fn lock_manager(&self) -> &Arc<dyn LockManagerTrait>;
}

/// Provides access to the max body size limit.
pub trait HasBodyLimits: Send + Sync {
    fn max_body_size(&self) -> u64;
}

/// Provides access to the maintenance mode flag.
pub trait HasMaintenanceMode: Send + Sync {
    fn maintenance_mode(&self) -> &Arc<AtomicBool>;

    fn is_maintenance(&self) -> bool {
        self.maintenance_mode().load(Ordering::Relaxed)
    }
}

/// Provides access to the startup-complete flag.
pub trait HasStartupState: Send + Sync {
    fn startup_complete(&self) -> &Arc<AtomicBool>;

    fn is_started(&self) -> bool {
        self.startup_complete().load(Ordering::Relaxed)
    }
}

/// Provides access to request-level metrics counters.
pub trait HasMetrics: Send + Sync {
    fn request_count(&self) -> &Arc<AtomicU64>;
    fn storage_op_counts(&self) -> &Arc<[AtomicU64; 6]>;
}

/// Provides access to the external URL for building absolute links.
pub trait HasExternalUrl: Send + Sync {
    fn external_url(&self) -> &str;
}

/// Provides access to the admin credentials.
pub trait HasAdminCreds: Send + Sync {
    fn admin_user(&self) -> Option<&str>;
    fn admin_password(&self) -> Option<&str>;
    fn admin_password_rotated(&self) -> &Arc<AtomicBool>;
}

/// Provides access to the data directory path.
pub trait HasDataDir: Send + Sync {
    fn data_dir(&self) -> Option<&str>;
}

/// Provides access to deduplication configuration.
pub trait HasDedupConfig: Send + Sync {
    fn dedup_enabled(&self) -> bool;
}

/// Provides access to the streaming upload threshold.
pub trait HasStreamingConfig: Send + Sync {
    fn streaming_upload_threshold(&self) -> u64;
}

/// Provides access to the trash store.
pub trait HasTrash: Send + Sync {
    fn trash_dir(&self) -> Option<&str>;
    fn max_file_versions(&self) -> u64;
}

/// Provides access to quota configuration.
pub trait HasQuota: Send + Sync {
    fn quota_bytes(&self) -> Option<u64>;
    fn used_bytes(&self) -> &Arc<AtomicU64>;
    fn file_count(&self) -> &Arc<AtomicU64>;
}

/// Provides access to storage health monitoring.
pub trait HasStorageHealth: Send + Sync {
    fn any_unhealthy(&self) -> bool;
}

/// Provides access to the WOPI integration.
pub trait HasWopi: Send + Sync {
    fn wopi_token_secret(&self) -> &str;
    fn wopi_office_url(&self) -> &str;
}

/// Provides access to the thumbnail configuration.
pub trait HasThumbnailConfig: Send + Sync {
    fn thumbnail_size(&self) -> u32;
}

/// Provides access to rate limiter configuration.
pub trait HasRateLimitConfig: Send + Sync {
    fn rate_limit_burst(&self) -> u32;
    fn rate_limit_refill(&self) -> u32;
    fn max_concurrent_requests(&self) -> usize;
}

/// Provides access to snapshot configuration.
pub trait HasSnapshotConfig: Send + Sync {
    fn max_snapshot_versions(&self) -> usize;
}

// =============================================================================
// TODO Phase 2: Traits to add when store types are moved to ferro-common
// =============================================================================
//
// The following traits require store types (UserStoreTrait, ShareStoreTrait, etc.)
// that are currently defined in ferro-server or ferro-auth. Once those types are
// extracted into ferro-common (or a new ferro-store-traits crate), create these
// traits and implement them for AppState.
//
// ## 1. HasUserStore
//    Used by: admin_api.rs, user_api.rs
//    Field: state.user_store (Arc<dyn UserStoreTrait>)
//    Methods needed: user list, create, delete, update password, authenticate, etc.
//    Dependencies: ferro-auth::UserStoreTrait
//
// ## 2. HasShareStore
//    Used by: shares.rs, batch.rs
//    Field: state.share_store (Arc<dyn ShareStoreTrait>)
//    Methods needed: create, delete, list shares, resolve share links, etc.
//    Dependencies: ferro-server::ShareStoreTrait
//
// ## 3. HasAuditLog
//    Used by: favorites.rs, backup.rs, lib.rs, activity.rs
//    Field: state.audit_log (AuditLog)
//    Methods needed: log events, query activity, etc.
//    Dependencies: ferro-server::AuditLog
//
// ## 4. HasTagStore
//    Used by: tags.rs
//    Field: state.tag_store (Arc<dyn TagStore>)
//    Methods needed: add/remove tags, list tags, filter by tag, etc.
//    Dependencies: ferro-server::TagStore
//
// ## 5. HasCommentStore
//    Used by: comments.rs
//    Field: state.comment_store (Arc<dyn CommentStore>)
//    Methods needed: add/update/delete comments, list comments for file, etc.
//    Dependencies: ferro-server::CommentStore
//
// ## 6. HasFavoriteStore
//    Used by: favorites.rs
//    Field: state.favorites (FavoriteStore)
//    Methods needed: list, add, remove favorites
//    Note: HasFavorites trait already exists (see above), but it delegates to
//          FavoriteStore directly. This trait would expose the store object itself
//          if needed for extracted crate access.
//    Dependencies: ferro-server::FavoriteStore
//
// =============================================================================
