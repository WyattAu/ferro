//! Session management with token rotation, device tracking, and concurrent session limits.

mod device;
mod error;
mod session;
mod token;

pub use device::DeviceInfo;
pub use error::SessionError;
pub use session::{Session, SessionConfig, SessionId, SessionManager};
pub use token::SessionToken;
