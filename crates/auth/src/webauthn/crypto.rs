use super::error::WebAuthnError;

/// Parsed COSE public key (EC2 or RSA).
#[derive(Debug, Clone)]
pub(crate) enum CosePublicKey {
    Ec2 { x: Vec<u8>, y: Vec<u8> },
    Rsa { n: Vec<u8>, e: Vec<u8> },
}

/// COSE key type constants.
pub(crate) const COSE_KTY_OKP: i64 = 1;
pub(crate) const COSE_KTY_EC2: i64 = 2;
pub(crate) const COSE_KTY_RSA: i64 = 3;

/// COSE algorithm constants.
pub(crate) const COSE_ALG_ES256: i32 = -7;
pub(crate) const COSE_ALG_RS256: i32 = -257;

/// COSE key map parameter labels.
const COSE_KEY_KTY: i64 = 1;
const COSE_KEY_ALG: i64 = 2;
const COSE_KEY_CRV_N: i64 = -1;
const COSE_KEY_X_E: i64 = -2;
const COSE_KEY_Y: i64 = -3;

/// COSE curve identifiers.
const COSE_CRV_P256: i64 = 1;

/// Parse a CBOR integer from a value, handling both positive and negative.
fn cbor_i64(val: &ciborium::Value) -> Option<i64> {
    use ciborium::Value;
    match val {
        Value::Integer(i) => (*i).try_into().ok(),
        _ => None,
    }
}

/// Parse a CBOR byte string from a value.
pub(crate) fn cbor_bytes(val: &ciborium::Value) -> Option<Vec<u8>> {
    use ciborium::Value;
    match val {
        Value::Bytes(b) => Some(b.clone()),
        Value::Text(t) => Some(t.as_bytes().to_vec()),
        _ => None,
    }
}

/// Parse a CBOR map into a key-value vec of (i64, Value).
pub(crate) fn cbor_map_entries(val: &ciborium::Value) -> Option<Vec<(i64, ciborium::Value)>> {
    use ciborium::Value;
    match val {
        Value::Map(entries) => {
            let mut result = Vec::with_capacity(entries.len());
            for (k, v) in entries {
                let key = cbor_i64(k)?;
                result.push((key, v.clone()));
            }
            Some(result)
        }
        _ => None,
    }
}

/// Parse a COSE key from its CBOR encoding.
pub(crate) fn parse_cose_key(cose_bytes: &[u8]) -> Result<(i32, CosePublicKey), WebAuthnError> {
    use ciborium::Value;

    let key_val: Value = ciborium::de::from_reader(cose_bytes)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("COSE key CBOR parse error: {e}")))?;

    let entries = cbor_map_entries(&key_val)
        .ok_or_else(|| WebAuthnError::VerificationFailed("COSE key is not a CBOR map".to_string()))?;

    let mut kty: Option<i64> = None;
    let mut alg: Option<i32> = None;
    let mut crv: Option<i64> = None;
    let mut x: Option<Vec<u8>> = None;
    let mut y: Option<Vec<u8>> = None;
    let mut n: Option<Vec<u8>> = None;
    let mut e: Option<Vec<u8>> = None;

    for (label, val) in &entries {
        match *label {
            COSE_KEY_KTY => kty = cbor_i64(val),
            COSE_KEY_ALG => alg = cbor_i64(val).map(|v| v as i32),
            COSE_KEY_CRV_N => match kty {
                Some(COSE_KTY_RSA) => n = cbor_bytes(val),
                _ => crv = cbor_i64(val),
            },
            COSE_KEY_X_E => match kty {
                Some(COSE_KTY_RSA) => e = cbor_bytes(val),
                _ => x = cbor_bytes(val),
            },
            COSE_KEY_Y => y = cbor_bytes(val),
            _ => {}
        }
    }

    let kty = kty.ok_or_else(|| WebAuthnError::VerificationFailed("COSE key missing 'kty'".to_string()))?;
    let alg = alg.ok_or_else(|| WebAuthnError::VerificationFailed("COSE key missing 'alg'".to_string()))?;

    match kty {
        COSE_KTY_EC2 => {
            let crv = crv.ok_or_else(|| WebAuthnError::VerificationFailed("EC2 key missing 'crv'".to_string()))?;
            if crv != COSE_CRV_P256 {
                return Err(WebAuthnError::UnsupportedAlgorithm(alg));
            }
            let x = x.ok_or_else(|| WebAuthnError::VerificationFailed("EC2 key missing 'x'".to_string()))?;
            let y = y.ok_or_else(|| WebAuthnError::VerificationFailed("EC2 key missing 'y'".to_string()))?;
            Ok((alg, CosePublicKey::Ec2 { x, y }))
        }
        COSE_KTY_RSA => {
            let n = n.ok_or_else(|| WebAuthnError::VerificationFailed("RSA key missing 'n'".to_string()))?;
            let e = e.ok_or_else(|| WebAuthnError::VerificationFailed("RSA key missing 'e'".to_string()))?;
            Ok((alg, CosePublicKey::Rsa { n, e }))
        }
        COSE_KTY_OKP => Err(WebAuthnError::UnsupportedAlgorithm(alg)),
        other => Err(WebAuthnError::VerificationFailed(format!(
            "Unsupported COSE key type: {other}"
        ))),
    }
}

/// Verify a COSE signature using the parsed public key.
pub(crate) fn verify_cose_signature(
    alg: i32,
    public_key: &CosePublicKey,
    signed_data: &[u8],
    signature: &[u8],
) -> Result<(), WebAuthnError> {
    use ring::signature;

    match alg {
        COSE_ALG_ES256 => {
            let CosePublicKey::Ec2 { x, y } = public_key else {
                return Err(WebAuthnError::VerificationFailed(
                    "EC2 key expected for ES256".to_string(),
                ));
            };

            let mut public_key_bytes = Vec::with_capacity(1 + x.len() + y.len());
            public_key_bytes.push(0x04);
            public_key_bytes.extend_from_slice(x);
            public_key_bytes.extend_from_slice(y);

            let public_key = signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_FIXED, &public_key_bytes);
            public_key
                .verify(signed_data, signature)
                .map_err(|_| WebAuthnError::SignatureVerificationFailed)?;
            Ok(())
        }
        COSE_ALG_RS256 => {
            let CosePublicKey::Rsa { n, e } = public_key else {
                return Err(WebAuthnError::VerificationFailed(
                    "RSA key expected for RS256".to_string(),
                ));
            };

            let rsa_public_key = RsaPublicKeyDer { n, e };
            let der_bytes = rsa_public_key.to_der()?;

            let public_key = signature::UnparsedPublicKey::new(&signature::RSA_PKCS1_2048_8192_SHA256, &der_bytes);
            public_key
                .verify(signed_data, signature)
                .map_err(|_| WebAuthnError::SignatureVerificationFailed)?;
            Ok(())
        }
        other => Err(WebAuthnError::UnsupportedAlgorithm(other)),
    }
}

/// ASN.1 DER encoding helper for RSA public keys.
struct RsaPublicKeyDer<'a> {
    n: &'a [u8],
    e: &'a [u8],
}

impl RsaPublicKeyDer<'_> {
    fn to_der(&self) -> Result<Vec<u8>, WebAuthnError> {
        let mut der = Vec::new();

        let alg_id = Self::encode_sequence_owned({
            let mut buf = Vec::new();
            buf.extend_from_slice(&[0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B]);
            buf.extend_from_slice(&[0x05, 0x00]);
            buf
        });

        let rsa_key = Self::encode_sequence_owned({
            let mut buf = Vec::new();
            Self::encode_integer(&mut buf, self.n);
            Self::encode_integer(&mut buf, self.e);
            buf
        });

        der.extend_from_slice(&alg_id);
        Self::encode_bit_string(&mut der, &rsa_key);
        Self::encode_sequence_in_place(&mut der);

        Ok(der)
    }

    fn encode_integer(buf: &mut Vec<u8>, value: &[u8]) {
        let mut v = value;
        while v.len() > 1 && v[0] == 0 {
            v = &v[1..];
        }
        if v.first().is_some_and(|&b| b & 0x80 != 0) {
            buf.push(0x02);
            buf.push((v.len() + 1) as u8);
            buf.push(0x00);
            buf.extend_from_slice(v);
        } else {
            buf.push(0x02);
            buf.push(v.len() as u8);
            buf.extend_from_slice(v);
        }
    }

    fn encode_bit_string(buf: &mut Vec<u8>, content: &[u8]) {
        let len = content.len() + 1;
        buf.push(0x03);
        Self::encode_length(buf, len);
        buf.push(0x00);
        buf.extend_from_slice(content);
    }

    fn encode_sequence_owned(content: Vec<u8>) -> Vec<u8> {
        let mut result = Vec::with_capacity(2 + content.len());
        result.push(0x30);
        Self::encode_length(&mut result, content.len());
        result.extend_from_slice(&content);
        result
    }

    fn encode_sequence_in_place(buf: &mut Vec<u8>) {
        let content_len = buf.len();
        let header_len = if content_len < 0x80 {
            2
        } else if content_len < 0x100 {
            3
        } else {
            4
        };
        buf.resize(content_len + header_len, 0);
        for i in (0..content_len).rev() {
            buf[i + header_len] = buf[i];
        }
        buf[0] = 0x30;
        Self::encode_length_at(buf, 1, content_len);
    }

    fn encode_length(buf: &mut Vec<u8>, len: usize) {
        if len < 0x80 {
            buf.push(len as u8);
        } else if len < 0x100 {
            buf.push(0x81);
            buf.push(len as u8);
        } else {
            buf.push(0x82);
            buf.push((len >> 8) as u8);
            buf.push((len & 0xFF) as u8);
        }
    }

    fn encode_length_at(buf: &mut [u8], offset: usize, len: usize) {
        if len < 0x80 {
            buf[offset] = len as u8;
        } else if len < 0x100 {
            buf[offset] = 0x81;
            buf[offset + 1] = len as u8;
        } else {
            buf[offset] = 0x82;
            buf[offset + 1] = (len >> 8) as u8;
            buf[offset + 2] = (len & 0xFF) as u8;
        }
    }
}

/// Convert a COSE algorithm ID to a human-readable name.
pub(crate) fn alg_to_name(alg: i32) -> &'static str {
    match alg {
        COSE_ALG_ES256 => "ES256",
        COSE_ALG_RS256 => "RS256",
        _ => "unknown",
    }
}

/// Generate 32 random bytes for a `WebAuthn` challenge.
pub(crate) fn generate_challenge_bytes() -> Vec<u8> {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.to_vec()
}

/// Base64url encode (no padding).
pub(crate) fn base64_encode_urlsafe(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Base64url decode (no padding).
pub(crate) fn base64_decode_urlsafe(data: &str) -> Result<Vec<u8>, WebAuthnError> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|e| WebAuthnError::VerificationFailed(format!("base64 decode error: {e}")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ring::signature::KeyPair as _;

    pub(crate) fn build_cose_ec2_key(x: &[u8], y: &[u8]) -> Vec<u8> {
        use ciborium::Value;
        let map = vec![
            (Value::Integer(1.into()), Value::Integer(2.into())),
            (Value::Integer(2.into()), Value::Integer((-7).into())),
            (Value::Integer((-1).into()), Value::Integer(1.into())),
            (Value::Integer((-2).into()), Value::Bytes(x.to_vec())),
            (Value::Integer((-3).into()), Value::Bytes(y.to_vec())),
        ];
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&Value::Map(map), &mut buf).unwrap();
        buf
    }

    pub(crate) fn build_cose_rsa_key(n: &[u8], e: &[u8]) -> Vec<u8> {
        use ciborium::Value;
        let map = vec![
            (Value::Integer(1.into()), Value::Integer(3.into())),
            (Value::Integer(2.into()), Value::Integer((-257).into())),
            (Value::Integer((-1).into()), Value::Bytes(n.to_vec())),
            (Value::Integer((-2).into()), Value::Bytes(e.to_vec())),
        ];
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&Value::Map(map), &mut buf).unwrap();
        buf
    }

    #[test]
    fn test_parse_cose_ec2_key() {
        let x = vec![0xAA; 32];
        let y = vec![0xBB; 32];
        let cose_key = build_cose_ec2_key(&x, &y);

        let (alg, key) = parse_cose_key(&cose_key).unwrap();
        assert_eq!(alg, -7);
        match key {
            CosePublicKey::Ec2 { x: kx, y: ky } => {
                assert_eq!(kx, x);
                assert_eq!(ky, y);
            }
            _ => panic!("Expected EC2 key"),
        }
    }

    #[test]
    fn test_parse_cose_rsa_key() {
        let n = vec![0xAA; 256];
        let e = vec![0x01, 0x00, 0x01];
        let cose_key = build_cose_rsa_key(&n, &e);

        let (alg, key) = parse_cose_key(&cose_key).unwrap();
        assert_eq!(alg, -257);
        match key {
            CosePublicKey::Rsa { n: kn, e: ke } => {
                assert_eq!(kn, n);
                assert_eq!(ke, e);
            }
            _ => panic!("Expected RSA key"),
        }
    }

    #[test]
    fn test_verify_cose_signature_es256() {
        use ring::signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair};

        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &rng).unwrap();
        let key_pair = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8.as_ref(), &rng).unwrap();

        let public_key_bytes = key_pair.public_key().as_ref().to_vec();
        let x = public_key_bytes[1..33].to_vec();
        let y = public_key_bytes[33..65].to_vec();

        let cose_key = CosePublicKey::Ec2 { x, y };
        let message = b"test message for WebAuthn";
        let signature = key_pair.sign(&rng, message).unwrap();

        let result = verify_cose_signature(COSE_ALG_ES256, &cose_key, message, signature.as_ref());
        assert!(result.is_ok());

        let wrong_result = verify_cose_signature(COSE_ALG_ES256, &cose_key, b"wrong message", signature.as_ref());
        assert!(matches!(wrong_result, Err(WebAuthnError::SignatureVerificationFailed)));
    }

    #[test]
    fn test_verify_cose_signature_rs256() {
        let n = vec![
            0xC0, 0xE9, 0x5A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let e = vec![0x01, 0x00, 0x01];
        let cose_key = build_cose_rsa_key(&n, &e);

        let (alg, key) = parse_cose_key(&cose_key).unwrap();
        assert_eq!(alg, -257);
        match key {
            CosePublicKey::Rsa { n: kn, e: ke } => {
                assert_eq!(kn, n);
                assert_eq!(ke, e);
            }
            _ => panic!("Expected RSA key"),
        }

        let rsa_key = RsaPublicKeyDer { n: &n, e: &e };
        let der = rsa_key.to_der().unwrap();
        assert_eq!(der[0], 0x30);
        assert!(der.len() > 40);
    }
}
