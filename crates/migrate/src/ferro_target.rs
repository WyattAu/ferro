use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::json;

use crate::error::{MigrationError, Result as MigrateResult};
use crate::mapper::FerroUser;

#[derive(Clone)]
pub struct FerroTarget {
    pub(crate) http: reqwest::Client,
    url: String,
    #[allow(dead_code)]
    admin_token: String,
}

impl FerroTarget {
    pub fn new(url: &str, admin_token: &str) -> MigrateResult<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", admin_token))
                .map_err(|e| MigrationError::config(e.to_string()))?,
        );

        let http = reqwest::Client::builder().default_headers(headers).build()?;

        Ok(Self {
            http,
            url: url.trim_end_matches('/').to_string(),
            admin_token: admin_token.to_string(),
        })
    }

    pub async fn validate(&self) -> MigrateResult<()> {
        let resp = self.http.get(format!("{}/.well-known/ferro", self.url)).send().await?;

        if !resp.status().is_success() {
            return Err(MigrationError::connection(format!(
                "Ferro target at {} is not reachable",
                self.url
            )));
        }
        Ok(())
    }

    pub async fn create_user(&self, user: &FerroUser) -> MigrateResult<()> {
        let body = json!({
            "username": user.username,
            "email": user.email,
            "display_name": user.display_name,
            "role": user.role,
        });

        let resp = self
            .http
            .post(format!("{}/api/admin/users", self.url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = err_body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(MigrationError::api(format!(
                "Create user '{}' failed ({}): {}",
                user.username, status, msg
            )));
        }
        Ok(())
    }

    pub async fn create_group(&self, name: &str, description: Option<&str>) -> MigrateResult<()> {
        let body = json!({
            "name": name,
            "description": description.unwrap_or(""),
        });

        let resp = self
            .http
            .post(format!("{}/api/admin/groups", self.url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = err_body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            tracing::warn!("Create group '{}' failed ({}): {}", name, status, msg);
        }
        Ok(())
    }

    pub async fn add_group_member(&self, group_name: &str, username: &str) -> MigrateResult<()> {
        let body = json!({
            "username": username,
        });

        let resp = self
            .http
            .post(format!("{}/api/admin/groups/{}/members", self.url, group_name))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            tracing::warn!("Add member '{}' to group '{}' failed: {}", username, group_name, status);
        }
        Ok(())
    }

    /// Create a share for a project space member.
    ///
    /// Maps OCIS roles to Ferro permissions:
    /// - Manager: read=true, write=true, share=true
    /// - Editor: read=true, write=true, share=false
    /// - Viewer: read=true, write=false, share=false
    pub async fn create_space_member_share(&self, space_path: &str, username: &str, role: &str) -> MigrateResult<()> {
        let (read, write, share) = match role {
            "manager" => (true, true, true),
            "editor" => (true, true, false),
            "viewer" => (true, false, false),
            _ => (true, false, false),
        };

        let body = json!({
            "path": space_path,
            "share_type": "user",
            "shared_with": username,
            "permissions": {
                "read": read,
                "write": write,
            },
        });

        let resp = self
            .http
            .post(format!("{}/api/shares", self.url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            tracing::warn!(
                "Share for space member '{}' on '{}' failed ({}): role={}",
                username,
                space_path,
                status,
                role
            );
        }
        Ok(())
    }

    pub async fn create_directory(&self, path: &str) -> MigrateResult<()> {
        let url = format!("{}{}", self.url, path);
        let resp = self
            .http
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url)
            .send()
            .await?;

        if !resp.status().is_success() && resp.status().as_u16() != 405 {
            return Err(MigrationError::webdav(format!(
                "MKCOL {} failed: {}",
                path,
                resp.status()
            )));
        }
        Ok(())
    }

    pub async fn put_file(&self, path: &str, content: &[u8]) -> MigrateResult<()> {
        let url = format!("{}{}", self.url, path);
        tracing::debug!("PUT {} ({} bytes)", url, content.len());
        let resp = self
            .http
            .put(&url)
            .header("Content-Type", "application/octet-stream")
            .body(content.to_vec())
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            tracing::error!("PUT {} failed: {} {}", path, status, &body[..body.len().min(200)]);
            return Err(MigrationError::webdav(format!(
                "PUT {} failed: {} {}",
                path,
                status,
                &body[..body.len().min(200)]
            )));
        }
        tracing::debug!("PUT {} OK ({})", path, status);
        Ok(())
    }

    pub async fn create_share(
        &self,
        path: &str,
        share_type: &str,
        shared_with: Option<&str>,
        permissions_read: bool,
        permissions_write: bool,
    ) -> MigrateResult<()> {
        let body = json!({
            "path": path,
            "share_type": share_type,
            "shared_with": shared_with,
            "permissions": {
                "read": permissions_read,
                "write": permissions_write,
            },
        });

        let resp = self
            .http
            .post(format!("{}/api/shares", self.url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body: serde_json::Value = resp.json().await.unwrap_or_default();
            let msg = err_body.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
            tracing::warn!("Share creation for '{}' failed ({}): {}", path, status, msg);
        }
        Ok(())
    }

    pub async fn apply_tags(&self, path: &str, tags: &[String]) -> MigrateResult<()> {
        if tags.is_empty() {
            return Ok(());
        }
        let body = json!({
            "path": path,
            "tags": tags,
        });

        let resp = self
            .http
            .post(format!("{}/api/files/tags", self.url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!("Tag application for '{}' failed: {}", path, resp.status());
        }
        Ok(())
    }

    pub async fn set_favorite(&self, path: &str, favorite: bool) -> MigrateResult<()> {
        let body = json!({
            "path": path,
            "favorite": favorite,
        });

        let resp = self
            .http
            .post(format!("{}/api/files/favorite", self.url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            tracing::warn!("Set favorite for '{}' failed: {}", path, resp.status());
        }
        Ok(())
    }

    /// Check if a file exists at the target with matching content hash.
    /// Returns `true` if the file exists and hashes match (skip upload).
    pub async fn file_exists_with_hash(&self, path: &str, expected_hash: &str) -> bool {
        let url = format!("{}/api/v1/files{}", self.url, path);
        match self.http.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(info) = resp.json::<FileInfo>().await {
                    if let Some(ref hash) = info.content_hash {
                        return hash == expected_hash;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

/// Response from GET /api/v1/files/{path} for CAS dedup.
#[derive(Debug, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub content_hash: Option<String>,
}
