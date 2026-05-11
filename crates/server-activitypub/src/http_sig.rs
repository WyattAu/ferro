use axum::http::{HeaderMap, Method};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct HttpSignature {
    pub key_id: String,
    pub algorithm: String,
    pub headers: Vec<String>,
    pub signature: Vec<u8>,
}

impl HttpSignature {
    pub fn parse(header_value: &str) -> Result<Self, String> {
        let mut key_id = None;
        let mut algorithm = None;
        let mut headers = None;
        let mut signature = None;

        for part in header_value.split(',') {
            let part = part.trim();
            if let Some((k, v)) = part.split_once('=') {
                let k = k.trim();
                let v = v.trim().trim_matches('"');
                match k {
                    "keyId" => key_id = Some(v.to_string()),
                    "algorithm" => algorithm = Some(v.to_string()),
                    "headers" => headers = Some(v.split_whitespace().map(String::from).collect()),
                    "signature" => {
                        signature = Some(
                            STANDARD
                                .decode(v)
                                .map_err(|e| format!("Invalid base64 signature: {}", e))?,
                        );
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            key_id: key_id.ok_or("Missing keyId in signature")?,
            algorithm: algorithm.unwrap_or_else(|| "hs2019".to_string()),
            headers: headers.unwrap_or_else(|| vec!["(request-target)".to_string()]),
            signature: signature.ok_or("Missing signature")?,
        })
    }

    pub fn signing_string(
        &self,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
    ) -> Result<String, String> {
        let mut lines = Vec::new();

        for header_name in &self.headers {
            match header_name.as_str() {
                "(request-target)" => {
                    lines.push(format!(
                        "(request-target): {} {}",
                        method.as_str().to_lowercase(),
                        path
                    ));
                }
                "(created)" => {
                    lines.push(format!("(created): {}", chrono::Utc::now().timestamp()));
                }
                name => {
                    let value = headers
                        .get(name)
                        .ok_or_else(|| format!("Missing header: {}", name))?
                        .to_str()
                        .map_err(|e| format!("Invalid header {}: {}", name, e))?;
                    lines.push(format!("{}: {}", name.to_lowercase(), value));
                }
            }
        }

        Ok(lines.join("\n"))
    }

    pub fn verify_hmac(
        &self,
        method: &Method,
        path: &str,
        headers: &HeaderMap,
        secret: &str,
    ) -> Result<bool, String> {
        let signing_string = self.signing_string(method, path, headers)?;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .map_err(|e| format!("HMAC error: {}", e))?;
        mac.update(signing_string.as_bytes());
        mac.verify_slice(&self.signature)
            .map(|_| true)
            .map_err(|e| format!("Signature verification failed: {}", e))
    }
}

pub fn actor_from_key_id(key_id: &str) -> String {
    key_id.split('#').next().unwrap_or(key_id).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_signature(key_id: &str, headers: &[&str], sig_b64: &str) -> String {
        format!(
            r#"keyId="{}",algorithm="hs2019",headers="{}",signature="{}""#,
            key_id,
            headers.join(" "),
            sig_b64
        )
    }

    #[test]
    fn test_parse_signature() {
        let sig = make_signature(
            "https://example.com/actor#key",
            &["(request-target)", "host", "date"],
            "dGVzdA==",
        );
        let parsed = HttpSignature::parse(&sig).unwrap();
        assert_eq!(parsed.key_id, "https://example.com/actor#key");
        assert_eq!(parsed.algorithm, "hs2019");
        assert_eq!(parsed.headers, vec!["(request-target)", "host", "date"]);
        assert_eq!(parsed.signature, b"test");
    }

    #[test]
    fn test_parse_missing_key_id() {
        let sig = r#"algorithm="hs2019",signature="dGVzdA==""#;
        assert!(HttpSignature::parse(sig).is_err());
    }

    #[test]
    fn test_signing_string_request_target() {
        let sig =
            HttpSignature::parse(r#"keyId="k",headers="(request-target)",signature="dGVzdA==""#)
                .unwrap();
        let string = sig
            .signing_string(&Method::POST, "/fed/inbox", &HeaderMap::new())
            .unwrap();
        assert_eq!(string, "(request-target): post /fed/inbox");
    }

    #[test]
    fn test_hmac_verify_roundtrip() {
        let secret = "test-secret-key";
        let signing_string = "(request-target): post /fed/inbox";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_string.as_bytes());
        let sig_bytes = mac.finalize().into_bytes();
        let sig_b64 = STANDARD.encode(sig_bytes);

        let sig = HttpSignature::parse(&format!(
            r#"keyId="k#main",headers="(request-target)",signature="{}""#,
            sig_b64
        ))
        .unwrap();

        let result = sig.verify_hmac(&Method::POST, "/fed/inbox", &HeaderMap::new(), secret);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_hmac_verify_wrong_secret() {
        let signing_string = "(request-target): post /fed/inbox";
        let mut mac = HmacSha256::new_from_slice("correct-secret".as_bytes()).unwrap();
        mac.update(signing_string.as_bytes());
        let sig_b64 = STANDARD.encode(mac.finalize().into_bytes());

        let sig = HttpSignature::parse(&format!(
            r#"keyId="k#main",headers="(request-target)",signature="{}""#,
            sig_b64
        ))
        .unwrap();

        let result = sig.verify_hmac(
            &Method::POST,
            "/fed/inbox",
            &HeaderMap::new(),
            "wrong-secret",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_actor_from_key_id() {
        assert_eq!(
            actor_from_key_id("https://example.com/actor#main-key"),
            "https://example.com/actor"
        );
        assert_eq!(
            actor_from_key_id("https://example.com/actor"),
            "https://example.com/actor"
        );
    }
}
