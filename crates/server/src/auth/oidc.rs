use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use common::auth::{Claims, is_public_auth_path};
use common::error::{FerroError, Result};
use jsonwebtoken::{Validation, decode_header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

use crate::users::UserInfo;

/// OIDC provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub audience: String,
    pub jwks_uri: Option<String>,
}

struct JwksCache {
    keys: HashMap<String, jsonwebtoken::DecodingKey>,
    fetched_at: Instant,
    ttl: Duration,
}

/// OIDC token validator with JWKS key caching and PKCE session support.
#[derive(Clone)]
pub struct OidcValidator {
    config: Arc<OidcConfig>,
    jwks: Arc<RwLock<JwksCache>>,
    pkce_sessions: Arc<RwLock<HashMap<String, PkceSession>>>,
    http_client: reqwest::Client,
}

/// Short-lived PKCE session stored between login redirect and callback.
pub struct PkceSession {
    pub code_verifier: String,
    pub redirect_uri: String,
    pub callback_url: String,
    created_at: Instant,
}

impl OidcValidator {
    pub fn new(config: OidcConfig) -> Self {
        Self {
            config: Arc::new(config),
            jwks: Arc::new(RwLock::new(JwksCache {
                keys: HashMap::new(),
                fetched_at: Instant::now(),
                ttl: Duration::from_secs(86400),
            })),
            pkce_sessions: Arc::new(RwLock::new(HashMap::new())),
            http_client: reqwest::Client::new(),
        }
    }

    /// Return the OIDC configuration.
    pub fn config(&self) -> &OidcConfig {
        &self.config
    }

    /// Store a PKCE session for later callback verification.
    pub async fn store_pkce_session(
        &self,
        state: &str,
        code_verifier: &str,
        redirect_uri: &str,
        callback_url: &str,
    ) {
        let mut sessions = self.pkce_sessions.write().await;
        sessions.insert(
            state.to_string(),
            PkceSession {
                code_verifier: code_verifier.to_string(),
                redirect_uri: redirect_uri.to_string(),
                callback_url: callback_url.to_string(),
                created_at: Instant::now(),
            },
        );
        // Cleanup old sessions (older than 10 minutes)
        let cutoff = Instant::now() - Duration::from_secs(600);
        sessions.retain(|_, s| s.created_at > cutoff);
    }

    /// Consume a PKCE session (removes it from the cache).
    pub async fn consume_pkce_session(&self, state: &str) -> Option<PkceSession> {
        let mut sessions = self.pkce_sessions.write().await;
        let cutoff = Instant::now() - Duration::from_secs(600);
        sessions.retain(|_, s| s.created_at > cutoff);
        sessions.remove(state)
    }

    /// Exchange an authorization code for tokens using PKCE.
    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<serde_json::Value> {
        // Discover the token endpoint from the issuer
        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            self.config.issuer.trim_end_matches('/')
        );
        let discovery: serde_json::Value = self
            .http_client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| FerroError::Internal(format!("OIDC discovery failed: {}", e)))?
            .json()
            .await
            .map_err(|e| FerroError::Internal(format!("OIDC discovery parse failed: {}", e)))?;

        let token_endpoint = discovery
            .get("token_endpoint")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FerroError::Internal("No token_endpoint in discovery".to_string()))?;

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &self.config.client_id),
            ("code_verifier", code_verifier),
            ("redirect_uri", redirect_uri),
        ];

        let response = self
            .http_client
            .post(token_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(|e| FerroError::Internal(format!("Token exchange request failed: {}", e)))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| {
                FerroError::Internal(format!("Token exchange response parse failed: {}", e))
            })?;

        Ok(response)
    }

    pub async fn validate_token(&self, token: &str) -> Result<Claims> {
        let cache = self.jwks.read().await;
        if !cache.keys.is_empty()
            && cache.fetched_at.elapsed() < cache.ttl
            && let Some(claims) = self.try_validate_with_keys(token, &cache.keys)
        {
            return Ok(claims);
        }
        drop(cache);

        self.refresh_jwks().await?;

        let cache = self.jwks.read().await;
        self.try_validate_with_keys(token, &cache.keys)
            .ok_or(FerroError::Unauthorized)
    }

    fn try_validate_with_keys(
        &self,
        token: &str,
        keys: &HashMap<String, jsonwebtoken::DecodingKey>,
    ) -> Option<Claims> {
        let header = decode_header(token).ok()?;
        let kid = header.kid?;
        let decoding_key = keys.get(&kid)?;

        let mut validation = Validation::new(header.alg);
        validation.set_audience(&[&self.config.audience]);
        validation.set_issuer(&[&self.config.issuer]);

        let token_data = jsonwebtoken::decode::<Claims>(token, decoding_key, &validation).ok()?;
        Some(token_data.claims)
    }

    async fn refresh_jwks(&self) -> Result<()> {
        let jwks_uri = match &self.config.jwks_uri {
            Some(uri) => uri.clone(),
            None => format!(
                "{}/.well-known/openid-configuration",
                self.config.issuer.trim_end_matches('/')
            ),
        };

        let actual_jwks_uri = if self.config.jwks_uri.is_none() {
            let discovery: serde_json::Value = self
                .http_client
                .get(&jwks_uri)
                .send()
                .await
                .map_err(|e| FerroError::Internal(format!("OIDC discovery failed: {}", e)))?
                .json()
                .await
                .map_err(|e| FerroError::Internal(format!("OIDC discovery parse failed: {}", e)))?;

            discovery
                .get("jwks_uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    FerroError::Internal("No jwks_uri in discovery document".to_string())
                })?
                .to_string()
        } else {
            jwks_uri
        };

        let jwks_response: serde_json::Value = self
            .http_client
            .get(&actual_jwks_uri)
            .send()
            .await
            .map_err(|e| FerroError::Internal(format!("JWKS fetch failed: {}", e)))?
            .json()
            .await
            .map_err(|e| FerroError::Internal(format!("JWKS parse failed: {}", e)))?;

        let jwk_set: jsonwebtoken::jwk::JwkSet = serde_json::from_value(jwks_response)
            .map_err(|e| FerroError::Internal(format!("JWKS deserialize failed: {}", e)))?;

        let mut keys = HashMap::new();
        for jwk in jwk_set.keys.into_iter() {
            let kid = jwk.common.key_id.clone().unwrap_or_default();
            if kid.is_empty() {
                continue;
            }

            match jsonwebtoken::DecodingKey::from_jwk(&jwk) {
                Ok(decoding_key) => {
                    keys.insert(kid, decoding_key);
                }
                Err(e) => {
                    debug!("Failed to create DecodingKey for kid={}: {}", kid, e);
                }
            }
        }

        let mut cache = self.jwks.write().await;
        cache.keys = keys;
        cache.fetched_at = Instant::now();

        debug!("JWKS refreshed with {} keys", cache.keys.len());
        Ok(())
    }
}

/// Decode JWT claims without signature verification (for development/testing).
#[cfg(test)]
fn decode_claims_unsafe(token: &str) -> Result<Claims> {
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(FerroError::Unauthorized);
    }

    let _header = decode_header(token).map_err(|_| FerroError::Unauthorized)?;

    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| FerroError::Unauthorized)?;

    let claims: Claims =
        serde_json::from_slice(&payload_bytes).map_err(|_| FerroError::Unauthorized)?;

    let now = jsonwebtoken::get_current_timestamp();
    if claims.exp != 0 && claims.exp < now {
        return Err(FerroError::Unauthorized);
    }

    Ok(claims)
}

/// Optional auth middleware: if OIDC is configured, validates the Bearer token.
/// If not configured, allows all requests through with anonymous claims.
/// The resulting Claims are inserted as a request Extension.
///
/// When a `UserInfo` extension is already present (set by API key middleware),
/// OIDC validation is skipped and synthetic claims are generated for the
/// API key-authenticated user.
pub async fn auth_middleware(
    oidc: Option<Arc<OidcValidator>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();

    if is_public_auth_path(path) {
        return next.run(request).await;
    }

    // If UserInfo was already set by API key middleware, skip OIDC validation
    // and generate synthetic Claims for the API key user.
    if let Some(user_info) = request.extensions().get::<crate::users::UserInfo>().cloned() {
        let claims = Claims {
            sub: user_info.user_id.clone(),
            aud: "ferro".to_string(),
            iss: "ferro-api-key".to_string(),
            exp: 0,
            iat: 0,
            nonce: None,
            email: None,
            name: Some(user_info.username.clone()),
            groups: None,
        };
        let user_sub = claims.sub.clone();
        request.extensions_mut().insert(claims);
        request.headers_mut().insert(
            "X-Ferro-User",
            axum::http::HeaderValue::from_str(&user_sub)
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("anonymous")),
        );
        return next.run(request).await;
    }

    let token = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let claims = match (&oidc, token) {
        (Some(validator), Some(t)) => match validator.validate_token(t).await {
            Ok(claims) => claims,
            Err(e) => {
                warn!("Token validation failed: {}", e);
                return crate::api_error::ApiError::unauthorized(
                    crate::api_error::ApiError::TOKEN_INVALID,
                    "Token validation failed",
                );
            }
        },
        (Some(_), None) => {
            return crate::api_error::ApiError::unauthorized(
                crate::api_error::ApiError::AUTH_REQUIRED,
                "Missing Bearer token",
            );
        }
        (None, _) => Claims::anonymous(),
    };

    let user_sub = claims.sub.clone();
    request.extensions_mut().insert(claims);
    request.headers_mut().insert(
        "X-Ferro-User",
        axum::http::HeaderValue::from_str(&user_sub)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("anonymous")),
    );
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_claims(sub: &str, exp: u64) -> common::auth::Claims {
        common::auth::Claims {
            sub: sub.to_string(),
            aud: "ferro".to_string(),
            iss: "https://auth.example.com".to_string(),
            exp,
            iat: 1000000,
            nonce: Some("test-nonce".to_string()),
            email: Some(format!("{}@example.com", sub)),
            name: Some(format!("Test {}", sub)),
            groups: Some(vec!["users".to_string()]),
        }
    }

    fn encode_claims_unsafe(claims: &common::auth::Claims) -> String {
        use base64::Engine;
        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        let header_json = serde_json::to_string(&header).unwrap();
        let claims_json = serde_json::to_string(claims).unwrap();

        let header_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
        let claims_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(claims_json.as_bytes());

        // Fake signature (not validated by decode_claims_unsafe)
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"fake-signature");

        format!("{}.{}.{}", header_b64, claims_b64, signature)
    }

    #[test]
    fn test_decode_claims_unsafe_valid() {
        let claims = make_claims("alice", 9999999999);
        let token = encode_claims_unsafe(&claims);

        let decoded = decode_claims_unsafe(&token).unwrap();
        assert_eq!(decoded.sub, "alice");
        assert_eq!(decoded.aud, "ferro");
        assert_eq!(decoded.iss, "https://auth.example.com");
        assert_eq!(decoded.email, Some("alice@example.com".to_string()));
        assert_eq!(decoded.name, Some("Test alice".to_string()));
        assert_eq!(decoded.groups, Some(vec!["users".to_string()]));
    }

    #[test]
    fn test_decode_claims_unsafe_expired() {
        let claims = make_claims("alice", 1000); // expired
        let token = encode_claims_unsafe(&claims);

        let result = decode_claims_unsafe(&token);
        assert!(result.is_err(), "Expired token should fail");
    }

    #[test]
    fn test_decode_claims_unsafe_no_expiry() {
        let mut claims = make_claims("alice", 0);
        claims.exp = 0; // no expiry
        let token = encode_claims_unsafe(&claims);

        let decoded = decode_claims_unsafe(&token).unwrap();
        assert_eq!(decoded.sub, "alice");
    }

    #[test]
    fn test_decode_claims_invalid_format() {
        assert!(decode_claims_unsafe("not-a-jwt").is_err());
        assert!(decode_claims_unsafe("only.two").is_err());
        assert!(decode_claims_unsafe("").is_err());
    }

    #[test]
    fn test_is_public_auth_path() {
        // Uses the canonical common::auth::is_public_auth_path function.
        // Verify the full set of public paths is recognized.
        assert!(is_public_auth_path("/.well-known/ferro"));
        assert!(is_public_auth_path("/.well-known/openid-configuration"));
        assert!(is_public_auth_path("/api/auth/login"));
        assert!(is_public_auth_path("/api/auth/login?redirect=/files"));
        assert!(is_public_auth_path("/api/auth/callback?code=test&state=s"));
        assert!(is_public_auth_path("/healthz"));
        assert!(is_public_auth_path("/api/config"));
        assert!(is_public_auth_path("/api/auth/info"));
        assert!(is_public_auth_path("/metrics"));
        assert!(is_public_auth_path("/ui"));
        assert!(is_public_auth_path("/ui/"));
        assert!(is_public_auth_path("/api/policies"));
        assert!(is_public_auth_path("/s/abc123"));
        assert!(!is_public_auth_path("/files/test.txt"));
        assert!(!is_public_auth_path("/api/audit"));
        assert!(!is_public_auth_path("/"));
    }

    #[test]
    fn test_oidc_config_creation() {
        let config = OidcConfig {
            issuer: "https://auth.example.com".to_string(),
            client_id: "ferro-client".to_string(),
            audience: "ferro".to_string(),
            jwks_uri: Some("https://auth.example.com/.well-known/jwks.json".to_string()),
        };

        let _validator = OidcValidator::new(config);
    }

    #[test]
    fn test_oidc_config_no_jwks_uri() {
        let config = OidcConfig {
            issuer: "https://auth.example.com".to_string(),
            client_id: "ferro-client".to_string(),
            audience: "ferro".to_string(),
            jwks_uri: None,
        };

        let _validator = OidcValidator::new(config);
    }
}
