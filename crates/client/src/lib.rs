//! Ferro Client SDK
//!
//! Async WebDAV client for Ferro servers, with optional C-FFI for mobile platforms.
//!
//! # Rust Usage
//!
//! ```rust,no_run
//! use ferro_client::FerroClient;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = FerroClient::new("https://ferro.example.com", "my-token")?;
//!     let files = client.list("/").await.unwrap();
//!     for file in &files {
//!         println!("{} ({} bytes)", file.name, file.size);
//!     }
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod types;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use client::FerroClient;
pub use error::ClientError;
pub use ferro_selective_sync::profile::{
    ConflictInfo, ConflictResolution, FilterPreviewRequest, FilterPreviewResponse, RuleDirection,
    SyncProfile, SyncRule,
};
pub use types::{DirectoryInfo, FileEntry, UploadProgress};
