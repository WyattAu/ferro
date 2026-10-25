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

use ml_dsa::{
    EncodedSignature, EncodedVerifyingKey, MlDsa65, Seed, Signature as MlDsaSignature, Signer,
    SigningKey as MlDsaSigningKey, Verifier, VerifyingKey as MlDsaVerifyingKey,
};
use ring::signature::{self, KeyPair};

/// A signed epoch anchor: the bag digest plus its Ed25519 signature.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EpochAnchor {
    pub bag: [u8; 32],
    pub ed25519_signature: Vec<u8>,
    pub ed25519_public: Vec<u8>,
    /// ML-DSA-65 signature over the bag (YP-LIFECYCLE 1.1 dual-signing,
    /// FIPS 204). Present on dual-signed anchors only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mldsa65_signature: Option<Vec<u8>>,
    /// ML-DSA-65 verification key (1952-byte encoded form).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mldsa65_public: Option<Vec<u8>>,
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
        mldsa65_public: None,
    })
}

/// Generate an ML-DSA-65 seed from the OS CSPRNG (32 bytes; store this
/// alongside the Ed25519 PKCS#8 — it reconstructs the PQ signing key).
pub fn generate_mldsa65_seed() -> Result<[u8; 32], ring::error::Unspecified> {
    use ring::rand::{SecureRandom, SystemRandom};
    let mut seed = [0u8; 32];
    SystemRandom::new().fill(&mut seed)?;
    Ok(seed)
}

/// Sign the bagged anchor with BOTH algorithms (YP-LIFECYCLE 1.1):
/// Ed25519 + ML-DSA-65 (FIPS 204, deterministic variant).
pub fn sign_anchor_dual(
    ed25519_pkcs8: &[u8],
    mldsa65_seed: &[u8; 32],
    bag: &[u8; 32],
) -> Result<EpochAnchor, ring::error::Unspecified> {
    let mut anchor = sign_anchor(ed25519_pkcs8, bag)?;
    let sk = MlDsaSigningKey::<MlDsa65>::from_seed(&Seed::from(*mldsa65_seed));
    let vk: &MlDsaVerifyingKey<MlDsa65> = sk.as_ref();
    let sig = sk.sign(bag);
    anchor.mldsa65_signature = Some(sig.encode().as_slice().to_vec());
    anchor.mldsa65_public = Some(vk.encode().as_slice().to_vec());
    Ok(anchor)
}

/// Verify an anchor: the Ed25519 signature must verify against the anchor's
/// own public key over the bag digest. Checks the classical half only —
/// for the YP dual contract use `verify_anchor_dual`.
#[must_use]
pub fn verify_anchor(anchor: &EpochAnchor) -> bool {
    signature::UnparsedPublicKey::new(&signature::ED25519, &anchor.ed25519_public)
        .verify(&anchor.bag, &anchor.ed25519_signature)
        .is_ok()
}

/// Verify a dual-signed anchor: BOTH the Ed25519 and ML-DSA-65 signatures
/// must be present and valid over the same bag (YP-LIFECYCLE 1.1 — a
/// missing PQ half fails; a classical-only anchor is not dual-valid).
#[must_use]
pub fn verify_anchor_dual(anchor: &EpochAnchor) -> bool {
    if !verify_anchor(anchor) {
        return false;
    }
    let (Some(sig), Some(vk)) = (&anchor.mldsa65_signature, &anchor.mldsa65_public) else {
        return false;
    };
    let (Some(sig), Some(vk)) = (decode_mldsa_sig(sig), decode_mldsa_vk(vk)) else {
        return false;
    };
    vk.verify(&anchor.bag, &sig).is_ok()
}

/// FIPS 204 ML-DSA-65 encoded sizes.
const MLDSA65_SIG_LEN: usize = 3309;
const MLDSA65_VK_LEN: usize = 1952;

fn decode_mldsa_sig(bytes: &[u8]) -> Option<MlDsaSignature<MlDsa65>> {
    if bytes.len() != MLDSA65_SIG_LEN {
        return None;
    }
    let mut arr = [0u8; MLDSA65_SIG_LEN];
    arr.copy_from_slice(bytes);
    MlDsaSignature::decode(&EncodedSignature::<MlDsa65>::from(arr))
}

fn decode_mldsa_vk(bytes: &[u8]) -> Option<MlDsaVerifyingKey<MlDsa65>> {
    if bytes.len() != MLDSA65_VK_LEN {
        return None;
    }
    let mut arr = [0u8; MLDSA65_VK_LEN];
    arr.copy_from_slice(bytes);
    Some(MlDsaVerifyingKey::decode(&EncodedVerifyingKey::<MlDsa65>::from(arr)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_sign_and_verify() {
        let ed = generate_signing_key().unwrap();
        let pq = generate_mldsa65_seed().unwrap();
        let bag = [7u8; 32];
        let anchor = sign_anchor_dual(&ed, &pq, &bag).unwrap();
        assert!(verify_anchor(&anchor));
        assert!(verify_anchor_dual(&anchor));
        assert_eq!(anchor.mldsa65_signature.as_ref().unwrap().len(), 3309);
        assert_eq!(anchor.mldsa65_public.as_ref().unwrap().len(), 1952);
    }

    #[test]
    fn test_classical_only_anchor_not_dual_valid() {
        // YP-LIFECYCLE: a missing PQ half is not dual-valid.
        let ed = generate_signing_key().unwrap();
        let anchor = sign_anchor(&ed, &[1u8; 32]).unwrap();
        assert!(verify_anchor(&anchor));
        assert!(!verify_anchor_dual(&anchor));
    }

    #[test]
    fn test_dual_tampered_pq_signature_rejected() {
        let ed = generate_signing_key().unwrap();
        let pq = generate_mldsa65_seed().unwrap();
        let mut anchor = sign_anchor_dual(&ed, &pq, &[2u8; 32]).unwrap();
        let n = anchor.mldsa65_signature.as_mut().unwrap().len();
        anchor.mldsa65_signature.as_mut().unwrap()[n - 1] ^= 0xff;
        assert!(verify_anchor(&anchor), "ed25519 half still valid");
        assert!(!verify_anchor_dual(&anchor));
    }

    #[test]
    fn test_dual_tampered_bag_rejected_by_both() {
        let ed = generate_signing_key().unwrap();
        let pq = generate_mldsa65_seed().unwrap();
        let mut anchor = sign_anchor_dual(&ed, &pq, &[3u8; 32]).unwrap();
        anchor.bag = [4u8; 32];
        assert!(!verify_anchor(&anchor));
        assert!(!verify_anchor_dual(&anchor));
    }

    #[test]
    fn test_dual_truncated_pq_fields_rejected() {
        let ed = generate_signing_key().unwrap();
        let pq = generate_mldsa65_seed().unwrap();
        let mut anchor = sign_anchor_dual(&ed, &pq, &[5u8; 32]).unwrap();
        anchor.mldsa65_public.as_mut().unwrap().truncate(100);
        assert!(!verify_anchor_dual(&anchor));
    }

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
