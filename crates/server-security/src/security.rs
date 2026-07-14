use std::net::ToSocketAddrs;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use dashmap::DashMap;
use rand::Rng;

use base64::Engine;

use crate::ApiError;

pub struct AuthAttemptTracker {
    attempts: DashMap<String, AuthAttemptState>,
    max_failures: u32,
    lockout_duration: Duration,
}

struct AuthAttemptState {
    fail_count: u32,
    first_fail_time: Instant,
    locked_until: Option<Instant>,
}

impl AuthAttemptTracker {
    #[must_use]
    pub fn new(max_failures: u32, lockout_duration: Duration) -> Self {
        Self {
            attempts: DashMap::new(),
            max_failures,
            lockout_duration,
        }
    }
}

impl Default for AuthAttemptTracker {
    fn default() -> Self {
        Self::new(10, Duration::from_mins(15))
    }
}

impl AuthAttemptTracker {
    #[must_use]
    pub fn record_failure(&self, client_ip: &str, username: &str) -> bool {
        let key = format!("{client_ip}:{username}");
        let mut entry = self.attempts.entry(key).or_insert(AuthAttemptState {
            fail_count: 0,
            first_fail_time: Instant::now(),
            locked_until: None,
        });

        if let Some(locked_until) = entry.locked_until {
            if Instant::now() < locked_until {
                return true;
            }
            entry.fail_count = 0;
            entry.locked_until = None;
        }

        entry.fail_count += 1;

        if entry.fail_count >= self.max_failures {
            entry.locked_until = Some(Instant::now() + self.lockout_duration);
            return true;
        }

        false
    }

    #[must_use]
    pub fn is_locked_out(&self, client_ip: &str, username: &str) -> bool {
        let key = format!("{client_ip}:{username}");
        if let Some(entry) = self.attempts.get(&key)
            && let Some(locked_until) = entry.locked_until
            && Instant::now() < locked_until
        {
            return true;
        }
        false
    }

    pub fn record_success(&self, client_ip: &str, username: &str) {
        let key = format!("{client_ip}:{username}");
        self.attempts.remove(&key);
    }

    pub fn cleanup(&self, max_age: Duration) {
        let cutoff = Instant::now().checked_sub(max_age).unwrap();
        self.attempts
            .retain(|_, state| state.first_fail_time > cutoff && state.locked_until.is_none_or(|t| t > cutoff));
    }
}

pub struct LoginRateLimiter {
    buckets: DashMap<String, LoginBucket>,
    max_attempts: u32,
    window: Duration,
}

struct LoginBucket {
    tokens: u32,
    last_refill: Instant,
}

impl LoginRateLimiter {
    #[must_use]
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        Self {
            buckets: DashMap::new(),
            max_attempts,
            window,
        }
    }
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self::new(5, Duration::from_mins(1))
    }
}

impl LoginRateLimiter {
    pub async fn check(&self, client_ip: &str) -> bool {
        let mut bucket = self.buckets.entry(client_ip.to_string()).or_insert(LoginBucket {
            tokens: self.max_attempts,
            last_refill: Instant::now(),
        });

        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() / self.window.as_secs_f64() * f64::from(self.max_attempts)) as u32;

        if tokens_to_add > 0 {
            bucket.tokens = (bucket.tokens + tokens_to_add).min(self.max_attempts);
            bucket.last_refill = now;
        }

        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    pub fn cleanup(&self, max_age: Duration) {
        let cutoff = Instant::now().checked_sub(max_age).unwrap();
        self.buckets.retain(|_, b| b.last_refill > cutoff);
    }
}

const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM0", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT0",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

fn is_invalid_char(c: char) -> bool {
    (c.is_control() && c != '\t') || c == '\x00'
}

pub fn validate_filename(name: &str) -> Result<(), &'static str> {
    if name.is_empty() {
        return Err("Filename must not be empty");
    }

    if name.chars().any(is_invalid_char) {
        return Err("Filename contains control characters");
    }

    for reserved in RESERVED_NAMES {
        if name == *reserved {
            return Err("Filename is a reserved system name");
        }
    }

    let trimmed = name.trim_matches(|c: char| c == '.' || c == ' ');
    if trimmed.is_empty() {
        return Err("Filename must not be empty or whitespace-only");
    }

    if name.len() > 255 {
        return Err("Filename exceeds 255 characters");
    }

    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err("Filename contains path separators");
    }

    Ok(())
}

pub fn validate_path(path: &str) -> Result<(), &'static str> {
    for component in path.split('/') {
        if component.is_empty() {
            continue;
        }
        validate_filename(component)?;
    }
    Ok(())
}

struct MagicSignature {
    offset: usize,
    bytes: &'static [u8],
    content_type: &'static str,
}

static MAGIC_SIGNATURES: &[MagicSignature] = &[
    MagicSignature {
        offset: 0,
        bytes: b"PK\x03\x04",
        content_type: "application/zip",
    },
    MagicSignature {
        offset: 0,
        bytes: b"%PDF",
        content_type: "application/pdf",
    },
    MagicSignature {
        offset: 0,
        bytes: b"\x89PNG",
        content_type: "image/png",
    },
    MagicSignature {
        offset: 0,
        bytes: b"\xFF\xD8\xFF",
        content_type: "image/jpeg",
    },
    MagicSignature {
        offset: 0,
        bytes: b"GIF87a",
        content_type: "image/gif",
    },
    MagicSignature {
        offset: 0,
        bytes: b"GIF89a",
        content_type: "image/gif",
    },
    MagicSignature {
        offset: 0,
        bytes: b"\x1F\x8B",
        content_type: "application/gzip",
    },
    MagicSignature {
        offset: 0,
        bytes: b"Rar!\x1A\x07",
        content_type: "application/vnd.rar",
    },
    MagicSignature {
        offset: 0,
        bytes: b"7z\xBC\xAF\x27\x1C",
        content_type: "application/x-7z-compressed",
    },
    MagicSignature {
        offset: 0,
        bytes: b"\x7FELF",
        content_type: "application/x-elf",
    },
    MagicSignature {
        offset: 0,
        bytes: b"\xFE\xED\xFA\xCE",
        content_type: "application/x-mach-o",
    },
    MagicSignature {
        offset: 0,
        bytes: b"\xCE\xFA\xED\xFE",
        content_type: "application/x-mach-o",
    },
    MagicSignature {
        offset: 0,
        bytes: b"MZ",
        content_type: "application/x-msdownload",
    },
    MagicSignature {
        offset: 0,
        bytes: b"#!",
        content_type: "text/x-script",
    },
    MagicSignature {
        offset: 0,
        bytes: b"<?xml",
        content_type: "image/svg+xml",
    },
    MagicSignature {
        offset: 0,
        bytes: b"<html",
        content_type: "text/html",
    },
    MagicSignature {
        offset: 0,
        bytes: b"<HTML",
        content_type: "text/html",
    },
    MagicSignature {
        offset: 0,
        bytes: b"OggS",
        content_type: "audio/ogg",
    },
    MagicSignature {
        offset: 4,
        bytes: b"ftyp",
        content_type: "video/mp4",
    },
    MagicSignature {
        offset: 8,
        bytes: b"WEBP",
        content_type: "image/webp",
    },
    MagicSignature {
        offset: 4,
        bytes: b"ftypavif",
        content_type: "image/avif",
    },
    MagicSignature {
        offset: 0,
        bytes: b"PK\x03\x04",
        content_type: "application/vnd.oasis",
    },
    MagicSignature {
        offset: 0,
        bytes: b"\x16\x00\x00\x00",
        content_type: "application/bson",
    },
    MagicSignature {
        offset: 0,
        bytes: b"wOFF",
        content_type: "font/woff",
    },
    MagicSignature {
        offset: 0,
        bytes: b"wOF2",
        content_type: "font/woff2",
    },
    MagicSignature {
        offset: 0,
        bytes: b"\x00\x01\x00\x00",
        content_type: "font/ttf",
    },
];

#[must_use]
pub fn verify_content_type(declared: &str, data: &[u8]) -> Option<String> {
    if declared.is_empty() || declared.starts_with("multipart/") || declared == "application/octet-stream" {
        return None;
    }

    for sig in MAGIC_SIGNATURES {
        if data.len() >= sig.offset + sig.bytes.len() && &data[sig.offset..sig.offset + sig.bytes.len()] == sig.bytes {
            if types_compatible(declared, sig.content_type) {
                return None;
            }
            return Some(sig.content_type.to_string());
        }
    }

    None
}

fn types_compatible(declared: &str, detected: &str) -> bool {
    if declared == detected {
        return true;
    }

    if declared.starts_with("image/") && detected.starts_with("image/") {
        return true;
    }

    if declared.starts_with("audio/") && detected.starts_with("audio/") {
        return true;
    }

    if declared.starts_with("video/") && detected.starts_with("video/") {
        return true;
    }

    if detected == "application/zip"
        && (declared.starts_with("application/vnd.openxmlformats") || declared.starts_with("application/vnd.oasis"))
    {
        return true;
    }

    if declared.starts_with("text/") && detected == "text/x-script" {
        return true;
    }

    if declared.starts_with("font/") && detected.starts_with("font/") {
        return true;
    }

    false
}

#[must_use]
pub fn detect_content_type(data: &[u8]) -> Option<&'static str> {
    for sig in MAGIC_SIGNATURES {
        if data.len() >= sig.offset + sig.bytes.len() && &data[sig.offset..sig.offset + sig.bytes.len()] == sig.bytes {
            return Some(sig.content_type);
        }
    }
    None
}

#[must_use]
pub fn generate_csrf_token() -> String {
    let mut buf = [0u8; 32];
    rand::rng().fill(&mut buf);
    hex::encode(buf)
}

#[must_use]
pub fn verify_csrf_token(expected: &str, provided: &str) -> bool {
    use subtle::ConstantTimeEq;
    let expected_bytes = expected.as_bytes();
    let provided_bytes = provided.as_bytes();
    expected_bytes.ct_eq(provided_bytes).into()
}

#[must_use]
pub fn is_default_password(password: &str) -> bool {
    matches!(password, "changeme" | "admin" | "password" | "ferro" | "")
}

#[must_use]
pub fn is_password_change_allowed_path(path: &str) -> bool {
    path == "/api/auth/change-password"
        || path == "/.well-known/ferro"
        || path == "/healthz"
        || path == "/readyz"
        || path == "/metrics"
        || path == "/api/auth/info"
        || path == "/api/config"
        || path.starts_with("/ui/")
        || path == "/ui"
}

#[must_use]
pub fn response_require_password_change() -> axum::response::Response {
    let body = axum::Json(serde_json::json!({
        "error": "Default password in use. Password change required before accessing this resource.",
        "error_code": ApiError::PASSWORD_CHANGE_REQUIRED,
        "action": "POST /api/auth/change-password with {\"password\":\"<new-password>\"}"
    }));
    let mut response = (StatusCode::FORBIDDEN, body).into_response();
    response.headers_mut().insert(
        "X-Ferro-Action",
        axum::http::HeaderValue::from_static("change-password"),
    );
    response
}

fn extract_client_ip(req: &axum::http::Request<axum::body::Body>) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}

fn extract_username(req: &axum::http::Request<axum::body::Body>) -> Option<String> {
    let auth = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())?;

    if !auth.starts_with("Basic ") {
        return None;
    }
    let decoded = base64::engine::general_purpose::STANDARD.decode(&auth[6..]).ok()?;
    let creds = String::from_utf8_lossy(&decoded);
    creds.split_once(':').map(|(u, _)| u.to_string())
}

pub async fn auth_guard_middleware<S: crate::SecurityAppState>(
    axum::extract::State(state): axum::extract::State<S>,
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let path = req.uri().path();
    if path == "/healthz"
        || path == "/.well-known/ferro"
        || path == "/.well-known/openid-configuration"
        || path.starts_with("/api/auth/login")
        || path.starts_with("/api/auth/callback")
        || path.starts_with("/api/config")
        || path.starts_with("/api/auth/info")
        || path == "/metrics"
        || path.starts_with("/ui/")
        || path == "/ui"
    {
        return next.run(req).await;
    }

    let client_ip = extract_client_ip(&req);
    let username = extract_username(&req).unwrap_or_default();

    if state.auth_attempt_tracker().is_locked_out(&client_ip, &username) {
        tracing::warn!(
            %client_ip,
            %username,
            "Account temporarily locked due to too many failed attempts"
        );
        return (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "Account temporarily locked",
                "error_code": "ACCOUNT_LOCKED",
            })),
        )
            .into_response();
    }

    let response = next.run(req).await;
    let status = response.status();

    if status == axum::http::StatusCode::UNAUTHORIZED {
        if !state.login_rate_limiter().check(&client_ip).await {
            tracing::warn!(
                %client_ip,
                %username,
                "Login rate limit exceeded"
            );
            return (
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                axum::Json(serde_json::json!({
                    "error": "Too many authentication attempts",
                    "error_code": "AUTH_RATE_LIMITED",
                })),
            )
                .into_response();
        }
        let _ = state.auth_attempt_tracker().record_failure(&client_ip, &username);
    } else if status.is_success() {
        state.auth_attempt_tracker().record_success(&client_ip, &username);
    }

    response
}

pub const MAX_URL_LENGTH: usize = 2048;

fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 0
                || octets[0] == 10
                || octets[0] == 127
                || (octets[0] == 169 && octets[1] == 254)
                || (octets[0] == 172 && (octets[1] & 0xF0) == 16)
                || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
                || (octets[0] == 192 && octets[1] == 168)
                || (octets[0] == 198 && (octets[1] & 0xFE) == 18)
                || octets[0] >= 224
        }
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback()
                || (v6.segments()[0] & 0xFFC0) == 0xFE80
                || (v6.segments()[0] & 0xFE00) == 0xFC00
                || matches!(v6.to_ipv4_mapped(), Some(v4) if is_private_ip(v4.into()))
        }
    }
}

const ALLOWED_URL_SCHEMES: &[&str] = &["http", "https"];

pub fn validate_url(url: &str) -> Result<(), String> {
    if url.len() > MAX_URL_LENGTH {
        return Err(format!("URL exceeds maximum length of {MAX_URL_LENGTH} characters"));
    }
    if url.is_empty() {
        return Err("URL must not be empty".to_string());
    }

    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;

    let scheme = parsed.scheme();
    if !ALLOWED_URL_SCHEMES.contains(&scheme) {
        return Err(format!(
            "URL scheme '{scheme}' is not allowed. Only http and https are permitted."
        ));
    }

    if !parsed.username().is_empty() {
        return Err("URL must not contain credentials (user:pass@host)".to_string());
    }

    let host = parsed.host_str().ok_or_else(|| "URL must have a host".to_string())?;

    let host_lower = host.to_lowercase();
    if host_lower == "localhost"
        || host_lower == "metadata.google.internal"
        || host_lower.ends_with(".local")
        || host_lower.ends_with(".internal")
    {
        return Err(format!("URL host '{host}' is not allowed"));
    }

    let port = parsed.port().unwrap_or(80);
    if let Ok(addrs) = format!("{host}:{port}").to_socket_addrs() {
        for addr in addrs {
            if is_private_ip(addr.ip()) {
                return Err(format!(
                    "URL host '{host}' resolves to a private/reserved IP address, which is not allowed"
                ));
            }
        }
    }

    Ok(())
}

#[must_use]
pub fn sanitize_control_chars(input: &str) -> String {
    input
        .chars()
        .map(|c| if c.is_control() || c == '\0' { ' ' } else { c })
        .collect()
}

#[must_use]
pub fn contains_html(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.contains("<script")
        || lower.contains("</script")
        || lower.contains("onerror=")
        || lower.contains("onload=")
        || lower.contains("onclick=")
        || lower.contains("onmouseover=")
        || lower.contains("javascript:")
        || lower.contains("<iframe")
        || lower.contains("<img")
        || lower.contains("<svg")
        || lower.contains("<object")
        || lower.contains("<embed")
        || lower.contains("<link")
        || lower.contains("<style")
        || lower.contains("alert(")
        || lower.contains("document.")
        || lower.contains("window.")
}

#[must_use]
pub fn is_smuggling_request(headers: &axum::http::HeaderMap) -> bool {
    let has_content_length = headers.contains_key("content-length");
    let has_transfer_encoding = headers.contains_key("transfer-encoding");
    has_content_length && has_transfer_encoding
}

pub async fn smuggling_rejection_middleware(
    req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if is_smuggling_request(req.headers()) {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": "INVALID_REQUEST",
                "message": "Request contains both Content-Length and Transfer-Encoding headers, which is not permitted"
            })),
        )
            .into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_filename_normal() {
        assert!(validate_filename("document.pdf").is_ok());
        assert!(validate_filename("photo.jpg").is_ok());
        assert!(validate_filename("my file.txt").is_ok());
        assert!(validate_filename("data-2024.csv").is_ok());
    }

    #[test]
    fn test_validate_filename_empty() {
        assert!(validate_filename("").is_err());
        assert!(validate_filename("   ").is_err());
        assert!(validate_filename("...").is_err());
    }

    #[test]
    fn test_validate_filename_control_chars() {
        assert!(validate_filename("file\x00name").is_err());
        assert!(validate_filename("file\x01name").is_err());
        assert!(validate_filename("file\x1Fname").is_err());
        assert!(validate_filename("file\tname").is_ok());
    }

    #[test]
    fn test_validate_filename_reserved_names() {
        assert!(validate_filename("CON").is_err());
        assert!(validate_filename("con").is_ok());
        assert!(validate_filename("NUL").is_err());
        assert!(validate_filename("AUX").is_err());
        assert!(validate_filename("LPT1").is_err());
        assert!(validate_filename("COM3").is_err());
    }

    #[test]
    fn test_validate_filename_too_long() {
        let long_name = "a".repeat(256);
        assert!(validate_filename(&long_name).is_err());
    }

    #[test]
    fn test_validate_filename_path_separators() {
        assert!(validate_filename("file/name").is_err());
        assert!(validate_filename("file\\name").is_err());
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("docs/report.pdf").is_ok());
        assert!(validate_path("a/b/c/file.txt").is_ok());
        assert!(validate_path("/a/b/c/").is_ok());
        assert!(validate_path("docs/CON/file.txt").is_err());
    }

    #[test]
    fn test_auth_tracker_lockout() {
        let tracker = AuthAttemptTracker::new(3, Duration::from_secs(10));
        assert!(!tracker.is_locked_out("1.2.3.4", "admin"));
        assert!(!tracker.record_failure("1.2.3.4", "admin"));
        assert!(!tracker.record_failure("1.2.3.4", "admin"));
        assert!(tracker.record_failure("1.2.3.4", "admin"));
        assert!(tracker.is_locked_out("1.2.3.4", "admin"));

        assert!(!tracker.is_locked_out("1.2.3.4", "other"));
    }

    #[test]
    fn test_auth_tracker_success_clears() {
        let tracker = AuthAttemptTracker::new(3, Duration::from_secs(10));
        let _ = tracker.record_failure("1.2.3.4", "admin");
        let _ = tracker.record_failure("1.2.3.4", "admin");
        tracker.record_success("1.2.3.4", "admin");
        assert!(!tracker.is_locked_out("1.2.3.4", "admin"));
        assert!(!tracker.record_failure("1.2.3.4", "admin"));
    }

    #[test]
    fn test_auth_tracker_different_ips() {
        let tracker = AuthAttemptTracker::new(2, Duration::from_secs(10));
        assert!(!tracker.record_failure("1.1.1.1", "admin"));
        assert!(!tracker.record_failure("2.2.2.2", "admin"));
        assert!(!tracker.is_locked_out("1.1.1.1", "admin"));
        assert!(!tracker.is_locked_out("2.2.2.2", "admin"));
    }

    #[test]
    fn test_login_rate_limiter() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let limiter = LoginRateLimiter::new(3, Duration::from_secs(60));
        assert!(rt.block_on(limiter.check("1.2.3.4")));
        assert!(rt.block_on(limiter.check("1.2.3.4")));
        assert!(rt.block_on(limiter.check("1.2.3.4")));
        assert!(!rt.block_on(limiter.check("1.2.3.4")));
        assert!(rt.block_on(limiter.check("5.6.7.8")));
    }

    #[test]
    fn test_verify_content_type_exact_match() {
        let png_data = b"\x89PNG\r\n\x1a\n";
        assert!(verify_content_type("image/png", png_data).is_none());
    }

    #[test]
    fn test_verify_content_type_mismatch() {
        let png_data = b"\x89PNG\r\n\x1a\n";
        let result = verify_content_type("application/pdf", png_data);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "image/png");
    }

    #[test]
    fn test_verify_content_type_octet_stream_skipped() {
        let data = b"\x89PNG\r\n\x1a\n";
        assert!(verify_content_type("application/octet-stream", data).is_none());
    }

    #[test]
    fn test_detect_content_type_png() {
        assert_eq!(detect_content_type(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
    }

    #[test]
    fn test_detect_content_type_pdf() {
        assert_eq!(detect_content_type(b"%PDF-1.4"), Some("application/pdf"));
    }

    #[test]
    fn test_detect_content_type_jpeg() {
        assert_eq!(detect_content_type(b"\xFF\xD8\xFF\xE0"), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_content_type_unknown() {
        assert_eq!(detect_content_type(b"random data here"), None);
    }

    #[test]
    fn test_detect_content_type_zip() {
        assert_eq!(detect_content_type(b"PK\x03\x04"), Some("application/zip"));
    }

    #[test]
    fn test_detect_content_type_mp4() {
        let data = [0u8; 8];
        let mut data = data;
        data[4..8].copy_from_slice(b"ftyp");
        assert_eq!(detect_content_type(&data), Some("video/mp4"));
    }

    #[test]
    fn test_generate_csrf_token_length() {
        let token = generate_csrf_token();
        assert_eq!(token.len(), 64);
    }

    #[test]
    fn test_verify_csrf_token() {
        let token = generate_csrf_token();
        assert!(verify_csrf_token(&token, &token));
        assert!(!verify_csrf_token(&token, "wrong"));
    }

    #[test]
    fn test_is_default_password() {
        assert!(is_default_password("changeme"));
        assert!(is_default_password("admin"));
        assert!(is_default_password("password"));
        assert!(is_default_password("ferro"));
        assert!(is_default_password(""));
        assert!(!is_default_password("SecurePass123!"));
    }
}

#[cfg(test)]
mod url_validation_tests {
    use super::*;

    #[test]
    fn test_validate_url_valid_https() {
        assert!(validate_url("https://example.com/webhook").is_ok());
    }

    #[test]
    fn test_validate_url_valid_http() {
        assert!(validate_url("http://example.com/hook").is_ok());
    }

    #[test]
    fn test_validate_url_file_scheme() {
        assert!(validate_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn test_validate_url_gopher_scheme() {
        assert!(validate_url("gopher://127.0.0.1:25").is_err());
    }

    #[test]
    fn test_validate_url_aws_metadata() {
        assert!(validate_url("http://169.254.169.254/latest/meta-data/").is_err());
    }

    #[test]
    fn test_validate_url_localhost() {
        assert!(validate_url("http://localhost:9090/api/v1/admin/stats").is_err());
    }

    #[test]
    fn test_validate_url_127_0_0_1() {
        assert!(validate_url("http://127.0.0.1:8080").is_err());
    }

    #[test]
    fn test_validate_url_gcp_metadata() {
        assert!(validate_url("http://metadata.google.internal/computeMetadata/v1/").is_err());
    }

    #[test]
    fn test_validate_url_with_credentials() {
        assert!(validate_url("http://user:pass@example.com/hook").is_err());
    }

    #[test]
    fn test_validate_url_too_long() {
        let long_url = format!("https://example.com/{}", "a".repeat(2049));
        assert!(validate_url(&long_url).is_err());
    }

    #[test]
    fn test_validate_url_empty() {
        assert!(validate_url("").is_err());
    }

    #[test]
    fn test_contains_html_script() {
        assert!(contains_html("<script>alert(1)</script>"));
    }

    #[test]
    fn test_contains_html_img_onerror() {
        assert!(contains_html("<img src=x onerror=alert(1)>"));
    }

    #[test]
    fn test_contains_html_safe() {
        assert!(!contains_html("Hello, this is a normal comment."));
        assert!(!contains_html("Test < 5 && result > 10"));
    }

    #[test]
    fn test_sanitize_control_chars() {
        let sanitized = sanitize_control_chars("hello\rworld\x00end");
        assert_eq!(sanitized, "hello world end");
    }

    #[test]
    fn test_is_smuggling_request() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("content-length", "100".parse().unwrap());
        headers.insert("transfer-encoding", "chunked".parse().unwrap());
        assert!(is_smuggling_request(&headers));
    }

    #[test]
    fn test_is_not_smuggling_cl_only() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("content-length", "100".parse().unwrap());
        assert!(!is_smuggling_request(&headers));
    }

    #[test]
    fn test_is_not_smuggling_te_only() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("transfer-encoding", "chunked".parse().unwrap());
        assert!(!is_smuggling_request(&headers));
    }
}
