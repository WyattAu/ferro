use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Email configuration for SMTP delivery.
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct EmailConfig {
    /// Whether email sending is enabled.
    pub enabled: bool,
    /// SMTP server hostname.
    pub smtp_host: String,
    /// SMTP server port (587 for STARTTLS, 465 for implicit TLS).
    pub smtp_port: u16,
    /// Optional SMTP AUTH username.
    pub smtp_username: Option<String>,
    /// Optional SMTP AUTH password.
    pub smtp_password: Option<String>,
    /// From address for outgoing emails.
    pub from_address: String,
    /// From display name for outgoing emails.
    pub from_name: String,
}

impl std::fmt::Debug for EmailConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailConfig")
            .field("enabled", &self.enabled)
            .field("smtp_host", &self.smtp_host)
            .field("smtp_port", &self.smtp_port)
            .field("smtp_username", &self.smtp_username)
            .field("smtp_password", &self.smtp_password.as_ref().map(|_| "[REDACTED]"))
            .field("from_address", &self.from_address)
            .field("from_name", &self.from_name)
            .finish()
    }
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

/// Send an email via SMTP using the `mailkit` crate's [`AsyncSmtpProvider`].
///
/// When `config.enabled` is false, logs the message at INFO level and returns Ok.
pub async fn send_email(config: &EmailConfig, msg: &EmailMessage) -> common::error::Result<()> {
    if !config.enabled {
        tracing::info!(
            to = %msg.to,
            subject = %msg.subject,
            "Email notification (SMTP disabled)"
        );
        return Ok(());
    }

    let from_mailbox: lettre::message::Mailbox = format!("{} <{}>", config.from_name, config.from_address)
        .parse()
        .map_err(|e| common::error::FerroError::Internal(format!("Invalid from address: {e}")))?;

    let provider = mailkit::AsyncSmtpProvider::new(
        &config.smtp_host,
        config.smtp_port,
        config.smtp_username.clone(),
        config.smtp_password.clone(),
        from_mailbox,
    )
    .map_err(|e| common::error::FerroError::Internal(format!("SMTP provider init error: {e}")))?;

    let mut builder = mailkit::EmailMessage::builder()
        .from(format!("{} <{}>", config.from_name, config.from_address))
        .to(&msg.to)
        .subject(&msg.subject);

    if let Some(ref html) = msg.body_html {
        builder = builder.html_body(html.clone());
    }

    builder = builder.text_body(msg.body_text.clone());

    let email_msg = builder
        .build()
        .map_err(|e| common::error::FerroError::Internal(format!("Email build error: {e}")))?;

    use mailkit::EmailProvider;

    provider
        .send(&email_msg)
        .await
        .map_err(|e| common::error::FerroError::Internal(format!("SMTP send error: {e}")))?;

    tracing::info!(
        to = %msg.to,
        subject = %msg.subject,
        "Email sent successfully"
    );
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
    async fn test_send_email_disabled_logs_and_returns_ok() {
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

    #[tokio::test]
    async fn test_send_email_enabled_no_smtp_server_returns_error() {
        let config = EmailConfig {
            enabled: true,
            smtp_host: "127.0.0.1".to_string(),
            smtp_port: 25999,
            smtp_username: None,
            smtp_password: None,
            from_address: "noreply@ferro.local".to_string(),
            from_name: "Ferro".to_string(),
        };
        let msg = EmailMessage {
            to: "user@example.com".to_string(),
            subject: "Test".to_string(),
            body_text: "Test body".to_string(),
            body_html: None,
        };
        let result = send_email(&config, &msg).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_email_config_serde_roundtrip() {
        let config = EmailConfig {
            enabled: true,
            smtp_host: "smtp.example.com".to_string(),
            smtp_port: 465,
            smtp_username: Some("user".to_string()),
            smtp_password: Some("pass".to_string()),
            from_address: "ferro@example.com".to_string(),
            from_name: "Ferro Files".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: EmailConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.enabled);
        assert_eq!(parsed.smtp_host, "smtp.example.com");
        assert_eq!(parsed.smtp_port, 465);
        assert_eq!(parsed.smtp_username.as_deref(), Some("user"));
        assert_eq!(parsed.from_address, "ferro@example.com");
    }
}
