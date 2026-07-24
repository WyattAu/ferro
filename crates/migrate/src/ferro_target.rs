use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::json;

use crate::error::{MigrationError, Result as MigrateResult};
use crate::mapper::FerroUser;

pub struct FerroTarget {
    http: reqwest::Client,
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
                path, status, &body[..body.len().min(200)]
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
}
