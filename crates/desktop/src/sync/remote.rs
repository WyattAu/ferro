//! Remote state fetcher via WebDAV PROPFIND.
//!
//! Connects to the Ferro server via WebDAV and fetches the file listing
//! with ETags (SHA-256 hashes), sizes, and modification times.

use anyhow::Result;
use std::collections::HashMap;

/// Result of a remote scan.
pub struct RemoteScanResult {
    /// Map of relative_path -> (etag/sha256, size_bytes, mtime_epoch_ms, is_dir).
    pub files: HashMap<String, (String, u64, i64, bool)>,
    /// Number of files found.
    pub file_count: usize,
    /// Number of directories found.
    pub dir_count: usize,
}

/// Fetch the remote file tree via WebDAV PROPFIND (depth: infinity).
pub async fn scan_remote(
    client: &reqwest::Client,
    server_url: &str,
    username: &str,
    password: &str,
    remote_path: &str,
) -> Result<RemoteScanResult> {
    let path_suffix = if remote_path.starts_with('/') {
        remote_path.to_string()
    } else {
        format!("/{}", remote_path)
    };
    let url = format!("{}{}", server_url.trim_end_matches('/'), path_suffix);

    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<d:propfind xmlns:d="DAV:">
  <d:prop>
    <d:getcontentlength/>
    <d:getlastmodified/>
    <d:getetag/>
    <d:resourcetype/>
  </d:prop>
</d:propfind>"#;

    let response = client
        .request(reqwest::Method::from_bytes(b"PROPFIND")?, &url)
        .header("Content-Type", "application/xml; charset=utf-8")
        .header("Depth", "infinity")
        .basic_auth(username, Some(password))
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("PROPFIND failed: {} for {}", response.status(), url);
    }

    let response_body = response.text().await?;
    parse_propfind_response(&response_body, remote_path)
}

/// Parse a WebDAV PROPFIND multistatus XML response.
fn parse_propfind_response(xml: &str, remote_root: &str) -> Result<RemoteScanResult> {
    let mut files = HashMap::new();
    let mut file_count = 0usize;
    let mut dir_count = 0usize;

    let doc = match roxmltree::Document::parse(xml) {
        Ok(d) => d,
        Err(e) => anyhow::bail!("failed to parse PROPFIND XML: {}", e),
    };

    // Find all <d:response> elements
    for response_node in doc
        .descendants()
        .filter(|n| n.has_tag_name(("DAV:", "response")) || n.has_tag_name("response"))
    {
        let mut href = String::new();
        let mut etag = String::new();
        let mut content_length: u64 = 0;
        let mut last_modified = String::new();
        let mut is_dir = false;

        for child in response_node.children() {
            // href
            if child.has_tag_name(("DAV:", "href")) || child.has_tag_name("href") {
                href = child.text().unwrap_or("").to_string();
            }
            // propstat
            if child.has_tag_name(("DAV:", "propstat")) || child.has_tag_name("propstat") {
                for prop_child in child.children() {
                    if prop_child.has_tag_name(("DAV:", "prop")) || prop_child.has_tag_name("prop")
                    {
                        for prop in prop_child.children() {
                            if prop.has_tag_name(("DAV:", "getetag"))
                                || prop.has_tag_name("getetag")
                            {
                                etag = prop.text().unwrap_or("").trim().to_string();
                                // Strip surrounding quotes
                                etag = etag.trim_matches('"').to_string();
                            }
                            if prop.has_tag_name(("DAV:", "getcontentlength"))
                                || prop.has_tag_name("getcontentlength")
                            {
                                content_length = prop.text().unwrap_or("0").parse().unwrap_or(0);
                            }
                            if prop.has_tag_name(("DAV:", "getlastmodified"))
                                || prop.has_tag_name("getlastmodified")
                            {
                                last_modified = prop.text().unwrap_or("").to_string();
                            }
                            if prop.has_tag_name(("DAV:", "resourcetype"))
                                || prop.has_tag_name("resourcetype")
                            {
                                is_dir = prop.children().any(|c| {
                                    c.has_tag_name(("DAV:", "collection"))
                                        || c.has_tag_name("collection")
                                });
                            }
                        }
                    }
                }
            }
        }

        // Convert href to relative path
        let relative = href_to_relative(&href, remote_root);
        if relative.is_empty() {
            continue; // Skip the root itself
        }

        // Parse last-modified to epoch ms
        let mtime_ms = parse_http_date_to_epoch_ms(&last_modified);

        files.insert(relative, (etag, content_length, mtime_ms, is_dir));
        if is_dir {
            dir_count += 1;
        } else {
            file_count += 1;
        }
    }

    Ok(RemoteScanResult {
        files,
        file_count,
        dir_count,
    })
}

/// Convert a WebDAV href to a relative path from the remote root.
fn href_to_relative(href: &str, remote_root: &str) -> String {
    // Decode percent-encoding
    let decoded = percent_decode(href);

    // Strip the remote root prefix
    let root = remote_root.trim_start_matches('/');
    let path = decoded.trim_start_matches('/');

    if root.is_empty() {
        if path.is_empty() {
            return String::new();
        }
        return path.to_string();
    }

    path.strip_prefix(root)
        .unwrap_or(path)
        .trim_start_matches('/')
        .to_string()
}

/// Simple percent-decoding for URLs.
fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next().unwrap_or(b'0');
            let lo = chars.next().unwrap_or(b'0');
            let val = hex_val(hi) << 4 | hex_val(lo);
            result.push(val as char);
        } else {
            result.push(b as char);
        }
    }
    result
}

fn hex_val(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

/// Parse an HTTP-date (RFC 7231) to UNIX epoch milliseconds.
/// Handles formats like "Thu, 29 May 2026 12:00:00 GMT".
fn parse_http_date_to_epoch_ms(date_str: &str) -> i64 {
    // Try chrono's RFC 2822 parsing
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(date_str) {
        return dt.timestamp_millis();
    }
    // Try RFC 3339
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return dt.timestamp_millis();
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_propfind() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/docs/</d:href>
    <d:propstat>
      <d:prop>
        <d:resourcetype><d:collection/></d:resourcetype>
        <d:getetag>"col-1234567890"</d:getetag>
      </d:prop>
    </d:propstat>
  </d:response>
  <d:response>
    <d:href>/docs/hello.txt</d:href>
    <d:propstat>
      <d:prop>
        <d:getcontentlength>11</d:getcontentlength>
        <d:getlastmodified>Thu, 29 May 2026 12:00:00 GMT</d:getlastmodified>
        <d:getetag>"b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"</d:getetag>
        <d:resourcetype/>
      </d:prop>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        let result = parse_propfind_response(xml, "/docs").unwrap();
        assert_eq!(result.file_count, 1);
        assert!(result.files.contains_key("hello.txt"));

        let (hash, size, _, is_dir) = result.files.get("hello.txt").unwrap();
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        assert_eq!(*size, 11);
        assert!(!is_dir);
    }

    #[test]
    fn test_href_to_relative() {
        assert_eq!(href_to_relative("/docs/file.txt", "/docs"), "file.txt");
        assert_eq!(
            href_to_relative("/docs/sub/file.txt", "/docs"),
            "sub/file.txt"
        );
        assert_eq!(href_to_relative("/docs/", "/docs"), "");
        assert_eq!(href_to_relative("/file.txt", ""), "file.txt");
    }

    #[test]
    fn test_percent_decode() {
        assert_eq!(percent_decode("hello%20world"), "hello world");
        assert_eq!(percent_decode("a%26b"), "a&b");
        assert_eq!(percent_decode("normal"), "normal");
    }
}
