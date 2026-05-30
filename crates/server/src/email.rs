//! Email notification system for file events.

use serde::{Deserialize, Serialize};

use common::error::Result;

/// Email configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub enabled: bool,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub from_address: String,
    pub from_name: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            smtp_host: "localhost".to_string(),
            smtp_port: 587,
            smtp_username: None,
            smtp_password: None,
            from_address: "noreply@ferro.local".to_string(),
            from_name: "Ferro".to_string(),
        }
    }
}

/// An email message to send.
#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub body_text: String,
    pub body_html: Option<String>,
}

/// Send an email. Placeholder -- actual sending requires lettre crate.
pub async fn send_email(_config: &EmailConfig, _msg: &EmailMessage) -> Result<()> {
    tracing::info!(
        to = _msg.to,
        subject = _msg.subject,
        "Email notification (SMTP not yet configured)"
    );
    // TODO: integrate lettre crate for actual SMTP delivery
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_config_default() {
        let config = EmailConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.smtp_host, "localhost");
        assert_eq!(config.smtp_port, 587);
        assert!(config.smtp_username.is_none());
        assert!(config.smtp_password.is_none());
        assert_eq!(config.from_address, "noreply@ferro.local");
        assert_eq!(config.from_name, "Ferro");
    }

    #[tokio::test]
    async fn test_send_email_placeholder() {
        let config = EmailConfig::default();
        let msg = EmailMessage {
            to: "user@example.com".to_string(),
            subject: "Test".to_string(),
            body_text: "Test body".to_string(),
            body_html: None,
        };
        let result = send_email(&config, &msg).await;
        assert!(result.is_ok());
    }
}
