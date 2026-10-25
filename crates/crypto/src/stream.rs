//! STREAM chunked AEAD (YP-CRYPTO-PQ-001 Rev 1.1, `ALG-STREAMAEAD-001`).
//!
//! Companion to `formal/Ferro/ChunkedAEAD.lean`. The container instantiates
//! the STREAM construction of Hoang, Reyhanitabar, Rogaway, Vizár
//! (ePrint 2015/189) exactly as age v1 (C2SP) and Tink Streaming AEAD do:
//! fixed 64 KiB chunks; a per-message random nonce prefix; per-chunk nonce
//! = prefix ‖ counter(BE, 4 bytes) ‖ last-block flag (1 byte); the chunk
//! index is bound into the AEAD's associated data. Decryption is fail-closed:
//! EOF without a final-flagged chunk is rejected (truncation detection), a
//! final flag before EOF is rejected, and the AAD must match.
//!
//! External assumptions: AES-256-GCM IND-CPA/authenticity and SHA-256
//! collision resistance (as everywhere in this tree).

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use sha2::{Digest, Sha256};

/// Payload chunk size (age v1 / Tink: 64 KiB).
pub const CHUNK_SIZE: usize = 64 * 1024;

/// Container magic.
pub const MAGIC: &[u8; 12] = b"FERROSTREAM1";

/// Random nonce prefix length (Tink STREAM layout: 7 + 4 + 1 = 12).
pub const NONCE_PREFIX_LEN: usize = 7;

/// AEAD nonce length (AES-GCM: 12 bytes).
pub const NONCE_LEN: usize = 12;

/// Tag length appended to every chunk (AES-GCM-256).
pub const TAG_LEN: usize = 16;

/// Container error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StreamError {
    #[error("container too short or missing magic")]
    Malformed,
    #[error("data ended without a final-flagged chunk (truncation)")]
    Truncated,
    #[error("final-flagged chunk appeared before end of data")]
    PrematureFinal,
    #[error("chunk counter exhausted")]
    CounterOverflow,
    #[error("authentication failed (tampered chunk or wrong key/aad)")]
    AuthFailed,
    #[error("no secure entropy source available for the nonce prefix")]
    NoEntropy,
}

fn chunk_aad(aad: &[u8], counter: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(aad.len() + 4);
    v.extend_from_slice(aad);
    v.extend_from_slice(&counter.to_be_bytes());
    v
}

fn build_nonce(prefix: &[u8; NONCE_PREFIX_LEN], counter: u32, last: bool) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    nonce[..NONCE_PREFIX_LEN].copy_from_slice(prefix);
    nonce[NONCE_PREFIX_LEN..NONCE_PREFIX_LEN + 4].copy_from_slice(&counter.to_be_bytes());
    nonce[NONCE_LEN - 1] = if last { 1 } else { 0 };
    nonce
}

/// Encrypt `plaintext` into the STREAM container.
///
/// AAD is position-bound: each chunk's associated data is `aad ‖ counter`,
/// so chunks cannot be reordered or replayed across positions
/// (THM-AEAD-POSITION-BOUND).
pub fn encrypt(key: &[u8; 32], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, StreamError> {
    let mut prefix = [0u8; NONCE_PREFIX_LEN];
    rand_fill(&mut prefix)?;
    encrypt_with_nonce_prefix(key, plaintext, aad, &prefix)
}

/// Encrypt with a caller-supplied nonce prefix (7 bytes) — required on
/// no-`ring` builds; also useful for callers sourcing entropy from
/// `CryptoProvider::random_bytes`. An all-zero prefix is rejected
/// fail-closed (a repeated prefix would collapse nonce uniqueness across
/// messages sharing a key).
pub fn encrypt_with_nonce_prefix(
    key: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
    prefix: &[u8; NONCE_PREFIX_LEN],
) -> Result<Vec<u8>, StreamError> {
    if prefix.iter().all(|b| *b == 0) {
        return Err(StreamError::NoEntropy);
    }
    encrypt_inner(key, plaintext, aad, prefix)
}

fn encrypt_inner(
    key: &[u8; 32],
    plaintext: &[u8],
    aad: &[u8],
    prefix: &[u8; NONCE_PREFIX_LEN],
) -> Result<Vec<u8>, StreamError> {
    let cipher = Aes256Gcm::new(key.into());

    let mut out = Vec::with_capacity(
        MAGIC.len() + NONCE_PREFIX_LEN + plaintext.len() + (plaintext.len() / CHUNK_SIZE + 1) * TAG_LEN,
    );
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(prefix);

    let total_chunks = if plaintext.is_empty() {
        1
    } else {
        plaintext.len().div_ceil(CHUNK_SIZE)
    };
    if total_chunks as u64 >= (1u64 << 32) - 1 {
        return Err(StreamError::CounterOverflow);
    }

    for (idx, chunk) in plaintext.chunks(CHUNK_SIZE).enumerate() {
        let counter = idx as u32;
        let last = idx + 1 == total_chunks;
        let nonce = build_nonce(&prefix, counter, last);
        let ct = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: chunk,
                    aad: &chunk_aad(aad, counter),
                },
            )
            .map_err(|_| StreamError::AuthFailed)?;
        out.extend_from_slice(&ct);
        if last {
            return Ok(out);
        }
    }

    // Empty plaintext: emit the single final empty chunk.
    let nonce = build_nonce(&prefix, 0, true);
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &[],
                aad: &chunk_aad(aad, 0),
            },
        )
        .map_err(|_| StreamError::AuthFailed)?;
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt a STREAM container back to the plaintext. Fail-closed on
/// truncation, premature final flags, tag failures, and AAD mismatch.
pub fn decrypt(key: &[u8; 32], container: &[u8], aad: &[u8]) -> Result<Vec<u8>, StreamError> {
    if container.len() < MAGIC.len() + NONCE_PREFIX_LEN + TAG_LEN {
        return Err(StreamError::Malformed);
    }
    if &container[..MAGIC.len()] != MAGIC {
        return Err(StreamError::Malformed);
    }
    let mut prefix = [0u8; NONCE_PREFIX_LEN];
    prefix.copy_from_slice(&container[MAGIC.len()..MAGIC.len() + NONCE_PREFIX_LEN]);

    let cipher = Aes256Gcm::new(key.into());
    let body = &container[MAGIC.len() + NONCE_PREFIX_LEN..];

    let mut out = Vec::with_capacity(body.len());
    let mut offset = 0usize;
    let mut counter = 0u32;
    while offset < body.len() {
        let end = (offset + CHUNK_SIZE + TAG_LEN).min(body.len());
        let chunk_ct = &body[offset..end];
        let is_last = end == body.len();

        if counter == u32::MAX {
            return Err(StreamError::CounterOverflow);
        }
        let nonce = build_nonce(&prefix, counter, is_last);
        let pt = cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: chunk_ct,
                    aad: &chunk_aad(aad, counter),
                },
            )
            .map_err(|_| StreamError::AuthFailed)?;
        out.extend_from_slice(&pt);

        if is_last {
            return Ok(out);
        }
        offset = end;
        counter += 1;
    }

    // Data ended without a final-flagged chunk: truncated (or the final
    // flag was stripped). Fail closed — never return a prefix.
    Err(StreamError::Truncated)
}

/// Key derivation helper for tests and callers holding a passphrase.
#[must_use]
pub fn derive_key(material: &[u8]) -> [u8; 32] {
    Sha256::digest(material).into()
}

/// Fill `buf` with secure randomness.
///
/// Under the default `ring` feature this is the OS CSPRNG
/// (`ring::rand::SystemRandom`). Without it the call fails closed — a
/// predictable nonce prefix defeats the STREAM construction, so there is
/// deliberately no weak fallback (R5 fix of the time/address mix).
#[cfg(feature = "ring")]
fn rand_fill(buf: &mut [u8]) -> Result<(), StreamError> {
    use ring::rand::{SecureRandom, SystemRandom};
    SystemRandom::new().fill(buf).map_err(|_| StreamError::NoEntropy)
}

/// No-`ring` builds must inject entropy explicitly (`encrypt_with_nonce_prefix`);
/// the implicit path fails closed.
#[cfg(not(feature = "ring"))]
fn rand_fill(_buf: &mut [u8]) -> Result<(), StreamError> {
    Err(StreamError::NoEntropy)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8; 32] = b"0123456789abcdef0123456789abcdef";
    const AAD: &[u8] = b"tenant:acme/file:report.txt";

    #[test]
    fn test_roundtrip_small() {
        let ct = encrypt(KEY, b"hello stream", AAD).unwrap();
        let pt = decrypt(KEY, &ct, AAD).unwrap();
        assert_eq!(pt, b"hello stream");
    }

    #[test]
    fn test_roundtrip_empty() {
        let ct = encrypt(KEY, b"", AAD).unwrap();
        let pt = decrypt(KEY, &ct, AAD).unwrap();
        assert!(pt.is_empty());
    }

    #[test]
    fn test_roundtrip_exact_chunk_multiple() {
        let pt = vec![7u8; CHUNK_SIZE * 3];
        let ct = encrypt(KEY, &pt, AAD).unwrap();
        assert_eq!(decrypt(KEY, &ct, AAD).unwrap(), pt);
    }

    #[test]
    fn test_roundtrip_large_multichunk() {
        let pt: Vec<u8> = (0..CHUNK_SIZE * 2 + 12345).map(|i| (i % 251) as u8).collect();
        let ct = encrypt(KEY, &pt, AAD).unwrap();
        assert_eq!(decrypt(KEY, &ct, AAD).unwrap(), pt);
    }

    #[test]
    fn test_truncation_detected() {
        // THM-AEAD-TRUNCATION-DETECTED: dropping the final chunk must fail
        // closed and never return a prefix. Because each chunk's AEAD nonce
        // encodes its final flag, the surviving chunk re-framed as final
        // fails authentication — the truncation is detected either as
        // AuthFailed (tag mismatch on the re-framed chunk) or Truncated
        // (data ending without a final chunk).
        let pt: Vec<u8> = (0..CHUNK_SIZE + 100).map(|i| (i % 251) as u8).collect();
        let ct = encrypt(KEY, &pt, AAD).unwrap();
        let truncated = &ct[..ct.len() - (100 + TAG_LEN)];
        match decrypt(KEY, truncated, AAD) {
            Err(StreamError::AuthFailed) | Err(StreamError::Truncated) => {}
            other => panic!("expected fail-closed, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn test_bitflip_rejected() {
        let ct = encrypt(KEY, b"attack at dawn", AAD).unwrap();
        let mut tampered = ct.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        assert_eq!(decrypt(KEY, &tampered, AAD), Err(StreamError::AuthFailed));
    }

    #[test]
    fn test_wrong_key_rejected() {
        let ct = encrypt(KEY, b"secret", AAD).unwrap();
        let other = derive_key(b"other");
        assert_eq!(decrypt(&other, &ct, AAD), Err(StreamError::AuthFailed));
    }

    #[test]
    fn test_aad_mismatch_rejected() {
        let ct = encrypt(KEY, b"secret", AAD).unwrap();
        assert_eq!(decrypt(KEY, &ct, b"tenant:other/file:x"), Err(StreamError::AuthFailed));
    }

    #[test]
    fn test_position_binding_reorder_rejected() {
        // THM-AEAD-POSITION-BOUND: chunks are bound to their counter; a
        // chunk moved to another position fails authentication.
        let pt: Vec<u8> = (0..CHUNK_SIZE * 2).map(|i| (i % 251) as u8).collect();
        let ct = encrypt(KEY, &pt, AAD).unwrap();
        let hdr = MAGIC.len() + NONCE_PREFIX_LEN;
        let c0len = CHUNK_SIZE + TAG_LEN;
        let mut swapped = ct.clone();
        swapped[hdr..hdr + c0len].copy_from_slice(&ct[hdr + c0len..hdr + 2 * c0len]);
        assert_eq!(decrypt(KEY, &swapped, AAD), Err(StreamError::AuthFailed));
    }

    #[test]
    fn test_malformed_containers() {
        assert_eq!(decrypt(KEY, &[], AAD), Err(StreamError::Malformed));
        assert_eq!(decrypt(KEY, b"garbage", AAD), Err(StreamError::Malformed));
        let mut bad_magic = encrypt(KEY, b"x", AAD).unwrap();
        bad_magic[0] ^= 0xff;
        assert_eq!(decrypt(KEY, &bad_magic, AAD), Err(StreamError::Malformed));
    }

    #[test]
    fn test_nonce_prefix_uniqueness_observable() {
        // Two encryptions of the same plaintext must differ (fresh random
        // nonce prefix per message — the STREAM freshness guarantee).
        let c1 = encrypt(KEY, b"same plaintext", AAD).unwrap();
        let c2 = encrypt(KEY, b"same plaintext", AAD).unwrap();
        assert_ne!(c1, c2);
        // ...and both decrypt to the same plaintext.
        assert_eq!(decrypt(KEY, &c1, AAD).unwrap(), b"same plaintext");
        assert_eq!(decrypt(KEY, &c2, AAD).unwrap(), b"same plaintext");
    }

    #[test]
    fn test_explicit_nonce_prefix_roundtrip() {
        let prefix = [3u8; NONCE_PREFIX_LEN];
        let ct = encrypt_with_nonce_prefix(KEY, b"explicit", AAD, &prefix).unwrap();
        assert_eq!(&ct[MAGIC.len()..MAGIC.len() + NONCE_PREFIX_LEN], &prefix);
        assert_eq!(decrypt(KEY, &ct, AAD).unwrap(), b"explicit");
    }

    #[test]
    fn test_all_zero_nonce_prefix_rejected() {
        assert_eq!(
            encrypt_with_nonce_prefix(KEY, b"x", AAD, &[0u8; NONCE_PREFIX_LEN]),
            Err(StreamError::NoEntropy)
        );
    }

    #[test]
    fn test_derive_key_deterministic() {
        assert_eq!(derive_key(b"pw"), derive_key(b"pw"));
        assert_ne!(derive_key(b"pw"), derive_key(b"pw2"));
    }
}
