//! Epoch anchor signing for the MMR audit log (ALG-MMRLOG-001).
//!
//! Companion to `formal/Ferro/AuditChain.lean` (anchor tamper-evidence) and
//! YP-LIFECYCLE-COMPLIANCE-001 Rev 1.2. The bagged epoch anchor is signed
//! with Ed25519 (ring). The YP specifies a *dual* signature (Ed25519 +
//! ML-DSA-65); the ML-DSA half is staged pending a PQ signature crate in
//! the dependency tree — the anchor struct carries a slot for it so adding
//! the second signature is additive, not a format break.
//!
//! External assumption: Ed25519 unforgeability (as everywhere in this tree).

use ring::signature::{self, KeyPair};

/// A signed epoch anchor: the bag digest plus its Ed25519 signature.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EpochAnchor {
    pub bag: [u8; 32],
    pub ed25519_signature: Vec<u8>,
    pub ed25519_public: Vec<u8>,
    /// Reserved for the ML-DSA-65 signature (YP-LIFECYCLE 1.1 dual-signing).
    /// Absent until the PQ dependency lands; verifiers MUST NOT treat a
    /// missing ML-DSA signature as valid on its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mldsa65_signature: Option<Vec<u8>>,
}

/// Generate a fresh Ed25519 signing key, PKCS#8-encoded (store this — it is
/// the audit root of trust).
pub fn generate_signing_key() -> Result<Vec<u8>, ring::error::Unspecified> {
    let rng = ring::rand::SystemRandom::new();
    let doc = signature::Ed25519KeyPair::generate_pkcs8(&rng)?;
    Ok(doc.as_ref().to_vec())
}

/// Sign the bagged anchor.
pub fn sign_anchor(key_pkcs8: &[u8], bag: &[u8; 32]) -> Result<EpochAnchor, ring::error::Unspecified> {
    let pair = signature::Ed25519KeyPair::from_pkcs8_maybe_unchecked(key_pkcs8)?;
    let signature = pair.sign(bag);
    Ok(EpochAnchor {
        bag: *bag,
        ed25519_signature: signature.as_ref().to_vec(),
        ed25519_public: pair.public_key().as_ref().to_vec(),
        mldsa65_signature: None,
    })
}

/// Verify an anchor: the signature must verify against the anchor's own
/// public key over the bag digest.
#[must_use]
pub fn verify_anchor(anchor: &EpochAnchor) -> bool {
    signature::UnparsedPublicKey::new(&signature::ED25519, &anchor.ed25519_public)
        .verify(&anchor.bag, &anchor.ed25519_signature)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_and_verify() {
        let key = generate_signing_key().unwrap();
        let bag = [7u8; 32];
        let anchor = sign_anchor(&key, &bag).unwrap();
        assert!(verify_anchor(&anchor));
        assert_eq!(anchor.bag, bag);
        assert_eq!(anchor.ed25519_signature.len(), 64);
    }

    #[test]
    fn test_tampered_bag_rejected() {
        let key = generate_signing_key().unwrap();
        let mut anchor = sign_anchor(&key, &[1u8; 32]).unwrap();
        anchor.bag = [2u8; 32];
        assert!(!verify_anchor(&anchor));
    }

    #[test]
    fn test_tampered_signature_rejected() {
        let key = generate_signing_key().unwrap();
        let mut anchor = sign_anchor(&key, &[3u8; 32]).unwrap();
        anchor.ed25519_signature[0] ^= 0xff;
        assert!(!verify_anchor(&anchor));
    }

    #[test]
    fn test_different_key_signature_rejected() {
        let key_a = generate_signing_key().unwrap();
        let key_b = generate_signing_key().unwrap();
        let mut anchor = sign_anchor(&key_a, &[4u8; 32]).unwrap();
        let forged = sign_anchor(&key_b, &[4u8; 32]).unwrap();
        // Swap in the other key's public key: signature no longer matches.
        anchor.ed25519_public = forged.ed25519_public;
        assert!(!verify_anchor(&anchor));
    }

    #[test]
    fn test_stable_across_restarts_same_key() {
        // The same imported key must produce verifying anchors across
        // process restarts (deterministic Ed25519).
        let key = generate_signing_key().unwrap();
        let a = sign_anchor(&key, &[9u8; 32]).unwrap();
        let b = sign_anchor(&key, &[9u8; 32]).unwrap();
        assert_eq!(a.ed25519_signature, b.ed25519_signature);
        assert!(verify_anchor(&b));
    }
}
