use axum::http::HeaderValue;
use axum::http::header::HeaderMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct RangeRequest {
    pub ranges: Vec<RangeSpec>,
}

#[derive(Debug, Clone)]
pub struct RangeSpec {
    pub start: Option<u64>,
    pub end: Option<u64>,
}

impl RangeSpec {
    pub fn resolve(&self, content_length: u64) -> Option<(u64, u64)> {
        let (start, end) = match self.start {
            Some(s) => (s, self.end.unwrap_or(content_length.saturating_sub(1))),
            None => {
                let suffix = self.end?;
                if suffix == 0 {
                    return None;
                }
                let s = content_length.saturating_sub(suffix);
                (s, content_length.saturating_sub(1))
            }
        };
        if start > end || start >= content_length {
            return None;
        }
        Some((start, end.min(content_length.saturating_sub(1))))
    }
}

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
        let (start_str, end_str): (Option<&str>, Option<&str>) =
            if let Some(rest) = spec.strip_prefix('-') {
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

pub fn build_range_headers(start: u64, end: u64, content_length: u64) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Range",
        HeaderValue::from_str(&format!("bytes {}-{}/{}", start, end, content_length))
            .unwrap_or_else(|_| HeaderValue::from_static("bytes */*")),
    );
    headers.insert("Accept-Ranges", HeaderValue::from_static("bytes"));
    headers.insert(
        "Content-Length",
        HeaderValue::from_str(&(end - start + 1).to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers
}

pub fn accept_ranges_header() -> HeaderValue {
    HeaderValue::from_static("bytes")
}

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
    fn test_range_spec_resolve_suffix_zero() {
        let spec = RangeSpec {
            start: None,
            end: Some(0),
        };
        assert_eq!(spec.resolve(10000), None);
    }

    #[test]
    fn test_range_spec_resolve_start_past_end() {
        let spec = RangeSpec {
            start: Some(100),
            end: Some(50),
        };
        assert_eq!(spec.resolve(1000), None);
    }

    #[test]
    fn test_range_spec_resolve_start_past_content() {
        let spec = RangeSpec {
            start: Some(1000),
            end: Some(1500),
        };
        assert_eq!(spec.resolve(1000), None);
    }

    #[test]
    fn test_range_spec_resolve_end_clamped() {
        let spec = RangeSpec {
            start: Some(0),
            end: Some(2000),
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
        assert_eq!(parsed.ranges.len(), 1);
        assert_eq!(parsed.ranges[0].resolve(1000), Some((500, 999)));
    }

    #[test]
    fn test_parse_range_header_start_only() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("bytes=500-"));
        let parsed = parse_range_header(&headers, 1000).unwrap();
        assert_eq!(parsed.ranges.len(), 1);
        assert_eq!(parsed.ranges[0].resolve(1000), Some((500, 999)));
    }

    #[test]
    fn test_parse_range_header_multiple() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("bytes=0-499, 500-999"));
        let parsed = parse_range_header(&headers, 1000).unwrap();
        assert_eq!(parsed.ranges.len(), 2);
    }

    #[test]
    fn test_parse_range_header_no_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("0-499"));
        assert!(parse_range_header(&headers, 1000).is_none());
    }

    #[test]
    fn test_parse_range_header_empty() {
        let mut headers = HeaderMap::new();
        headers.insert("Range", HeaderValue::from_static("bytes="));
        assert!(parse_range_header(&headers, 1000).is_none());
    }

    #[test]
    fn test_parse_range_header_no_range() {
        let headers = HeaderMap::new();
        assert!(parse_range_header(&headers, 1000).is_none());
    }

    #[test]
    fn test_build_range_headers() {
        let headers = build_range_headers(0, 499, 1000);
        assert_eq!(headers.get("Content-Range").unwrap(), "bytes 0-499/1000");
        assert_eq!(headers.get("Content-Length").unwrap(), "500");
        assert_eq!(headers.get("Accept-Ranges").unwrap(), "bytes");
    }

    #[test]
    fn test_accept_ranges_header() {
        assert_eq!(accept_ranges_header(), "bytes");
    }

    #[test]
    fn test_range_spec_clone() {
        let spec = RangeSpec {
            start: Some(100),
            end: Some(200),
        };
        let cloned = spec.clone();
        assert_eq!(cloned.start, Some(100));
        assert_eq!(cloned.end, Some(200));
    }

    #[test]
    fn test_range_request_clone() {
        let req = RangeRequest {
            ranges: vec![
                RangeSpec { start: Some(0), end: Some(100) },
                RangeSpec { start: Some(200), end: Some(300) },
            ],
        };
        let cloned = req.clone();
        assert_eq!(cloned.ranges.len(), 2);
    }
}
