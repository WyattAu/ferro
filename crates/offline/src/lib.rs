//! Offline-first local storage with change queuing and server reconciliation.
//!
//! Provides a write-ahead log for file operations when disconnected from the server,
//! and a reconciliation engine for syncing changes back when connectivity is restored.
//!
//! ## Architecture
//!
//! ```text
//! Client App → OfflineStore (implements StorageEngine-like API)
//!               │
//!               ├─ Online  → passthrough to remote StorageEngine
//!               └─ Offline → write to local SQLite WAL + content cache
//!
//! Reconnect → Reconciler.compare_and_sync()
//!   1. Block-diff local vs remote chunk hashes
//!   2. Upload missing chunks
//!   3. Download new remote changes
//!   4. Resolve conflicts using ConflictDetector
//! ```

pub mod cache;
pub mod change_queue;
pub mod crypto;
pub mod error;
pub mod monitor;
pub mod reconciler;
