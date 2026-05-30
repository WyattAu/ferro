//! Authentication and authorization for the Ferro server.
//!
//! Provides HTTP Basic auth middleware, OIDC token validation, Cedar-based
//! policy authorization, SAML 2.0 Service Provider support, and user management.

pub mod cedar;
pub mod keys;
pub mod oidc;
pub mod policies;
pub mod saml;
pub mod simple_auth;
pub mod totp;
pub mod users;
pub mod webauthn;
