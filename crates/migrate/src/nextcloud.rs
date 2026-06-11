use base64::Engine;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::error::{MigrationError, Result as MigrateResult};

pub struct NextcloudClient {
    http: reqwest::Client,
    url: String,
    username: String,
    #[allow(dead_code)]
    password: String,
}

impl NextcloudClient {
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
        })
    }

    pub async fn validate(&self) -> MigrateResult<()> {
        let resp = self
            .http
            .request(
                reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
                format!("{}/remote.php/dav/files/{}/", self.url, self.username),
            )
            .header("Depth", "0")
            .send()
            .await?;

        if resp.status().as_u16() == 401 {
            return Err(MigrationError::authentication(
                "Nextcloud authentication failed: invalid credentials",
            ));
        }

        if !resp.status().is_success() && resp.status().as_u16() != 207 {
            return Err(MigrationError::connection(format!(
                "Nextcloud WebDAV not reachable at {} (status: {})",
                self.url,
                resp.status()
            )));
        }
        Ok(())
    }

    pub async fn list_directory(&self, user: &str, path: &str) -> MigrateResult<Vec<DavEntry>> {
        let url = format!(
            "{}/remote.php/dav/files/{}/{}",
            self.url,
            user,
            path.trim_start_matches('/')
        );
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
        let url = format!(
            "{}/remote.php/dav/files/{}/{}",
            self.url,
            user,
            path.trim_start_matches('/')
        );
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

    pub fn webdav_base(&self, user: &str) -> String {
        format!("{}/remote.php/dav/files/{}/", self.url, user)
    }
}

#[derive(Debug, Clone)]
pub struct DavEntry {
    pub path: String,
    pub is_collection: bool,
    pub size: u64,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
}

fn base64_engine() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn parse_propfind(xml: &str) -> MigrateResult<Vec<DavEntry>> {
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
                if in_prop && local_name.as_ref() == b"collection" {
                    current_props.insert("resourcetype".to_string(), "<collection/>".to_string());
                }
            }
            Ok(Event::End(ref e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"href" {
                    current_href = text_buf.trim().to_string();
                    capture_text = false;
                } else if local_name.as_ref() == b"response" {
                    if !current_href.is_empty() {
                        let is_collection = current_props
                            .get("resourcetype")
                            .map(|v| v.contains("collection"))
                            .unwrap_or(false);
                        let size = current_props
                            .get("getcontentlength")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);

                        entries.push(DavEntry {
                            path: decode_href(&current_href),
                            is_collection,
                            size,
                            last_modified: current_props.get("getlastmodified").cloned(),
                            etag: current_props.get("getetag").cloned(),
                            content_type: current_props.get("getcontenttype").cloned(),
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
            Ok(Event::Text(ref e)) if capture_text => {
                text_buf.push_str(&e.unescape().unwrap_or_default());
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(MigrationError::webdav(format!("XML parse error: {}", e)));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(entries)
}

fn decode_href(href: &str) -> String {
    urlencoding::decode(href)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| href.to_string())
}

mod urlencoding {
    pub fn decode(input: &str) -> Result<String, ()> {
        let mut result = String::new();
        let mut chars = input.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push_str(&hex);
                }
            } else {
                result.push(c);
            }
        }
        Ok(result)
    }
}
