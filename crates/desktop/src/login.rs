//! Interactive desktop login via RFC 8252 loopback flow (Keycloak).
//!
//! Binds an ephemeral `127.0.0.1` port, opens the system browser at the
//! Keycloak authorization URL (PKCE S256), captures the single redirect,
//! exchanges the code, and hands back tokens for the secret store.

use anyhow::{Context, Result};
use oauth_toolkit::{loopback, oidc, token};

/// Default Keycloak issuer for the WyattAu deployment.
pub const DEFAULT_ISSUER: &str = "https://auth.wyattau.com/realms/company-realm";
/// Public PKCE-only client for desktop installs.
pub const DESKTOP_CLIENT_ID: &str = "ferro-desktop";

// offline_access: Keycloak issues a rotation-proof refresh token that
// survives SSO idle expiry — desktop sync must work across days.
const SCOPES: &[&str] = &["openid", "profile", "email", "offline_access"];
const FLOW_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

pub struct LoginTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub username: Option<String>,
}

/// Run the full loopback login. Opens `url` via the platform opener when
/// available; the URL is always printed for manual fallback.
pub async fn run_login(issuer: &str, client_id: &str) -> Result<LoginTokens> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;
    let discovery = oidc::fetch_discovery(&http, issuer)
        .await
        .context("OIDC discovery failed — check the issuer URL")?;

    let flow = loopback::LoopbackFlow::start(
        &discovery.authorization_endpoint,
        client_id,
        &SCOPES.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        None,
        FLOW_TIMEOUT,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    let url = flow.authorization_url().to_string();
    println!("Sign in at:\n  {url}\n");
    try_open_browser(&url);

    let captured = flow
        .wait_for_code()
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("waiting for the browser redirect")?;

    let redirect_uri = loopback::loopback_redirect_uri(captured.port);
    let tokens = token::exchange_code(
        &http,
        &discovery.token_endpoint,
        client_id,
        None, // public client — PKCE only, no secret
        &captured.code,
        &redirect_uri,
        &captured.verifier,
    )
    .await
    .map_err(|e| anyhow::anyhow!("token exchange failed: {e}"))?;

    let username = oidc::fetch_user_info(&http, &discovery.userinfo_endpoint, &tokens.access_token)
        .await
        .ok()
        .map(|u| u.sub);

    Ok(LoginTokens {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        username,
    })
}

fn try_open_browser(url: &str) {
    #[cfg(target_os = "linux")]
    let candidates: &[&str] = &["xdg-open"];
    #[cfg(target_os = "macos")]
    let candidates: &[&str] = &["open"];
    #[cfg(target_os = "windows")]
    let candidates: &[&str] = &["cmd /c start"];

    for candidate in candidates {
        let mut parts = candidate.split_whitespace();
        let Some(prog) = parts.next() else { continue };
        let mut cmd = std::process::Command::new(prog);
        cmd.args(parts).arg(url);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        if cmd.output().is_ok() {
            return;
        }
    }
    // Non-fatal: the URL is printed for manual opening.
}

#[cfg(windows)]
trait CreationFlags {
    fn creation_flags(&mut self, flags: u32) -> &mut Self;
}
#[cfg(windows)]
impl CreationFlags for std::process::Command {
    fn creation_flags(&mut self, flags: u32) -> &mut Self {
        std::os::windows::process::CommandExt::creation_flags(self, flags)
    }
}
