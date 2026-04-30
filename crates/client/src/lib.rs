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
//! async fn main() {
//!     let client = FerroClient::new("https://ferro.example.com", "my-token");
//!     let files = client.list("/").await.unwrap();
//!     for file in &files {
//!         println!("{} ({} bytes)", file.name, file.size);
//!     }
//! }
//! ```

mod client;
mod error;
mod types;

#[cfg(feature = "ffi")]
pub mod ffi;

pub use client::FerroClient;
pub use error::ClientError;
pub use types::{DirectoryInfo, FileEntry, UploadProgress};
