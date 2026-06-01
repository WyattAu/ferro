use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn sign_payload(secret: &str, payload: &[u8], timestamp: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC key error");
    let message = format!("{timestamp}.");
    mac.update(message.as_bytes());
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify_signature(secret: &str, payload: &[u8], timestamp: &str, signature: &str) -> bool {
    let expected = sign_payload(secret, payload, timestamp);
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let result = a
        .iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y));
    result == 0
}
