//! Authentication and authorization for the Ferro server.
//!
//! Provides HTTP Basic auth middleware, OIDC token validation, Cedar-based
//! policy authorization, and user management.

pub mod cedar;
pub mod oidc;
#[cfg(feature = "handlers")]
pub mod policies;
pub mod simple_auth;
pub mod users;
