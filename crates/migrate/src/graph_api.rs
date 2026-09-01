//! OCIS Graph API client for file discovery.
//!
//! The Graph API (`/graph/v1.0/me/drive/root/children`) provides:
//! - Complete file inventory with sizes (unlike WebDAV which has 404 issues)
//! - Space-aware listing (personal, project, shares)
//! - Standard Microsoft Graph API format

#![allow(non_snake_case)]

use crate::error::{MigrationError, Result as MigrateResult};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDriveItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub file: Option<GraphFile>,
    #[serde(default)]
    pub folder: Option<GraphFolder>,
    #[serde(default)]
    pub deleted: Option<GraphDeleted>,
    #[serde(default)]
    pub createdDateTime: Option<String>,
    #[serde(default)]
    pub lastModifiedDateTime: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphFile {
    #[serde(default)]
    pub mimeType: Option<String>,
    #[serde(default)]
    pub hashes: Option<GraphHashes>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphHashes {
    #[serde(default)]
    pub sha1Hash: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphFolder {
    #[serde(default)]
    pub childCount: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDeleted {
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphListResponse {
    #[serde(default)]
    pub value: Vec<GraphDriveItem>,
    #[serde(default)]
    pub odata_nextLink: Option<String>,
}

#[derive(Clone)]
pub struct GraphApiClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl GraphApiClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
        }
    }

    /// List children of a drive item by path.
    /// Path format: "/Documents", "/Photos/vacation"
    pub async fn list_children(&self, path: &str) -> MigrateResult<Vec<GraphDriveItem>> {
        let encoded_path = path.trim_start_matches('/').replace('/', "%2F");
        let url = format!("{}/graph/v1.0/me/drive/root:/{}:/children", self.base_url, encoded_path);

        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MigrationError::connection(format!(
                "Graph API list_children failed ({}): {}",
                status,
                &body[..body.len().min(200)]
            )));
        }

        let data: GraphListResponse = resp.json().await?;
        Ok(data.value)
    }

    /// List children by item ID.
    pub async fn list_children_by_id(&self, item_id: &str) -> MigrateResult<Vec<GraphDriveItem>> {
        let url = format!("{}/graph/v1.0/me/drive/items/{}/children", self.base_url, item_id);

        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Ok(vec![]); // Some items don't support children listing
        }

        let data: GraphListResponse = resp.json().await?;
        Ok(data.value)
    }

    /// Download a file by path.
    pub async fn download_file(&self, path: &str) -> MigrateResult<Vec<u8>> {
        let encoded_path = path.trim_start_matches('/').replace('/', "%2F");
        let url = format!("{}/graph/v1.0/me/drive/root:/{}:/content", self.base_url, encoded_path);

        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(MigrationError::webdav(format!("Graph API download failed: {}", status)));
        }

        Ok(resp.bytes().await?.to_vec())
    }

    /// List all spaces accessible to the user.
    pub async fn list_spaces(&self) -> MigrateResult<Vec<GraphDrive>> {
        let url = format!("{}/graph/v1.0/me/drives", self.base_url);
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MigrationError::connection(format!(
                "Graph API list_spaces failed ({}): {}",
                status,
                &body[..body.len().min(200)]
            )));
        }

        let data: GraphDriveListResponse = resp.json().await?;
        Ok(data.value)
    }

    /// List all users via Graph API (requires admin token).
    pub async fn list_users(&self) -> MigrateResult<Vec<GraphUser>> {
        let url = format!("{}/graph/v1.0/users", self.base_url);
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MigrationError::connection(format!(
                "Graph API list_users failed ({}): {}",
                status,
                &body[..body.len().min(200)]
            )));
        }

        let data: GraphUserListResponse = resp.json().await?;
        Ok(data.value)
    }

    /// List all groups via Graph API (requires admin token).
    pub async fn list_groups(&self) -> MigrateResult<Vec<GraphGroup>> {
        let url = format!("{}/graph/v1.0/groups", self.base_url);
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MigrationError::connection(format!(
                "Graph API list_groups failed ({}): {}",
                status,
                &body[..body.len().min(200)]
            )));
        }

        let data: GraphGroupListResponse = resp.json().await?;
        Ok(data.value)
    }

    /// List members of a group.
    pub async fn list_group_members(&self, group_id: &str) -> MigrateResult<Vec<GraphUser>> {
        let url = format!("{}/graph/v1.0/groups/{}/members", self.base_url, group_id);
        let resp = self.http.get(&url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Ok(vec![]);
        }

        let data: GraphUserListResponse = resp.json().await?;
        Ok(data.value)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphUser {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub displayName: Option<String>,
    #[serde(default)]
    pub mail: Option<String>,
    #[serde(default)]
    pub userPrincipalName: Option<String>,
    #[serde(default)]
    pub accountEnabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphUserListResponse {
    #[serde(default)]
    pub value: Vec<GraphUser>,
    #[serde(default)]
    pub odata_nextLink: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphGroup {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub displayName: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphGroupListResponse {
    #[serde(default)]
    pub value: Vec<GraphGroup>,
    #[serde(default)]
    pub odata_nextLink: Option<String>,
}

// ── Drive / Space types ───────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDrive {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub driveType: Option<String>,
    #[serde(default)]
    pub driveAlias: Option<String>,
    #[serde(default)]
    pub owner: Option<GraphDriveIdentitySet>,
    #[serde(default)]
    pub quota: Option<GraphDriveQuota>,
    #[serde(default)]
    pub root: Option<GraphDriveRoot>,
    #[serde(default)]
    pub lastModifiedDateTime: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDriveIdentitySet {
    #[serde(default)]
    pub user: Option<GraphDriveIdentity>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDriveIdentity {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub displayName: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDriveQuota {
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub used: Option<u64>,
    #[serde(default)]
    pub remaining: Option<u64>,
    #[serde(default)]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDriveRoot {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub webDavUrl: Option<String>,
    #[serde(default)]
    pub permissions: Option<Vec<GraphDrivePermission>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDrivePermission {
    #[serde(default)]
    pub grantedToIdentities: Option<Vec<GraphDriveIdentitySet>>,
    #[serde(default)]
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphDriveListResponse {
    #[serde(default)]
    pub value: Vec<GraphDrive>,
    #[serde(default)]
    pub odata_nextLink: Option<String>,
}
