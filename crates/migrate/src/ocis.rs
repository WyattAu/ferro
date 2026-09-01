use base64::Engine;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

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

/// OIDC credentials for token refresh.
#[derive(Debug, Clone)]
struct OidcCredentials {
    token_endpoint: String,
    client_id: String,
    username: String,
    password: String,
}

#[derive(Clone)]
pub struct OcisClient {
    http: Arc<RwLock<reqwest::Client>>,
    url: String,
    #[allow(dead_code)]
    username: String,
    auth: AuthMethod,
    webdav_base: String,
    /// OIDC credentials for token refresh (None for Basic auth or PAT)
    oidc_creds: Option<OidcCredentials>,
    /// Token expiration time (instant when token expires)
    token_expires: Arc<RwLock<Option<std::time::Instant>>>,
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
    expires_in: Option<u64>,
}

impl OcisClient {
    /// Get the bearer token if available (for use with Graph API).
    pub fn token(&self) -> &str {
        match &self.auth {
            AuthMethod::Bearer(token) => token,
            AuthMethod::Basic { .. } => "",
        }
    }

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
            http: Arc::new(RwLock::new(http)),
            url: url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            auth: AuthMethod::Basic {
                username: username.to_string(),
                password: password.to_string(),
            },
            webdav_base: "/dav/files".to_string(),
            oidc_creds: None,
            token_expires: Arc::new(RwLock::new(None)),
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
            http: Arc::new(RwLock::new(http)),
            url: url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            auth: AuthMethod::Bearer(token.to_string()),
            webdav_base: "/dav/files".to_string(),
            oidc_creds: None,
            token_expires: Arc::new(RwLock::new(None)),
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

        let expires_in = token_resp.expires_in.unwrap_or(3600);
        tracing::info!(
            "OIDC token acquired (type={}, expires_in={:?})",
            token_resp.token_type,
            token_resp.expires_in
        );

        let expires = std::time::Instant::now() + std::time::Duration::from_secs(expires_in);

        let mut client = Self::with_token(url, username, &token_resp.access_token)?;
        client.oidc_creds = Some(OidcCredentials {
            token_endpoint: discovery.token_endpoint,
            client_id: oidc_client_id.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        });
        *client.token_expires.write().await = Some(expires);

        Ok(client)
    }

    /// Try to connect to oCIS using the best available auth method.
    ///
    /// Tries:
    /// 1. Existing auth (already set in headers)
    /// 2. If Basic, also attempts without auth to detect the server's auth mode
    pub async fn validate(&self, user: &str) -> MigrateResult<()> {
        self.ensure_token_valid().await?;
        let url = format!("{}/{}/{}/", self.url, self.webdav_base.trim_start_matches('/'), user);
        let http = self.http.read().await;
        let resp = http
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "0")
            .send()
            .await?;
        drop(http);

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

    /// Set up OIDC refresh credentials for a token-based client.
    /// This allows the client to refresh the token before it expires.
    pub async fn set_oidc_refresh(&mut self, url: &str, username: String, password: String, client_id: String) {
        // Discover OIDC token endpoint
        let discovery_url = format!("{}/.well-known/openid-configuration", url);
        if let Ok(discovery) = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .and_then(|c| Ok(c))
        {
            if let Ok(resp) = discovery.get(&discovery_url).send().await {
                if let Ok(doc) = resp.json::<OidcDiscovery>().await {
                    self.oidc_creds = Some(OidcCredentials {
                        token_endpoint: doc.token_endpoint,
                        client_id,
                        username,
                        password,
                    });
                    // Set initial expiration to 60 seconds (token lifetime is 5 min)
                    // This ensures the first request triggers a refresh
                    *self.token_expires.write().await =
                        Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
                    tracing::info!(
                        "OIDC refresh configured (token_endpoint={})",
                        self.oidc_creds.as_ref().unwrap().token_endpoint
                    );
                    return;
                }
            }
        }
        tracing::warn!("Could not configure OIDC refresh — token will not be auto-refreshed");
    }

    /// Refresh the Bearer token if it's about to expire or has expired.
    /// When OIDC credentials are available, refresh aggressively to avoid 401 errors.
    async fn ensure_token_valid(&self) -> MigrateResult<()> {
        if self.oidc_creds.is_some() {
            let expires = self.token_expires.read().await;
            if let Some(exp) = *expires {
                // Refresh if less than 60 seconds remaining
                // OCIS tokens last 5 min, refresh at ~4 min remaining
                if std::time::Instant::now() + std::time::Duration::from_secs(60) < exp {
                    drop(expires);
                    return Ok(());
                }
            }
            drop(expires);
        }

        // Try OIDC refresh if credentials available
        if let Some(ref creds) = self.oidc_creds {
            tracing::info!("Refreshing OIDC token...");
            let http = reqwest::Client::builder().danger_accept_invalid_certs(true).build()?;
            let token_resp: OidcTokenResponse = http
                .post(&creds.token_endpoint)
                .form(&[
                    ("grant_type", "password"),
                    ("client_id", &creds.client_id),
                    ("username", &creds.username),
                    ("password", &creds.password),
                    ("scope", "openid profile email"),
                ])
                .send()
                .await?
                .json()
                .await
                .map_err(|e| MigrationError::authentication(format!("Token refresh failed: {}", e)))?;

            let expires_in = token_resp.expires_in.unwrap_or(3600);
            let new_expires = std::time::Instant::now() + std::time::Duration::from_secs(expires_in);

            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token_resp.access_token))
                    .map_err(|e| MigrationError::config(e.to_string()))?,
            );

            let new_http = reqwest::Client::builder()
                .danger_accept_invalid_certs(true)
                .default_headers(headers)
                .build()?;

            *self.http.write().await = new_http;
            *self.token_expires.write().await = Some(new_expires);
            tracing::info!("Token refreshed (expires in {}s)", expires_in);
            return Ok(());
        }

        // No OIDC credentials — try to detect JWT expiration from the current token
        // and re-acquire using the OIDC token endpoint if we know the issuer
        if let AuthMethod::Bearer(ref token) = self.auth {
            if token.starts_with("eyJ") {
                // Try to decode JWT header to get kid, then find the issuer
                // For now, just warn and continue
                tracing::warn!(
                    "Bearer token may expire soon but no OIDC credentials for refresh. \
                    Consider using --oidc-client-id for automatic token refresh."
                );
            }
        }

        Ok(())
    }

    /// Create a Graph API client using the current OIDC token.
    pub fn graph_client(&self) -> crate::error::Result<crate::graph_api::GraphApiClient> {
        let token = match &self.auth {
            AuthMethod::Bearer(t) => t.clone(),
            _ => return Err(crate::MigrationError::authentication("Graph API requires Bearer token")),
        };
        Ok(crate::graph_api::GraphApiClient::new(&self.url, &token))
    }

    fn webdav_url(&self, user: &str, path: &str) -> String {
        // PROPFIND returns full paths like /dav/files/wyatt/Documents/
        // Strip the base prefix to get relative path, then reconstruct URL
        let base_prefix = format!("{}/{}", self.webdav_base.trim_start_matches('/'), user);
        let clean_path = path
            .trim_start_matches('/')
            .strip_prefix(&base_prefix)
            .unwrap_or_else(|| path.trim_start_matches('/'))
            .trim_start_matches('/');

        // Don't percent-encode — reqwest handles UTF-8 URLs correctly.
        // Our manual encoding was double-encoding UTF-8 bytes (treating them as Latin-1).
        format!(
            "{}/{}/{}/{}",
            self.url,
            self.webdav_base.trim_start_matches('/'),
            user,
            clean_path
        )
    }

    pub async fn list_directory(&self, user: &str, path: &str) -> MigrateResult<Vec<DavEntry>> {
        self.ensure_token_valid().await?;
        let url = self.webdav_url(user, path);
        tracing::debug!("PROPFIND {}", url);
        let http = self.http.read().await;
        let resp = http
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "1")
            .send()
            .await?;
        drop(http);

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
        self.ensure_token_valid().await?;
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
        self.ensure_token_valid().await?;
        let url = self.webdav_url(user, path);
        tracing::debug!("DOWNLOAD {}", url);
        let http = self.http.read().await;
        let resp = http.get(&url).send().await?;
        drop(http);

        if !resp.status().is_success() {
            return Err(MigrationError::webdav(format!(
                "GET {} failed: {}",
                path,
                resp.status()
            )));
        }

        Ok(resp.bytes().await?.to_vec())
    }

    /// List all shares via OCS API.
    pub async fn list_shares(&self) -> MigrateResult<Vec<OcsShare>> {
        self.ensure_token_valid().await?;
        let url = format!(
            "{}/ocs/v2.php/apps/files_sharing/api/v1/shares?format=json&reshare=false",
            self.url
        );
        let http = self.http.read().await;
        let resp = http
            .get(&url)
            .header("OCS-APIRequest", "true")
            .header("Accept", "application/json")
            .send()
            .await?;
        drop(http);

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MigrationError::connection(format!(
                "OCS shares API failed ({}): {}",
                status,
                &body[..body.len().min(200)]
            )));
        }

        let data: OcsShareResponse = resp.json().await?;
        Ok(data.ocs.data)
    }

    /// List directory contents with extended metadata (tags, favorites) via PROPFIND.
    pub async fn list_with_metadata(&self, user: &str, path: &str) -> MigrateResult<Vec<DavMetadataEntry>> {
        self.ensure_token_valid().await?;
        let url = self.webdav_url(user, path);
        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
  <d:allprop/>
  <d:prop>
    <oc:tags/>
    <oc:favorite/>
  </d:prop>
</d:propfind>"#;

        let http = self.http.read().await;
        let resp = http
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .body(body)
            .send()
            .await?;
        drop(http);

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MigrationError::webdav(format!(
                "PROPFIND metadata failed ({}): {}",
                status,
                &body[..body.len().min(200)]
            )));
        }

        let xml = resp.text().await?;
        parse_metadata_propfind(&xml)
    }

    /// Build a WebDAV URL for a project space.
    pub fn space_webdav_url(&self, space_id: &str, path: &str) -> String {
        let clean_path = path
            .trim_start_matches('/');
        format!(
            "{}/dav/spaces/{}/{}",
            self.url,
            space_id,
            clean_path
        )
    }

    /// List directory contents of a project space via PROPFIND.
    pub async fn list_space_contents(&self, space_id: &str, path: &str) -> MigrateResult<Vec<DavEntry>> {
        self.ensure_token_valid().await?;
        let url = self.space_webdav_url(space_id, path);
        tracing::debug!("PROPFIND (space) {}", url);
        let http = self.http.read().await;
        let resp = http
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "1")
            .send()
            .await?;
        drop(http);

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MigrationError::webdav(format!(
                "PROPFIND space {} failed ({}): {}",
                space_id,
                status,
                &body[..body.len().min(200)]
            )));
        }

        let xml = resp.text().await?;
        crate::webdav::parse_propfind(&xml)
    }

    /// Download a file from a project space.
    pub async fn download_space_file(&self, space_id: &str, path: &str) -> MigrateResult<Vec<u8>> {
        self.ensure_token_valid().await?;
        let url = self.space_webdav_url(space_id, path);
        tracing::debug!("DOWNLOAD (space) {}", url);
        let http = self.http.read().await;
        let resp = http.get(&url).send().await?;
        drop(http);

        if !resp.status().is_success() {
            return Err(MigrationError::webdav(format!(
                "GET space file failed: {}",
                resp.status()
            )));
        }

        Ok(resp.bytes().await?.to_vec())
    }
}

/// A single entry from an extended PROPFIND response with tags and favorites.
#[derive(Debug, Clone)]
pub struct DavMetadataEntry {
    pub href: String,
    pub is_collection: bool,
    pub size: u64,
    pub tags: Vec<String>,
    pub is_favorite: bool,
}

fn parse_metadata_propfind(xml: &str) -> MigrateResult<Vec<DavMetadataEntry>> {
    let mut entries = Vec::new();
    let mut current_href = String::new();
    let mut current_is_collection = false;
    let mut current_size = 0u64;
    let mut current_tags = Vec::new();
    let mut current_favorite = false;
    let mut in_response = false;

    for line in xml.lines() {
        let trimmed = line.trim();

        if trimmed.contains("<d:response") || trimmed.contains("<D:response") {
            in_response = true;
            current_href.clear();
            current_is_collection = false;
            current_size = 0;
            current_tags.clear();
            current_favorite = false;
        }

        if in_response {
            // Extract href
            if let Some(start) = trimmed.find("<d:href>").or_else(|| trimmed.find("<D:href>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_href = trimmed[s..s + e].to_string();
                    }
                }
            }

            // Check for collection
            if trimmed.contains("<d:collection") || trimmed.contains("<D:collection") {
                current_is_collection = true;
            }

            // Extract content length
            if let Some(start) = trimmed.find("<d:getcontentlength>").or_else(|| trimmed.find("<D:getcontentlength>")) {
                if let Some(s) = trimmed[start..].find('>') {
                    let s = start + s + 1;
                    if let Some(e) = trimmed[s..].find('<') {
                        current_size = trimmed[s..s + e].parse().unwrap_or(0);
                    }
                }
            }

            // Extract tags
            if trimmed.contains("<oc:tags>") || trimmed.contains("<oc:tag>") {
                // Simple extraction: look for <oc:tag>name</oc:tag>
                let mut search = trimmed;
                while let Some(tag_start) = search.find("<oc:tag>") {
                    let s = tag_start + 8;
                    if let Some(e) = search[s..].find("</oc:tag>") {
                        let tag = search[s..s + e].to_string();
                        if !tag.is_empty() {
                            current_tags.push(tag);
                        }
                        search = &search[s + e..];
                    } else {
                        break;
                    }
                }
            }

            // Extract favorite status
            if trimmed.contains("<oc:favorite>") {
                if let Some(start) = trimmed.find("<oc:favorite>") {
                    let s = start + 13;
                    if let Some(e) = trimmed[s..].find("</oc:favorite>") {
                        current_favorite = trimmed[s..s + e].trim() == "1"
                            || trimmed[s..s + e].trim().to_lowercase() == "true";
                    }
                }
            }
        }

        // End of response
        if trimmed.contains("</d:response>") || trimmed.contains("</D:response>") {
            if in_response && !current_href.is_empty() {
                entries.push(DavMetadataEntry {
                    href: current_href.clone(),
                    is_collection: current_is_collection,
                    size: current_size,
                    tags: current_tags.clone(),
                    is_favorite: current_favorite,
                });
            }
            in_response = false;
        }
    }

    Ok(entries)
}

#[derive(Debug, Clone, Deserialize)]
pub struct OcsShareResponse {
    pub ocs: OcsShareData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OcsShareData {
    pub data: Vec<OcsShare>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OcsShare {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub share_type: i32,
    #[serde(default)]
    pub uid_owner: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub share_with: Option<String>,
    #[serde(default)]
    pub permissions: i32,
    #[serde(default)]
    pub share_name: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

fn base64_engine() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}
