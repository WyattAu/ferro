//! Shared HTTP client construction used by desktop and mobile crates.

use std::time::Duration;

/// Options for constructing an HTTP client.
pub struct HttpClientOptions {
    /// Request timeout (default: 30s).
    pub timeout: Option<Duration>,
    /// TCP connect timeout (default: 10s).
    pub connect_timeout: Option<Duration>,
    /// When true, bypass system proxy settings (default: false).
    pub no_proxy: bool,
}

impl Default for HttpClientOptions {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            connect_timeout: Some(Duration::from_secs(10)),
            no_proxy: false,
        }
    }
}

/// Build a `reqwest::Client` with the given auth token.
///
/// Token detection:
/// - Empty string: no auth header.
/// - Contains `:`: raw `user:pass`, base64-encoded as Basic auth.
/// - Base64-decodes to `user:pass`: used as-is as Basic auth.
/// - Otherwise: treated as a Bearer token.
pub fn build_client(token: &str, opts: HttpClientOptions) -> Result<reqwest::Client, String> {
    let mut headers = reqwest::header::HeaderMap::new();

    let auth_header = if token.is_empty() {
        String::new()
    } else if token.contains(':') {
        format!("Basic {}", base64_encode(token.as_bytes()))
    } else if let Some(decoded) = try_base64_decode(token) {
        if decoded.contains(':') {
            format!("Basic {token}")
        } else {
            format!("Bearer {token}")
        }
    } else {
        format!("Bearer {token}")
    };

    if !auth_header.is_empty() {
        let value =
            reqwest::header::HeaderValue::from_str(&auth_header).map_err(|e| format!("Invalid token: {e}"))?;
        headers.insert(reqwest::header::AUTHORIZATION, value);
    }

    let mut builder = reqwest::Client::builder().default_headers(headers);

    if opts.no_proxy {
        builder = builder.no_proxy();
    }
    if let Some(timeout) = opts.timeout {
        builder = builder.timeout(timeout);
    }
    if let Some(ct) = opts.connect_timeout {
        builder = builder.connect_timeout(ct);
    }

    builder.build().map_err(|e| format!("Failed to create HTTP client: {e}"))
}

fn base64_encode(input: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = Vec::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        output.push(CHARS[((triple >> 18) & 0x3F) as usize]);
        output.push(CHARS[((triple >> 12) & 0x3F) as usize]);
        if chunk.len() > 1 {
            output.push(CHARS[((triple >> 6) & 0x3F) as usize]);
        } else {
            output.push(b'=');
        }
        if chunk.len() > 2 {
            output.push(CHARS[(triple & 0x3F) as usize]);
        } else {
            output.push(b'=');
        }
    }
    // SAFETY: CHARS is ASCII-only, so output is valid UTF-8.
    String::from_utf8(output).unwrap_or_default()
}

fn try_base64_decode(input: &str) -> Option<String> {
    const TABLE: [i8; 256] = {
        let mut table = [0i8; 256];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[chars[i] as usize] = i as i8;
            i += 1;
        }
        table[b'=' as usize] = -1;
        table
    };

    let input = input.as_bytes();
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in input {
        let val = TABLE[byte as usize];
        if val == -1 {
            if bits >= 6 {
                output.push((buf >> (bits - 6)) as u8);
            }
            break;
        } else if val >= 0 {
            buf = (buf << 6) | (val as u32);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push((buf >> bits) as u8);
            }
        }
    }

    String::from_utf8(output).ok()
}
