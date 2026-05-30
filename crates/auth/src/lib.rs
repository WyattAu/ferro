//! Authentication and authorization for the Ferro server.
//!
//! Provides HTTP Basic auth middleware, OIDC token validation, Cedar-based
//! policy authorization, and user management.

pub mod cedar;
pub mod keys;
pub mod oidc;
pub mod policies;
pub mod simple_auth;
pub mod totp;
pub mod users;
pub mod webauthn;
