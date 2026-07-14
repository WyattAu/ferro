use base64::Engine;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::Deserialize;

use crate::error::{MigrationError, Result as MigrateResult};
use crate::webdav::{DavEntry, parse_propfind};

/// Authentication method for oCIS WebDAV access.
///
/// oCIS supports two auth mechanisms for WebDAV:
/// 1. **Basic Auth** -- when `PROXY_BASIC_AUTH_ENABLE=true` is set on the server
/// 2. **Bearer Token** -- obtained via OIDC (Keycloak) or oCIS personal access tokens
///
/// The client tries Bearer first, falls back to Basic if a password is provided.
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// Pre-obtained Bearer token (from PAT or OIDC flow)
    Bearer(String),
    /// Basic auth with username:password
    Basic { username: String, password: String },
}

pub struct OcisClient {
    http: reqwest::Client,
    url: String,
    #[allow(dead_code)]
    username: String,
    auth: AuthMethod,
    webdav_base: String,
}

/// OIDC discovery document from `.well-known/openid-configuration`.
#[derive(Deserialize)]
struct OidcDiscovery {
    token_endpoint: String,
    #[allow(dead_code)]
    authorization_endpoint: String,
}

/// OIDC token response.
#[derive(Deserialize)]
struct OidcTokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    expires_in: Option<u64>,
}

impl OcisClient {
    /// Create a new client with Basic Auth (legacy behavior).
    pub fn new(url: &str, username: &str, password: &str) -> MigrateResult<Self> {
        let mut headers = HeaderMap::new();
        let credentials = format!("{}:{}", username, password);
        let encoded = base64_engine().encode(credentials.as_bytes());
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {}", encoded)).map_err(|e| MigrationError::config(e.to_string()))?,
        );

        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http,
            url: url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            auth: AuthMethod::Basic {
                username: username.to_string(),
                password: password.to_string(),
            },
            webdav_base: "/dav/files".to_string(),
        })
    }

    /// Create a new client with a pre-obtained Bearer token (PAT or OIDC).
    pub fn with_token(url: &str, username: &str, token: &str) -> MigrateResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token)).map_err(|e| MigrationError::config(e.to_string()))?,
        );

        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http,
            url: url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            auth: AuthMethod::Bearer(token.to_string()),
            webdav_base: "/dav/files".to_string(),
        })
    }

    /// Create a new client by performing OIDC Resource Owner Password Credentials (ROPC) grant.
    ///
    /// This discovers the OIDC issuer from the oCIS `.well-known/openid-configuration`,
    /// then exchanges username + password for an access token.
    pub async fn with_oidc(url: &str, username: &str, password: &str, oidc_client_id: &str) -> MigrateResult<Self> {
        let base_url = url.trim_end_matches('/');

        // Discover OIDC endpoints
        let discovery_url = format!("{}/.well-known/openid-configuration", base_url);
        let http = reqwest::Client::builder().danger_accept_invalid_certs(true).build()?;

        let discovery: OidcDiscovery = http.get(&discovery_url).send().await?.json().await.map_err(|e| {
            MigrationError::connection(format!("Failed to fetch OIDC discovery from {}: {}", discovery_url, e))
        })?;

        tracing::info!("OIDC discovery: token_endpoint={}", discovery.token_endpoint);

        // Exchange credentials for token via ROPC grant
        let token_resp: OidcTokenResponse = http
            .post(&discovery.token_endpoint)
            .form(&[
                ("grant_type", "password"),
                ("client_id", oidc_client_id),
                ("username", username),
                ("password", password),
                ("scope", "openid profile email"),
            ])
            .send()
            .await?
            .json()
            .await
            .map_err(|e| {
                MigrationError::authentication(format!(
                    "OIDC token exchange failed (ROPC grant). \
                     Ensure ROPC is enabled on the OIDC provider and credentials are correct: {}",
                    e
                ))
            })?;

        tracing::info!(
            "OIDC token acquired (type={}, expires_in={:?})",
            token_resp.token_type,
            token_resp.expires_in
        );

        // Build client with Bearer token
        Self::with_token(url, username, &token_resp.access_token)
    }

    /// Try to connect to oCIS using the best available auth method.
    ///
    /// Tries:
    /// 1. Existing auth (already set in headers)
    /// 2. If Basic, also attempts without auth to detect the server's auth mode
    pub async fn validate(&self, user: &str) -> MigrateResult<()> {
        let url = format!("{}/{}/{}/", self.url, self.webdav_base, user);
        let resp = self
            .http
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "0")
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(MigrationError::authentication(format!(
                "oCIS authentication failed (status {}). \
                 If your oCIS uses OIDC (Keycloak), use --source-token with a personal access token, \
                 or provide --oidc-client-id for automatic token acquisition.",
                status
            )));
        }

        if !status.is_success() && status.as_u16() != 207 {
            return Err(MigrationError::connection(format!(
                "oCIS WebDAV not reachable at {} (status: {})",
                self.url, status
            )));
        }

        tracing::info!("oCIS connection validated (auth method: {:?})", self.auth);
        Ok(())
    }

    pub fn with_webdav_base(mut self, base: &str) -> Self {
        self.webdav_base = base.trim_end_matches('/').to_string();
        self
    }

    fn webdav_url(&self, user: &str, path: &str) -> String {
        // PROPFIND returns full paths like /dav/files/wyatt/Documents/
        // We need to strip the base prefix to avoid double-prefixing
        let base_prefix = format!("{}/{}/", self.webdav_base, user);
        let clean_path = path
            .trim_start_matches('/')
            .strip_prefix(base_prefix.trim_start_matches('/'))
            .unwrap_or(path.trim_start_matches('/'));

        format!("{}/{}/{}/{}", self.url, self.webdav_base, user, clean_path)
    }

    pub async fn list_directory(&self, user: &str, path: &str) -> MigrateResult<Vec<DavEntry>> {
        let url = self.webdav_url(user, path);
        let resp = self
            .http
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "1")
            .send()
            .await?;

        if resp.status().as_u16() != 207 {
            return Err(MigrationError::webdav(format!(
                "PROPFIND {} failed: {}",
                path,
                resp.status()
            )));
        }

        let body = resp.text().await?;
        parse_propfind(&body)
    }

    pub async fn list_directory_recursive(&self, user: &str, path: &str) -> MigrateResult<Vec<DavEntry>> {
        let mut all_entries = Vec::new();
        let mut dirs_to_process = vec![path.to_string()];

        while let Some(dir) = dirs_to_process.pop() {
            match self.list_directory(user, &dir).await {
                Ok(entries) => {
                    for entry in entries.iter().skip(1) {
                        if entry.is_collection {
                            dirs_to_process.push(entry.path.clone());
                        }
                        all_entries.push(entry.clone());
                    }
                }
                Err(e) => {
                    // Virtual directories (like oCIS Shares/) may return 404
                    // on recursive listing -- skip them gracefully
                    tracing::warn!("Skipping directory {}: {}", dir, e);
                }
            }
        }

        all_entries.sort_by(|a, b| match (a.is_collection, b.is_collection) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.path.cmp(&b.path),
        });

        Ok(all_entries)
    }

    pub async fn download_file(&self, user: &str, path: &str) -> MigrateResult<Vec<u8>> {
        let url = self.webdav_url(user, path);
        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(MigrationError::webdav(format!(
                "GET {} failed: {}",
                path,
                resp.status()
            )));
        }

        Ok(resp.bytes().await?.to_vec())
    }
}

fn base64_engine() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}
