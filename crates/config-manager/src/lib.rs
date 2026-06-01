pub mod config;
pub mod error;
pub mod loader;
pub mod validator;

pub use config::FerroConfig;
pub use error::{ConfigError, ConfigWarning, WarningSeverity};
pub use loader::ConfigLoader;

#[cfg(test)]
mod tests;
