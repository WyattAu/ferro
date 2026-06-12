use base64::Engine;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::error::{MigrationError, Result as MigrateResult};
use crate::webdav::{DavEntry, parse_propfind};

pub struct OcisClient {
    http: reqwest::Client,
    url: String,
    #[allow(dead_code)]
    username: String,
    #[allow(dead_code)]
    password: String,
    webdav_base: String,
}

impl OcisClient {
    pub fn new(url: &str, username: &str, password: &str) -> MigrateResult<Self> {
        let mut headers = HeaderMap::new();
        let credentials = format!("{}:{}", username, password);
        let encoded = base64_engine().encode(credentials.as_bytes());
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {}", encoded))
                .map_err(|e| MigrationError::config(e.to_string()))?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http,
            url: url.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password: password.to_string(),
            webdav_base: "/dav/files".to_string(),
        })
    }

    pub fn with_webdav_base(mut self, base: &str) -> Self {
        self.webdav_base = base.trim_end_matches('/').to_string();
        self
    }

    fn webdav_url(&self, user: &str, path: &str) -> String {
        format!(
            "{}/{}/{}/{}",
            self.url,
            self.webdav_base,
            user,
            path.trim_start_matches('/')
        )
    }

    pub async fn validate(&self, user: &str) -> MigrateResult<()> {
        let url = format!("{}/{}/{}/", self.url, self.webdav_base, user);
        let resp = self
            .http
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "0")
            .send()
            .await?;

        if resp.status().as_u16() == 401 {
            return Err(MigrationError::authentication(
                "oCIS authentication failed: invalid credentials",
            ));
        }

        if !resp.status().is_success() && resp.status().as_u16() != 207 {
            return Err(MigrationError::connection(format!(
                "oCIS WebDAV not reachable at {} (status: {})",
                self.url,
                resp.status()
            )));
        }
        Ok(())
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

    pub async fn list_directory_recursive(
        &self,
        user: &str,
        path: &str,
    ) -> MigrateResult<Vec<DavEntry>> {
        let mut all_entries = Vec::new();
        let mut dirs_to_process = vec![path.to_string()];

        while let Some(dir) = dirs_to_process.pop() {
            let entries = self.list_directory(user, &dir).await?;
            for entry in entries.iter().skip(1) {
                if entry.is_collection {
                    dirs_to_process.push(entry.path.clone());
                }
                all_entries.push(entry.clone());
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
