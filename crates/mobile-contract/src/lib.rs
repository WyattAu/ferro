//! This crate defines the API contracts for mobile clients (iOS Files Provider, Android SAF).
//! The actual mobile apps are implemented in Swift (iOS) and Kotlin (Android) — this crate
//! provides shared type definitions and test vectors for the REST API interface.

pub mod api;
pub mod error;
pub mod notifications;
pub mod sync;
