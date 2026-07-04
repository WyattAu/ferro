use anyhow::Result;
use ferro_common::metadata::{ContentHash, FileMetadata};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use serde::{Deserialize, Serialize};

pub struct FerroClient {
    http: reqwest::Client,
    server_url: String,
    token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser {
    pub username: String,
    pub email: Option<String>,
    pub role: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub webdav: String,
    pub auth: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub subject: String,
    pub issuer: String,
    pub audience: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

impl FerroClient {
    pub fn new(server_url: &str, token: Option<&str>) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(t) = token {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", t))?,
            );
        }

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http,
            server_url: server_url.trim_end_matches('/').to_string(),
            token: token.map(|s| s.to_string()),
        })
    }

    pub fn server_url(&self) -> &str {
        &self.server_url
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/.well-known/ferro", self.server_url);
        let resp = self.http.get(&url).send().await?;
        Ok(resp.status().is_success())
    }

    pub async fn get_capabilities(&self) -> Result<ServerCapabilities> {
        let url = format!("{}/", self.server_url);
        let resp = self
            .http
            .request(reqwest::Method::OPTIONS, &url)
            .send()
            .await?;

        let dav = resp
            .headers()
            .get("DAV")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let auth = if self.token.is_some() {
            "configured"
        } else {
            "none"
        };

        Ok(ServerCapabilities {
            webdav: dav,
            auth: auth.to_string(),
        })
    }

    pub async fn list_files(&self, path: &str, depth: u8) -> Result<Vec<FileMetadata>> {
        let url = format!("{}{}", self.server_url, path);
        let resp = self
            .http
            .request(reqwest::Method::from_bytes(b"PROPFIND")?, &url)
            .header("Depth", depth.to_string())
            .send()
            .await?;

        if resp.status().as_u16() != 207 {
            anyhow::bail!("PROPFIND failed: {}", resp.status());
        }

        let body = resp.text().await?;
        parse_propfind_response(&body)
    }

    pub async fn put_file(&self, path: &str, content: &[u8]) -> Result<()> {
        let url = format!("{}{}", self.server_url, path);
        let resp = self
            .http
            .put(&url)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(content.to_vec())
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("PUT failed: {}", resp.status());
        }
        Ok(())
    }

    pub async fn get_file(&self, path: &str) -> Result<Vec<u8>> {
        let url = format!("{}{}", self.server_url, path);
        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!("GET failed: {}", resp.status());
        }
        Ok(resp.bytes().await?.to_vec())
    }

    pub async fn delete_file(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.server_url, path);
        let resp = self.http.delete(&url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!("DELETE failed: {}", resp.status());
        }
        Ok(())
    }

    pub async fn create_directory(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.server_url, path);
        let resp = self
            .http
            .request(reqwest::Method::from_bytes(b"MKCOL")?, &url)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("MKCOL failed: {}", resp.status());
        }
        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/search?q={}&limit={}", self.server_url, query, limit);
        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!("Search failed: {}", resp.status());
        }

        let body: serde_json::Value = resp.json().await?;
        let results = body
            .get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(results.into_iter().collect())
    }

    pub async fn head_file(&self, path: &str) -> Result<FileMetadata> {
        let url = format!("{}{}", self.server_url, path);
        let resp = self.http.head(&url).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!("HEAD failed: {}", resp.status());
        }

        let size = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        let etag = resp
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let mime_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let is_collection = mime_type == "httpd/unix-directory";

        let last_modified = resp
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| chrono::DateTime::parse_from_rfc2822(v).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        Ok(FileMetadata {
            path: path.to_string(),
            content_hash: ContentHash::from_etag(&etag),
            size,
            mime_type,
            is_collection,
            created_at: last_modified,
            modified_at: last_modified,
            owner: "unknown".to_string(),
            etag,
        })
    }

    pub async fn whoami(&self) -> Result<UserInfo> {
        let url = format!("{}/api/auth/info", self.server_url);
        let resp = self.http.get(&url).send().await?;

        if !resp.status().is_success() {
            // If auth endpoint fails, return anonymous
            return Ok(UserInfo {
                subject: "anonymous".to_string(),
                issuer: "ferro".to_string(),
                audience: "ferro".to_string(),
                email: None,
                name: None,
            });
        }

        let body: serde_json::Value = resp.json().await?;
        let claims = body.get("claims").cloned().unwrap_or_default();

        Ok(UserInfo {
            subject: claims
                .get("sub")
                .and_then(|v| v.as_str())
                .unwrap_or("anonymous")
                .to_string(),
            issuer: claims
                .get("iss")
                .and_then(|v| v.as_str())
                .unwrap_or("ferro")
                .to_string(),
            audience: claims
                .get("aud")
                .and_then(|v| v.as_str())
                .unwrap_or("ferro")
                .to_string(),
            email: claims
                .get("email")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            name: claims
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        })
    }

    // ── Admin user operations ──────────────────────────────────────────

    pub async fn list_users(&self) -> Result<Vec<AdminUser>> {
        let url = format!("{}/api/admin/users", self.server_url);
        let resp = self.http.get(&url).send().await?;
        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await?;
            let users = body
                .get("users")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| serde_json::from_value(v.clone()).ok())
                        .collect()
                })
                .unwrap_or_default();
            return Ok(users);
        }
        if resp.status().as_u16() == 404 {
            anyhow::bail!(
                "Admin users endpoint not available (404). The server may not support user management."
            );
        }
        anyhow::bail!("List users failed: {}", resp.status())
    }

    // ── Share link operations ──────────────────────────────────────────

    pub async fn list_shares(&self) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/shares", self.server_url);
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("List shares failed: {}", resp.status());
        }
        let body: serde_json::Value = resp.json().await?;
        Ok(body
            .get("shares")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect())
    }

    pub async fn create_share(
        &self,
        path: &str,
        expires_hours: Option<u64>,
        password: Option<&str>,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/api/shares", self.server_url);
        let mut body = serde_json::json!({
            "path": path,
            "expires_hours": expires_hours.unwrap_or(24),
        });
        if let Some(pw) = password {
            body["password"] = serde_json::json!(pw);
        }
        let resp = self.http.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Create share failed: {}", resp.status());
        }
        Ok(resp.json().await?)
    }

    pub async fn delete_share(&self, token: &str) -> Result<()> {
        let url = format!("{}/api/shares/{}", self.server_url, token);
        let resp = self.http.delete(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Delete share failed: {}", resp.status());
        }
        Ok(())
    }

    // ── Snapshot operations ────────────────────────────────────────────

    pub async fn list_snapshots(&self) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/snapshots", self.server_url);
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("List snapshots failed: {}", resp.status());
        }
        let body: serde_json::Value = resp.json().await?;
        Ok(body
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect())
    }

    pub async fn create_snapshot(&self) -> Result<serde_json::Value> {
        let url = format!("{}/api/snapshots", self.server_url);
        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({}))
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("Create snapshot failed: {}", resp.status());
        }
        Ok(resp.json().await?)
    }

    pub async fn delete_snapshot(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/snapshots/{}", self.server_url, id);
        let resp = self.http.delete(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Delete snapshot failed: {}", resp.status());
        }
        Ok(())
    }

    pub async fn restore_snapshot(&self, id: &str) -> Result<()> {
        let url = format!("{}/api/snapshots/{}/restore", self.server_url, id);
        let resp = self.http.post(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Restore snapshot failed: {}", resp.status());
        }
        Ok(())
    }

    // ── Policy operations ──────────────────────────────────────────────

    pub async fn list_policies(&self) -> Result<serde_json::Value> {
        let url = format!("{}/api/policies", self.server_url);
        let resp = self.http.get(&url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("List policies failed: {}", resp.status());
        }
        Ok(resp.json().await?)
    }

    pub async fn add_policy(&self, policy: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/policies", self.server_url);
        let body = serde_json::json!({ "policy": policy });
        let resp = self.http.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let error_body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(msg) = error_body.get("error").and_then(|v| v.as_str()) {
                anyhow::bail!("Add policy failed ({}): {}", status, msg);
            }
            anyhow::bail!("Add policy failed: {}", status);
        }
        Ok(resp.json().await?)
    }

    pub async fn remove_policy(&self, policy_id: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/policies", self.server_url);
        let body = serde_json::json!({ "policy_id": policy_id });
        let resp = self.http.delete(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let error_body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(msg) = error_body.get("error").and_then(|v| v.as_str()) {
                anyhow::bail!("Remove policy failed ({}): {}", status, msg);
            }
            anyhow::bail!("Remove policy failed: {}", status);
        }
        Ok(resp.json().await?)
    }

    /// Generic GET returning JSON.
    pub async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.server_url, path);
        let mut req = self.http.get(&url);
        if let Some(token) = &self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", token));
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("GET {} failed: {}", url, resp.status());
        }
        Ok(resp.json().await?)
    }

    /// Generic POST with JSON body returning JSON.
    pub async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.server_url, path);
        let mut req = self.http.post(&url).json(body);
        if let Some(token) = &self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", token));
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let error_body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(msg) = error_body.get("error").and_then(|v| v.as_str()) {
                anyhow::bail!("POST {} failed ({}): {}", url, status, msg);
            }
            anyhow::bail!("POST {} failed: {}", url, status);
        }
        Ok(resp.json().await?)
    }

    /// Generic DELETE.
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.server_url, path);
        let mut req = self.http.delete(&url);
        if let Some(token) = &self.token {
            req = req.header(AUTHORIZATION, format!("Bearer {}", token));
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("DELETE {} failed: {}", url, resp.status());
        }
        Ok(())
    }
}

/// Parse a WebDAV PROPFIND multistatus XML response into a list of FileMetadata.
fn parse_propfind_response(xml: &str) -> Result<Vec<FileMetadata>> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut entries = Vec::new();
    let mut current_href = String::new();
    let mut current_props: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut in_prop = false;
    let mut current_tag = String::new();
    let mut capture_text = false;
    let mut text_buf = String::new();

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"response" {
                    current_href.clear();
                    current_props.clear();
                } else if local_name.as_ref() == b"href" {
                    capture_text = true;
                    text_buf.clear();
                } else if local_name.as_ref() == b"propstat" {
                    // nothing
                } else if local_name.as_ref() == b"prop" {
                    in_prop = true;
                } else if in_prop {
                    current_tag = String::from_utf8_lossy(local_name.as_ref()).to_string();
                    capture_text = true;
                    text_buf.clear();
                }
            }
            Ok(Event::Empty(ref e)) => {
                let local_name = e.local_name();
                // Self-closing <D:collection/> inside resourcetype — store marker
                if in_prop && local_name.as_ref() == b"collection" {
                    current_props.insert("resourcetype".to_string(), "<collection/>".to_string());
                }
                buf.clear();
                continue;
            }
            Ok(Event::End(ref e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"href" {
                    current_href = text_buf.trim().to_string();
                    capture_text = false;
                } else if local_name.as_ref() == b"response" {
                    // End of a response block — emit entry
                    if !current_href.is_empty() {
                        let is_collection = current_props
                            .get("resourcetype")
                            .map(|v| v.contains("collection"))
                            .unwrap_or(false);
                        let size = current_props
                            .get("getcontentlength")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);
                        let etag = current_props.get("getetag").cloned().unwrap_or_default();
                        let mime_type = current_props
                            .get("getcontenttype")
                            .cloned()
                            .unwrap_or_else(|| {
                                if is_collection {
                                    "httpd/unix-directory".to_string()
                                } else {
                                    "application/octet-stream".to_string()
                                }
                            });
                        let modified_at = current_props
                            .get("getlastmodified")
                            .and_then(|v| chrono::DateTime::parse_from_rfc2822(v).ok())
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .unwrap_or_else(chrono::Utc::now);

                        entries.push(FileMetadata {
                            path: current_href.clone(),
                            content_hash: ContentHash::from_etag(&etag),
                            size,
                            mime_type,
                            is_collection,
                            created_at: modified_at,
                            modified_at,
                            owner: "unknown".to_string(),
                            etag,
                        });
                    }
                    in_prop = false;
                } else if in_prop && !current_tag.is_empty() && !text_buf.trim().is_empty() {
                    current_props.insert(current_tag.clone(), text_buf.trim().to_string());
                    capture_text = false;
                    current_tag.clear();
                } else if local_name.as_ref() == b"prop" {
                    in_prop = false;
                }
            }
            Ok(Event::Text(ref e)) => {
                if capture_text {
                    text_buf.push_str(
                        &quick_xml::escape::unescape(std::str::from_utf8(e.as_ref()).unwrap_or(""))
                            .unwrap_or_default(),
                    );
                }
            }
            Ok(
                Event::Decl(_)
                | Event::PI(_)
                | Event::DocType(_)
                | Event::Comment(_)
                | Event::CData(_)
                | Event::GeneralRef(_),
            ) => {}
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("XML parse error: {}", e),
        }
        buf.clear();
    }

    entries.sort_by(|a, b| match (a.is_collection, b.is_collection) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.path.to_lowercase().cmp(&b.path.to_lowercase()),
    });

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_propfind_response() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>/</D:href><D:propstat><D:prop><D:getlastmodified>Tue, 21 Apr 2026 16:45:59 GMT</D:getlastmodified><D:getcontentlength>0</D:getcontentlength><D:getetag>&quot;col-1&quot;</D:getetag><D:getcontenttype>httpd/unix-directory</D:getcontenttype><D:resourcetype><D:collection/></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response><D:response><D:href>/test.txt</D:href><D:propstat><D:prop><D:getlastmodified>Tue, 21 Apr 2026 16:45:59 GMT</D:getlastmodified><D:getcontentlength>5</D:getcontentlength><D:getetag>&quot;abc123&quot;</D:getetag><D:getcontenttype>application/octet-stream</D:getcontenttype><D:resourcetype></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
        let entries = parse_propfind_response(xml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "/");
        assert!(entries[0].is_collection);
        assert_eq!(entries[0].size, 0);
        assert_eq!(entries[1].path, "/test.txt");
        assert!(!entries[1].is_collection);
        assert_eq!(entries[1].size, 5);
    }

    #[test]
    fn test_parse_propfind_empty() {
        let xml = r#"<?xml version="1.0"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>/</D:href><D:propstat><D:prop></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
        let entries = parse_propfind_response(xml).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "/");
    }

    #[test]
    fn test_parse_propfind_single_line() {
        // The actual server returns everything on one line
        let xml = "<?xml version=\"1.0\" encoding=\"utf-8\"?><D:multistatus xmlns:D=\"DAV:\"><D:response><D:href>/</D:href><D:propstat><D:prop><D:getlastmodified>Tue, 21 Apr 2026 16:45:59 GMT</D:getlastmodified><D:getcontentlength>0</D:getcontentlength><D:getetag>&quot;col-1776789959493&quot;</D:getetag><D:getcontenttype>httpd/unix-directory</D:getcontenttype><D:resourcetype><D:collection/></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response><D:response><D:href>/test.txt</D:href><D:propstat><D:prop><D:getlastmodified>Tue, 21 Apr 2026 16:45:59 GMT</D:getlastmodified><D:getcontentlength>5</D:getcontentlength><D:getetag>&quot;2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824&quot;</D:getetag><D:getcontenttype>application/octet-stream</D:getcontenttype><D:resourcetype></D:resourcetype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>";
        let entries = parse_propfind_response(xml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].path, "/test.txt");
        assert_eq!(entries[1].size, 5);
    }

    #[test]
    fn test_parse_propfind_nested_collections() {
        let xml = r#"<?xml version="1.0"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>/</D:href><D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype><D:getlastmodified>Tue, 21 Apr 2026 00:00:00 GMT</D:getlastmodified><D:getcontentlength>0</D:getcontentlength><D:getetag>&quot;root&quot;</D:getetag><D:getcontenttype>httpd/unix-directory</D:getcontenttype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response><D:response><D:href>/docs</D:href><D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype><D:getlastmodified>Tue, 21 Apr 2026 00:00:00 GMT</D:getlastmodified><D:getcontentlength>0</D:getcontentlength><D:getetag>&quot;col-docs&quot;</D:getetag><D:getcontenttype>httpd/unix-directory</D:getcontenttype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response><D:response><D:href>/docs/readme.md</D:href><D:propstat><D:prop><D:resourcetype></D:resourcetype><D:getlastmodified>Tue, 21 Apr 2026 00:00:00 GMT</D:getlastmodified><D:getcontentlength>42</D:getcontentlength><D:getetag>&quot;readme&quot;</D:getetag><D:getcontenttype>text/markdown</D:getcontenttype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
        let entries = parse_propfind_response(xml).unwrap();
        assert_eq!(entries.len(), 3);
        // Collections should sort before files
        assert!(entries[0].is_collection);
        assert_eq!(entries[0].path, "/");
        assert!(entries[1].is_collection);
        assert_eq!(entries[1].path, "/docs");
        assert!(!entries[2].is_collection);
        assert_eq!(entries[2].path, "/docs/readme.md");
        assert_eq!(entries[2].size, 42);
        assert_eq!(entries[2].mime_type, "text/markdown");
    }

    #[test]
    fn test_parse_propfind_empty_collection() {
        // A collection with no children
        let xml = r#"<?xml version="1.0"?><D:multistatus xmlns:D="DAV:"><D:response><D:href>/empty-dir</D:href><D:propstat><D:prop><D:resourcetype><D:collection/></D:resourcetype><D:getlastmodified>Tue, 21 Apr 2026 00:00:00 GMT</D:getlastmodified><D:getcontentlength>0</D:getcontentlength><D:getetag>&quot;col-empty&quot;</D:getetag><D:getcontenttype>httpd/unix-directory</D:getcontenttype></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response></D:multistatus>"#;
        let entries = parse_propfind_response(xml).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_collection);
        assert_eq!(entries[0].path, "/empty-dir");
    }

    #[test]
    fn test_client_construction() {
        let client = FerroClient::new("http://localhost:8080", None).unwrap();
        assert_eq!(client.server_url(), "http://localhost:8080");
        assert!(client.token.is_none());

        let client_with_token =
            FerroClient::new("http://localhost:8080/", Some("test-token")).unwrap();
        assert_eq!(client_with_token.server_url(), "http://localhost:8080");
        assert_eq!(client_with_token.token.as_deref(), Some("test-token"));
    }

    #[test]
    fn test_client_trims_trailing_slash() {
        let client = FerroClient::new("http://example.com/api/", None).unwrap();
        assert_eq!(client.server_url(), "http://example.com/api");
    }

    #[test]
    fn test_user_info_serialization() {
        let info = UserInfo {
            subject: "alice".to_string(),
            issuer: "https://auth.example.com".to_string(),
            audience: "ferro".to_string(),
            email: Some("alice@example.com".to_string()),
            name: Some("Alice Smith".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: UserInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.subject, "alice");
        assert_eq!(parsed.email, Some("alice@example.com".to_string()));
    }
}
