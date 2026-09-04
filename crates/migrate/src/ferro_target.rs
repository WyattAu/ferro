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

        // Self-signed Traefik origins are common for self-hosted targets.
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .default_headers(headers)
            .build()?;

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
        // Ferro requires a password for local auth; migrated users authenticate via
        // OIDC, so generate a random throwaway password.
        let random_password = uuid::Uuid::new_v4().to_string();

        // Ferro's UserRole enum deserializes PascalCase variants ("Admin"/"User"/"ReadOnly").
        let role_pascal = match user.role.to_ascii_lowercase().as_str() {
            "admin" => "Admin",
            "readonly" | "read_only" | "read-only" => "ReadOnly",
            _ => "User",
        };

        let body = json!({
            "username": user.username,
            "email": user.email.clone().unwrap_or_default(),
            "display_name": user.display_name.clone().unwrap_or_else(|| user.username.clone()),
            "role": role_pascal,
            "password": random_password,
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
            "members": [],
        });

        let resp = self
            .http
            .post(format!("{}/api/groups", self.url))
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
        // Resolve group id by name via /api/groups listing
        let resp = self.http.get(format!("{}/api/groups", self.url)).send().await?;
        if !resp.status().is_success() {
            tracing::warn!(
                "Add member '{}' to group '{}': list failed ({})",
                username,
                group_name,
                resp.status()
            );
            return Ok(());
        }
        let body: serde_json::Value = resp.json().await.unwrap_or_default();
        let group_id = body.get("groups").and_then(|g| g.as_array()).and_then(|arr| {
            arr.iter()
                .find(|g| g.get("name").and_then(|n| n.as_str()) == Some(group_name))
                .and_then(|g| g.get("id").and_then(|i| i.as_str()))
                .map(|s| s.to_string())
        });

        let Some(group_id) = group_id else {
            tracing::warn!("Add member '{}' to group '{}': group not found", username, group_name);
            return Ok(());
        };

        let resp = self
            .http
            .post(format!("{}/api/groups/{}/members/{}", self.url, group_id, username))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            tracing::warn!("Add member '{}' to group '{}' failed: {}", username, group_name, status);
        }
        Ok(())
    }

    /// Record a project space member.
    ///
    /// Ferro has no per-user space ACL API — space access is governed by Cedar
    /// policies. This logs the role mapping so operators can tighten Cedar later:
    /// - Manager: read+write+share
    /// - Editor: read+write
    /// - Viewer: read
    pub async fn create_space_member_share(&self, space_path: &str, username: &str, role: &str) -> MigrateResult<()> {
        tracing::info!(
            "Space member (no Ferro ACL API — enforce via Cedar): '{}' role={} on '{}'",
            username,
            role,
            space_path
        );
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

    /// Create a share on the Ferro target.
    ///
    /// Ferro's share model is public link shares (`/api/shares` → ShareLink with
    /// token); it has no per-user file ACL API. OCIS link shares (share_type 3)
    /// map to Ferro link shares; user/group shares (0/1/2) have no equivalent and
    /// are skipped — recipients get access via `/_spaces` + Cedar policies.
    pub async fn create_share(
        &self,
        path: &str,
        share_type: &str,
        shared_with: Option<&str>,
        permissions_read: bool,
        permissions_write: bool,
    ) -> MigrateResult<()> {
        if share_type != "link" {
            tracing::info!(
                "Skipping {} share on '{}' (no Ferro equivalent — use /_spaces or link shares)",
                share_type,
                path
            );
            return Ok(());
        }

        // Dedup: skip if a share already exists for this path.
        let existing: serde_json::Value = self
            .http
            .get(format!("{}/api/shares", self.url))
            .send()
            .await?
            .json()
            .await
            .unwrap_or_default();
        if existing
            .get("shares")
            .and_then(|s| s.as_array())
            .map(|arr| arr.iter().any(|s| s.get("path").and_then(|p| p.as_str()) == Some(path)))
            .unwrap_or(false)
        {
            tracing::debug!("Link share for '{}' already exists — skipping", path);
            return Ok(());
        }

        let body = json!({
            "path": path,
            "allow_download": permissions_read,
            "allow_upload": permissions_write,
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
            tracing::warn!("Link share for '{}' failed ({}): {}", path, status, msg);
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
