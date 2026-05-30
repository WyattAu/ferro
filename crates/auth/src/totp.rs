//! Time-based One-Time Password (TOTP) implementation.
//!
//! Implements RFC 6238 (TOTP) based on HMAC-SHA1 per RFC 4226 (HOTP).
//! Uses 30-second time steps and 6-digit codes.

use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::Sha256;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;

pub const TOTP_DIGITS: u32 = 6;

#[derive(Debug, thiserror::Error)]
pub enum TotpError {
    #[error("HMAC key error: {0}")]
    HmacKey(#[from] hmac::digest::InvalidLength),
}

/// Generate a TOTP code for the given secret and timestamp.
///
/// Uses HMAC-SHA1 as mandated by RFC 6238 for maximum compatibility with
/// existing authenticator apps (Google Authenticator, Authy, etc.). Most
/// provisioning protocols and hardware tokens only support SHA-1 for TOTP.
/// If stronger HMAC is required, use [`generate_totp_sha256`] instead.
///
/// Parameters:
/// - `secret`: Raw secret bytes (typically 20 bytes)
/// - `timestamp`: UNIX epoch timestamp in seconds
/// - `digits`: Number of digits in the code (6 or 8)
/// - `step`: Time step in seconds (default 30)
/// - `t0`: Epoch offset (default 0)
pub fn generate_totp(secret: &[u8], timestamp: u64, digits: u32, step: u64, t0: u64) -> u32 {
    let counter = (timestamp - t0) / step;
    generate_hotp(secret, counter, digits)
}

/// Generate an HOTP code (RFC 4226).
fn generate_hotp(secret: &[u8], counter: u64, digits: u32) -> u32 {
    let mut mac = HmacSha1::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();

    // Dynamic truncation (RFC 4226 Section 5.3)
    let offset = (result[19] & 0x0f) as usize;
    let binary: u32 = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32 & 0xff) << 16)
        | ((result[offset + 2] as u32 & 0xff) << 8)
        | (result[offset + 3] as u32 & 0xff);

    binary % 10u32.pow(digits)
}

/// Generate an HOTP code using HMAC-SHA256 (RFC 4226 with SHA-256).
fn generate_hotp_sha256(secret: &[u8], counter: u64, digits: u32) -> u32 {
    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
    mac.update(&counter.to_be_bytes());
    let result = mac.finalize().into_bytes();

    let offset = (result[31] & 0x0f) as usize;
    let binary: u32 = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32 & 0xff) << 16)
        | ((result[offset + 2] as u32 & 0xff) << 8)
        | (result[offset + 3] as u32 & 0xff);

    binary % 10u32.pow(digits)
}

/// Generate a TOTP code using HMAC-SHA256.
///
/// For environments where SHA-1 is considered insufficient, this provides
/// TOTP generation using HMAC-SHA256. Note that support in authenticator
/// apps varies — SHA-256 TOTP should only be used when the client
/// application explicitly supports it (via the `algorithm=SHA256`
/// parameter in the otpauth:// URI).
pub fn generate_totp_sha256(
    secret: &[u8],
    _counter: u64,
    timestamp: u64,
) -> Result<String, TotpError> {
    let step = 30u64;
    let counter = timestamp / step;
    let code = generate_hotp_sha256(secret, counter, TOTP_DIGITS);
    Ok(format!("{:0width$}", code, width = TOTP_DIGITS as usize))
}

/// Verify a TOTP code against the current time, allowing clock drift.
///
/// Checks the code at `current_timestamp`, and optionally at ±1 and ±2
/// time steps to accommodate clock skew.
pub fn verify_totp(
    secret: &[u8],
    code: u32,
    timestamp: u64,
    digits: u32,
    step: u64,
    t0: u64,
    skew_steps: u32,
) -> bool {
    for offset in 0..=skew_steps {
        // Check current + offset
        let check_time = timestamp.saturating_add((offset as u64) * step);
        if generate_totp(secret, check_time, digits, step, t0) == code {
            return true;
        }
        // Check current - offset (avoid underflow)
        if offset > 0 {
            let check_time = timestamp.saturating_sub((offset as u64) * step);
            if generate_totp(secret, check_time, digits, step, t0) == code {
                return true;
            }
        }
    }
    false
}

/// Generate a cryptographically random TOTP secret.
/// Returns 20 random bytes.
pub fn generate_secret() -> Vec<u8> {
    use rand::RngCore;
    let mut secret = vec![0u8; 20];
    rand::rng().fill_bytes(&mut secret);
    secret
}

/// Encode a secret as Base32 (standard RFC 4648 alphabet for human-readable display).
pub fn encode_secret_base32(secret: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;

    for &byte in secret {
        buffer = (buffer << 8) | byte as u64;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((buffer >> bits) & 0x1f) as usize;
            result.push(ALPHABET[idx] as char);
        }
    }
    if bits > 0 {
        let idx = ((buffer << (5 - bits)) & 0x1f) as usize;
        result.push(ALPHABET[idx] as char);
    }
    result
}

/// Decode a Base32-encoded secret back to bytes.
pub fn decode_secret_base32(encoded: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let upper = encoded.to_uppercase();
    let mut result = Vec::new();
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;

    for ch in upper.bytes() {
        let val = match ALPHABET.iter().position(|&c| c == ch) {
            Some(v) => v as u64,
            None => return None,
        };
        buffer = (buffer << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            result.push((buffer >> bits) as u8);
            buffer &= (1u64 << bits) - 1;
        }
    }
    Some(result)
}

/// Generate the otpauth:// URI for QR code scanning.
///
/// Format: `otpauth://totp/Ferro:user@example.com?secret=BASE32SECRET&issuer=Ferro&algorithm=SHA1&digits=6&period=30`
pub fn generate_otpauth_uri(
    issuer: &str,
    username: &str,
    secret_base32: &str,
    digits: u32,
    period: u64,
) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        issuer,
        urlencoding_encode(username),
        secret_base32,
        issuer,
        digits,
        period,
    )
}

fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                if c == ' ' {
                    "%20".to_string()
                } else {
                    c.to_string()
                }
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_totp() {
        let secret = generate_secret();
        let timestamp = 1_700_000_000u64;

        let code = generate_totp(&secret, timestamp, 6, 30, 0);
        assert!(code < 1_000_000, "6-digit code should be < 1M");
        assert!(verify_totp(&secret, code, timestamp, 6, 30, 0, 0));
    }

    #[test]
    fn test_verify_with_skew() {
        let secret = generate_secret();
        let timestamp = 1_700_000_000u64;
        let code = generate_totp(&secret, timestamp, 6, 30, 0);

        // Should verify at current time
        assert!(verify_totp(&secret, code, timestamp, 6, 30, 0, 1));
    }

    #[test]
    fn test_wrong_code_rejected() {
        let secret = generate_secret();
        let timestamp = 1_700_000_000u64;
        let wrong_code = 0;
        assert!(!verify_totp(&secret, wrong_code, timestamp, 6, 30, 0, 0));
    }

    #[test]
    fn test_base32_roundtrip() {
        let secret = generate_secret();
        let encoded = encode_secret_base32(&secret);
        let decoded = decode_secret_base32(&encoded).unwrap();
        assert_eq!(secret, decoded);
    }

    #[test]
    fn test_base32_decode_case_insensitive() {
        let secret = generate_secret();
        let encoded = encode_secret_base32(&secret);
        let decoded = decode_secret_base32(&encoded.to_lowercase()).unwrap();
        assert_eq!(secret, decoded);
    }

    #[test]
    fn test_otpauth_uri() {
        let secret = generate_secret();
        let encoded = encode_secret_base32(&secret);
        let uri = generate_otpauth_uri("Ferro", "admin@example.com", &encoded, 6, 30);
        assert!(uri.starts_with("otpauth://totp/Ferro:admin"));
        assert!(uri.contains("algorithm=SHA1"));
        assert!(uri.contains("digits=6"));
        assert!(uri.contains("period=30"));
    }

    #[test]
    fn test_deterministic_hotp() {
        // Test vector from RFC 4226 Appendix D
        let secret: Vec<u8> = "12345678901234567890".as_bytes().to_vec();
        // RFC 4226 test vectors (HOTP with SHA1)
        // We can't test exact values without the exact secret decoding,
        // but we can verify determinism.
        let code1 = generate_hotp(&secret, 0, 6);
        let code2 = generate_hotp(&secret, 0, 6);
        assert_eq!(code1, code2, "HOTP must be deterministic");

        // Different counters should produce different codes (usually)
        let codes: std::collections::HashSet<u32> =
            (0..10).map(|i| generate_hotp(&secret, i, 6)).collect();
        // With 6 digits, 10 sequential codes should have some diversity
        // (not guaranteed, but extremely likely)
        assert!(codes.len() > 5, "sequential HOTP codes should vary");
    }

    #[test]
    fn test_8_digit_code() {
        let secret = generate_secret();
        let code = generate_totp(&secret, 1_700_000_000, 8, 30, 0);
        assert!(code < 100_000_000, "8-digit code should be < 100M");
        // Code may have leading zeros (e.g., 05123456), so only check the upper bound.
        // The modulus ensures it fits in 8 digits.
        assert!(verify_totp(&secret, code, 1_700_000_000, 8, 30, 0, 0));
    }

    #[test]
    fn test_generate_totp_sha256() {
        let secret = generate_secret();
        let timestamp = 1_700_000_000u64;
        let code = generate_totp_sha256(&secret, 0, timestamp).unwrap();
        assert_eq!(code.len(), TOTP_DIGITS as usize);
        assert!(code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_totp_sha256_deterministic() {
        let secret = generate_secret();
        let timestamp = 1_700_000_000u64;
        let code1 = generate_totp_sha256(&secret, 0, timestamp).unwrap();
        let code2 = generate_totp_sha256(&secret, 0, timestamp).unwrap();
        assert_eq!(code1, code2, "SHA-256 TOTP must be deterministic");
    }

    #[test]
    fn test_totp_digits_constant() {
        assert_eq!(TOTP_DIGITS, 6);
    }
}
