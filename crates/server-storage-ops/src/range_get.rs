//! Range GET support for partial content downloads (Phase 6.5).

use axum::http::HeaderValue;
use axum::http::header::HeaderMap;
use std::str::FromStr;

/// Parsed Range header value.
#[derive(Debug, Clone)]
pub struct RangeRequest {
    /// The byte ranges requested.
    pub ranges: Vec<RangeSpec>,
}

/// A single byte range specification.
#[derive(Debug, Clone)]
pub struct RangeSpec {
    /// Start byte (None means "from the end").
    pub start: Option<u64>,
    /// End byte (None means "to the end").
    pub end: Option<u64>,
}

impl RangeSpec {
    /// Resolve this range spec against a given content length.
    /// Returns (start, end) inclusive, or None if invalid.
    #[must_use]
    pub fn resolve(&self, content_length: u64) -> Option<(u64, u64)> {
        let (start, end) = if let Some(s) = self.start {
            (s, self.end.unwrap_or(content_length.saturating_sub(1)))
        } else {
            // suffix range: -N means last N bytes
            let suffix = self.end?;
            if suffix == 0 {
                return None;
            }
            let s = content_length.saturating_sub(suffix);
            (s, content_length.saturating_sub(1))
        };
        if start > end || start >= content_length {
            return None;
        }
        Some((start, end.min(content_length.saturating_sub(1))))
    }
}

/// Parse a Range header value.
/// Supports: `bytes=0-499`, `bytes=500-999`, `bytes=-500`, `bytes=9500-`
#[must_use]
pub fn parse_range_header(headers: &HeaderMap, _content_length: u64) -> Option<RangeRequest> {
    let range_header = headers.get("Range")?.to_str().ok()?;
    if !range_header.starts_with("bytes=") {
        return None;
    }
    let specs = &range_header[6..];
    if specs.is_empty() {
        return None;
    }

    let mut ranges = Vec::new();
    for spec in specs.split(',') {
        let spec = spec.trim();
        if spec.is_empty() {
            continue;
        }
        let (start_str, end_str): (Option<&str>, Option<&str>) = if let Some(rest) = spec.strip_prefix('-') {
            (None, Some(rest.trim()))
        } else if let Some(rest) = spec.strip_suffix('-') {
            (Some(rest.trim()), None)
        } else {
            let parts: Vec<&str> = spec.splitn(2, '-').collect();
            if parts.len() != 2 {
                continue;
            }
            (Some(parts[0].trim()), Some(parts[1].trim()))
        };

        let start = start_str.and_then(|s| u64::from_str(s).ok());
        let end = end_str.and_then(|s| u64::from_str(s).ok());

        ranges.push(RangeSpec { start, end });
    }

    if ranges.is_empty() {
        return None;
    }

    Some(RangeRequest { ranges })
}

/// Build response headers for a partial content (206) response.
#[must_use]
pub fn build_range_headers(start: u64, end: u64, content_length: u64) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Range",
        HeaderValue::from_str(&format!("bytes {start}-{end}/{content_length}"))
            .unwrap_or_else(|_| HeaderValue::from_static("bytes */*")),
    );
    headers.insert("Accept-Ranges", HeaderValue::from_static("bytes"));
    headers.insert(
        "Content-Length",
        HeaderValue::from_str(&(end - start + 1).to_string()).unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers
}

/// Build Accept-Ranges header for full responses.
#[must_use]
pub fn accept_ranges_header() -> HeaderValue {
    HeaderValue::from_static("bytes")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_spec_resolve_full() {
        let spec = RangeSpec {
            start: Some(0),
            end: Some(499),
        };
        assert_eq!(spec.resolve(1000), Some((0, 499)));
    }

    #[test]
    fn test_range_spec_resolve_partial() {
        let spec = RangeSpec {
            start: Some(500),
            end: Some(999),
        };
        assert_eq!(spec.resolve(1000), Some((500, 999)));
    }

    #[test]
    fn test_range_spec_resolve_from_offset() {
        let spec = RangeSpec {
            start: Some(9500),
            end: None,
        };
        assert_eq!(spec.resolve(10000), Some((9500, 9999)));
    }

    #[test]
    fn test_range_spec_resolve_suffix() {
        let spec = RangeSpec {
            start: None,
            end: Some(500),
        };
        assert_eq!(spec.resolve(10000), Some((9500, 9999)));
    }

    #[test]
    fn test_range_spec_resolve_last_byte() {
        let spec = RangeSpec {
            start: None,
            end: Some(1),
        };
        assert_eq!(spec.resolve(1000), Some((999, 999)));
    }

    #[test]
    fn test_range_spec_resolve_invalid_start_beyond_length() {
        let spec = RangeSpec {
            start: Some(2000),
            end: Some(2999),
        };
        assert_eq!(spec.resolve(1000), None);
    }

    #[test]
    fn test_range_spec_resolve_invalid_suffix_zero() {
        let spec = RangeSpec {
            start: None,
            end: Some(0),
        };
        assert_eq!(spec.resolve(1000), None);
    }

    #[test]
    fn test_range_spec_resolve_clamp_end() {
        let spec = RangeSpec {
            start: Some(0),
            end: Some(9999),
        };
        assert_eq!(spec.resolve(1000), Some((0, 999)));
    }

    #[test]
    fn test_parse_range_header_simple() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("bytes=0-499"));
        let parsed = parse_range_header(&headers, 1000).unwrap();
        assert_eq!(parsed.ranges.len(), 1);
        assert_eq!(parsed.ranges[0].resolve(1000), Some((0, 499)));
    }

    #[test]
    fn test_parse_range_header_suffix() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("bytes=-500"));
        let parsed = parse_range_header(&headers, 1000).unwrap();
        assert_eq!(parsed.ranges[0].resolve(1000), Some((500, 999)));
    }

    #[test]
    fn test_parse_range_header_from_offset() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("bytes=9500-"));
        let parsed = parse_range_header(&headers, 10000).unwrap();
        assert_eq!(parsed.ranges[0].resolve(10000), Some((9500, 9999)));
    }

    #[test]
    fn test_parse_range_header_multi() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("bytes=0-100, 200-300"));
        let parsed = parse_range_header(&headers, 1000).unwrap();
        assert_eq!(parsed.ranges.len(), 2);
    }

    #[test]
    fn test_parse_range_header_missing() {
        let headers = HeaderMap::new();
        assert!(parse_range_header(&headers, 1000).is_none());
    }

    #[test]
    fn test_parse_range_header_invalid_unit() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("bits=0-499"));
        assert!(parse_range_header(&headers, 1000).is_none());
    }

    #[test]
    fn test_build_range_headers() {
        let headers = build_range_headers(0, 499, 1000);
        assert_eq!(headers.get("Content-Range").unwrap(), "bytes 0-499/1000");
        assert_eq!(headers.get("Content-Length").unwrap(), "500");
        assert_eq!(headers.get("Accept-Ranges").unwrap(), "bytes");
    }
}
