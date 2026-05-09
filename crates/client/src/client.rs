use crate::error::ClientError;
use crate::types::{DirectoryInfo, FileEntry, ServerInfo};
use reqwest::header::CONTENT_TYPE;
use roxmltree::Document;

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct FerroClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl FerroClient {
    pub fn new(server_url: &str, token: &str) -> Self {
        let base_url = server_url.trim_end_matches('/').to_string();
        let http = reqwest::Client::builder()
            .user_agent("ferro-client/0.1.0")
            .pool_max_idle_per_host(4)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url,
            token: token.to_string(),
            http,
        }
    }

    pub fn with_client(server_url: &str, token: &str, http: reqwest::Client) -> Self {
        let base_url = server_url.trim_end_matches('/').to_string();
        Self {
            base_url,
            token: token.to_string(),
            http,
        }
    }

    pub fn server_url(&self) -> &str {
        &self.base_url
    }

    pub async fn test_connection(&self) -> Result<ServerInfo, ClientError> {
        let url = format!("{}/", self.base_url);
        let response = self.propfind_raw(&url, "0").await?;

        if response.status().as_u16() == 401 {
            return Err(ClientError::AuthFailed);
        }

        let body = response.text().await.map_err(ClientError::Network)?;
        let entries = parse_multistatus(&body, "/");
        let count = entries.len() as u64;

        Ok(ServerInfo {
            server_url: self.base_url.clone(),
            root_entries: count,
            is_authenticated: true,
        })
    }

    pub async fn list(&self, path: &str) -> Result<Vec<FileEntry>, ClientError> {
        let url = self.build_url(path);
        let response = self.propfind_raw(&url, "1").await?;
        let body = response.text().await.map_err(ClientError::Network)?;
        let entries = parse_multistatus(&body, path);
        Ok(entries)
    }

    pub async fn list_directory(&self, path: &str) -> Result<DirectoryInfo, ClientError> {
        let entries = self.list(path).await?;
        let total_size: u64 = entries.iter().map(|e| e.size).sum();
        Ok(DirectoryInfo {
            path: path.to_string(),
            entries,
            total_size,
        })
    }

    pub async fn list_recursive(&self, path: &str) -> Result<Vec<FileEntry>, ClientError> {
        let url = self.build_url(path);
        let response = self.propfind_raw(&url, "infinity").await?;
        let body = response.text().await.map_err(ClientError::Network)?;
        let entries = parse_multistatus(&body, path);
        Ok(entries)
    }

    pub async fn metadata(&self, path: &str) -> Result<FileEntry, ClientError> {
        let url = self.build_url(path);
        let response = self.propfind_raw(&url, "0").await?;
        let body = response.text().await.map_err(ClientError::Network)?;
        let entries = parse_multistatus(&body, path);
        entries
            .into_iter()
            .next()
            .ok_or_else(|| ClientError::NotFound(path.to_string()))
    }

    pub async fn get(&self, path: &str) -> Result<Vec<u8>, ClientError> {
        let url = self.build_url(path);
        let response = self.http.get(&url).bearer_auth(&self.token).send().await?;

        match response.status().as_u16() {
            200 => Ok(response
                .bytes()
                .await
                .map_err(ClientError::Network)?
                .to_vec()),
            404 => Err(ClientError::NotFound(path.to_string())),
            401 => Err(ClientError::AuthFailed),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(ClientError::Http { status, body })
            }
        }
    }

    pub async fn get_text(&self, path: &str) -> Result<String, ClientError> {
        let bytes = self.get(path).await?;
        String::from_utf8(bytes)
            .map_err(|e| ClientError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
    }

    pub async fn put(&self, path: &str, data: &[u8]) -> Result<(), ClientError> {
        let url = self.build_url(path);
        let response = self
            .http
            .put(&url)
            .bearer_auth(&self.token)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await?;

        match response.status().as_u16() {
            201 | 204 => Ok(()),
            401 => Err(ClientError::AuthFailed),
            409 => Err(ClientError::Http {
                status: 409,
                body: "Conflict".to_string(),
            }),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(ClientError::Http { status, body })
            }
        }
    }

    pub async fn put_text(&self, path: &str, content: &str) -> Result<(), ClientError> {
        self.put(path, content.as_bytes()).await
    }

    pub async fn delete(&self, path: &str) -> Result<(), ClientError> {
        let url = self.build_url(path);
        let response = self
            .http
            .delete(&url)
            .bearer_auth(&self.token)
            .send()
            .await?;

        match response.status().as_u16() {
            204 => Ok(()),
            404 => Err(ClientError::NotFound(path.to_string())),
            401 => Err(ClientError::AuthFailed),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(ClientError::Http { status, body })
            }
        }
    }

    pub async fn create_directory(&self, path: &str) -> Result<(), ClientError> {
        let url = self.build_url(path);
        let response = self
            .http
            .request(
                reqwest::Method::from_bytes(b"MKCOL").expect("valid HTTP method"),
                &url,
            )
            .bearer_auth(&self.token)
            .send()
            .await?;

        match response.status().as_u16() {
            201 => Ok(()),
            401 => Err(ClientError::AuthFailed),
            405 => Err(ClientError::Http {
                status: 405,
                body: "Already exists".to_string(),
            }),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(ClientError::Http { status, body })
            }
        }
    }

    pub async fn move_item(&self, from: &str, to: &str) -> Result<(), ClientError> {
        let url = self.build_url(from);
        let destination = self.build_url(to);
        let response = self
            .http
            .request(
                reqwest::Method::from_bytes(b"MOVE").expect("valid HTTP method"),
                &url,
            )
            .bearer_auth(&self.token)
            .header("Destination", &destination)
            .send()
            .await?;

        match response.status().as_u16() {
            201 | 204 => Ok(()),
            401 => Err(ClientError::AuthFailed),
            404 => Err(ClientError::NotFound(from.to_string())),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(ClientError::Http { status, body })
            }
        }
    }

    pub async fn copy(&self, from: &str, to: &str) -> Result<(), ClientError> {
        let url = self.build_url(from);
        let destination = self.build_url(to);
        let response = self
            .http
            .request(
                reqwest::Method::from_bytes(b"COPY").expect("valid HTTP method"),
                &url,
            )
            .bearer_auth(&self.token)
            .header("Destination", &destination)
            .send()
            .await?;

        match response.status().as_u16() {
            201 | 204 => Ok(()),
            401 => Err(ClientError::AuthFailed),
            404 => Err(ClientError::NotFound(from.to_string())),
            status => {
                let body = response.text().await.unwrap_or_default();
                Err(ClientError::Http { status, body })
            }
        }
    }

    pub async fn exists(&self, path: &str) -> Result<bool, ClientError> {
        match self.metadata(path).await {
            Ok(_) => Ok(true),
            Err(ClientError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn build_url(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');
        format!("{}/{}", self.base_url, path)
    }

    async fn propfind_raw(&self, url: &str, depth: &str) -> Result<reqwest::Response, ClientError> {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:getetag/>
    <D:getcontenttype/>
  </D:prop>
</D:propfind>"#;

        let response = self
            .http
            .request(
                reqwest::Method::from_bytes(b"PROPFIND").expect("valid HTTP method"),
                url,
            )
            .bearer_auth(&self.token)
            .header("Depth", depth)
            .header(CONTENT_TYPE, "application/xml")
            .body(body)
            .send()
            .await?;

        Ok(response)
    }
}

fn parse_multistatus(xml: &str, base_path: &str) -> Vec<FileEntry> {
    let doc = match Document::parse(xml) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let base_path_normalized = if base_path.ends_with('/') {
        base_path.to_string()
    } else {
        format!("{}/", base_path)
    };

    let mut entries = Vec::new();

    for node in doc.descendants() {
        if !node.has_tag_name("response") {
            continue;
        }

        let href = match node.children().find(|n| n.has_tag_name("href")) {
            Some(n) => n.text().unwrap_or("").to_string(),
            None => continue,
        };

        let href_normalized = if href.ends_with('/') {
            href.clone()
        } else {
            format!("{}/", href)
        };

        if href == base_path || href_normalized == base_path_normalized {
            continue;
        }

        let mut is_dir = false;
        let mut size: u64 = 0;
        let mut modified = String::new();
        let mut etag = None;
        let mut content_type = None;

        for propstat in node.children().filter(|n| n.has_tag_name("propstat")) {
            for prop in propstat.children().filter(|n| n.has_tag_name("prop")) {
                for child in prop.children() {
                    if child.has_tag_name("resourcetype") {
                        if child.children().any(|c| c.has_tag_name("collection")) {
                            is_dir = true;
                        }
                    } else if child.has_tag_name("getcontentlength") {
                        size = child.text().and_then(|t| t.parse().ok()).unwrap_or(0);
                    } else if child.has_tag_name("getlastmodified") {
                        modified = child.text().unwrap_or("").to_string();
                    } else if child.has_tag_name("getetag") {
                        etag = child.text().map(|s| s.to_string());
                    } else if child.has_tag_name("getcontenttype") {
                        content_type = child.text().map(|s| s.to_string());
                    }
                }
            }
        }

        let name = href
            .trim_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();

        entries.push(FileEntry {
            name,
            path: href,
            size,
            is_dir,
            modified,
            etag,
            content_type,
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UploadProgress;

    #[test]
    fn test_client_new() {
        let client = FerroClient::new("https://example.com", "token");
        assert_eq!(client.server_url(), "https://example.com");
    }

    #[test]
    fn test_client_trailing_slash() {
        let client = FerroClient::new("https://example.com/", "token");
        assert_eq!(client.server_url(), "https://example.com");
    }

    #[test]
    fn test_parse_multistatus_empty() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_multistatus(xml, "/");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_multistatus_files() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/file.txt</D:href>
    <D:propstat>
      <D:prop>
        <D:getcontentlength>1024</D:getcontentlength>
        <D:getlastmodified>Wed, 01 Jan 2024 00:00:00 GMT</D:getlastmodified>
        <D:getetag>"abc"</D:getetag>
        <D:getcontenttype>text/plain</D:getcontenttype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
  <D:response>
    <D:href>/docs/</D:href>
    <D:propstat>
      <D:prop>
        <D:resourcetype><D:collection/></D:resourcetype>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_multistatus(xml, "/");
        assert_eq!(entries.len(), 2);

        let dir = entries.iter().find(|e| e.name == "docs").unwrap();
        assert!(dir.is_dir);

        let file = entries.iter().find(|e| e.name == "file.txt").unwrap();
        assert!(!file.is_dir);
        assert_eq!(file.size, 1024);
        assert_eq!(file.etag, Some("\"abc\"".to_string()));
        assert_eq!(file.content_type, Some("text/plain".to_string()));
    }

    #[test]
    fn test_parse_multistatus_nested_path() {
        let xml = r#"<?xml version="1.0"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/</D:href>
    <D:propstat><D:prop></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  </D:response>
  <D:response>
    <D:href>/a/b/c/deep.txt</D:href>
    <D:propstat><D:prop><D:getcontentlength>99</D:getcontentlength></D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat>
  </D:response>
</D:multistatus>"#;
        let entries = parse_multistatus(xml, "/");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "deep.txt");
        assert_eq!(entries[0].path, "/a/b/c/deep.txt");
    }

    #[test]
    fn test_parse_multistatus_invalid_xml() {
        let entries = parse_multistatus("not xml at all", "/");
        assert!(entries.is_empty());
    }

    #[test]
    fn test_upload_progress() {
        let p = UploadProgress::new(500, 1000);
        assert_eq!(p.percent, 50.0);
        assert_eq!(p.bytes_uploaded, 500);

        let p = UploadProgress::new(0, 0);
        assert_eq!(p.percent, 0.0);
    }

    #[test]
    fn test_error_status_code() {
        let err = ClientError::Http {
            status: 404,
            body: "not found".into(),
        };
        assert_eq!(err.status_code(), Some(404));

        let err = ClientError::AuthFailed;
        assert_eq!(err.status_code(), Some(401));

        let err = ClientError::NotFound("/test".into());
        assert_eq!(err.status_code(), Some(404));

        let err = ClientError::XmlParse("bad".into());
        assert_eq!(err.status_code(), None);
    }
}
