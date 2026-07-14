use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use rand::RngCore as _;
use serde::{Deserialize, Serialize};

use crate::encryption::{decrypt_content, encrypt_content};
use crate::error::ApiError;

#[derive(Deserialize, Serialize)]
pub struct E2eeEncryptRequest {
    pub data: String,
    pub passphrase: String,
}

#[derive(Serialize, Deserialize)]
pub struct E2eeEncryptResponse {
    pub ciphertext: String,
}

pub async fn e2ee_encrypt(axum::Json(req): axum::Json<E2eeEncryptRequest>) -> Response {
    let data = match base64::engine::general_purpose::STANDARD.decode(&req.data) {
        Ok(d) => d,
        Err(e) => {
            return ApiError::bad_request(ApiError::ENCRYPT_FAILED, format!("Invalid base64 data: {e}"));
        }
    };

    match encrypt_content(&data, &req.passphrase).await {
        Ok(ciphertext) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&ciphertext);
            (StatusCode::OK, axum::Json(E2eeEncryptResponse { ciphertext: b64 })).into_response()
        }
        Err(e) => ApiError::internal(ApiError::INTERNAL_ERROR, format!("Encryption failed: {e}")),
    }
}

pub async fn e2ee_decrypt(axum::Json(req): axum::Json<E2eeEncryptRequest>) -> Response {
    let ciphertext = match base64::engine::general_purpose::STANDARD.decode(&req.data) {
        Ok(d) => d,
        Err(e) => {
            return ApiError::bad_request(ApiError::DECRYPT_FAILED, format!("Invalid base64 data: {e}"));
        }
    };

    match decrypt_content(&ciphertext, &req.passphrase).await {
        Ok(plaintext) => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&plaintext);
            (StatusCode::OK, axum::Json(E2eeEncryptResponse { ciphertext: b64 })).into_response()
        }
        Err(e) => ApiError::bad_request(
            ApiError::DECRYPT_FAILED,
            format!("Decryption failed: wrong passphrase? {e}"),
        ),
    }
}

#[derive(Serialize, Deserialize)]
pub struct E2eeKeyGenerateResponse {
    pub key_id: String,
    pub public_key: String,
    pub algorithm: String,
    pub created_at: i64,
}

pub async fn e2ee_key_generate() -> Response {
    let mut secret_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut secret_bytes);
    let secret = x25519_dalek::StaticSecret::from(secret_bytes);
    let public_key = x25519_dalek::PublicKey::from(&secret);
    let public_key_bytes = public_key.as_bytes();
    let public_key_hex = hex::encode(public_key_bytes);

    use sha2::{Digest, Sha256};
    let key_id = hex::encode(Sha256::digest(public_key_bytes));

    let created_at = chrono::Utc::now().timestamp();

    (
        StatusCode::OK,
        axum::Json(E2eeKeyGenerateResponse {
            key_id,
            public_key: public_key_hex,
            algorithm: "x25519".to_string(),
            created_at,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn make_app() -> axum::Router<()> {
        axum::Router::new()
            .route("/e2ee/encrypt", axum::routing::post(e2ee_encrypt))
            .route("/e2ee/decrypt", axum::routing::post(e2ee_decrypt))
            .route("/e2ee/key/generate", axum::routing::post(e2ee_key_generate))
    }

    #[tokio::test]
    async fn test_encrypt_roundtrip() {
        let original = b"hello e2ee world";
        let b64_data = base64::engine::general_purpose::STANDARD.encode(original);

        let encrypt_req = E2eeEncryptRequest {
            data: b64_data.clone(),
            passphrase: "test-pass-123".to_string(),
        };
        let encrypt_resp = make_app()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/e2ee/encrypt")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&encrypt_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(encrypt_resp.status(), StatusCode::OK);
        let body = encrypt_resp.into_body().collect().await.unwrap().to_bytes();
        let enc_resp: E2eeEncryptResponse = serde_json::from_slice(&body).unwrap();

        let decrypt_req = E2eeEncryptRequest {
            data: enc_resp.ciphertext.clone(),
            passphrase: "test-pass-123".to_string(),
        };
        let decrypt_resp = make_app()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/e2ee/decrypt")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&decrypt_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(decrypt_resp.status(), StatusCode::OK);
        let body = decrypt_resp.into_body().collect().await.unwrap().to_bytes();
        let dec_resp: E2eeEncryptResponse = serde_json::from_slice(&body).unwrap();

        let roundtrip = base64::engine::general_purpose::STANDARD
            .decode(&dec_resp.ciphertext)
            .unwrap();
        assert_eq!(roundtrip, original);
    }

    #[tokio::test]
    async fn test_key_generate_format() {
        let resp = make_app()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/e2ee/key/generate")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let key_resp: E2eeKeyGenerateResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(key_resp.algorithm, "x25519");
        assert!(!key_resp.key_id.is_empty());
        assert!(!key_resp.public_key.is_empty());
        assert!(key_resp.created_at > 0);
        assert_eq!(key_resp.public_key.len(), 64);
        assert_eq!(key_resp.key_id.len(), 64);
    }

    #[tokio::test]
    async fn test_encrypt_wrong_passphrase() {
        let original = b"secret e2ee data";
        let b64_data = base64::engine::general_purpose::STANDARD.encode(original);

        let encrypt_req = E2eeEncryptRequest {
            data: b64_data,
            passphrase: "correct-pass".to_string(),
        };
        let encrypt_resp = make_app()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/e2ee/encrypt")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&encrypt_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = encrypt_resp.into_body().collect().await.unwrap().to_bytes();
        let enc_resp: E2eeEncryptResponse = serde_json::from_slice(&body).unwrap();

        let decrypt_req = E2eeEncryptRequest {
            data: enc_resp.ciphertext,
            passphrase: "wrong-pass".to_string(),
        };
        let decrypt_resp = make_app()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/e2ee/decrypt")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_string(&decrypt_req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(decrypt_resp.status(), StatusCode::BAD_REQUEST);
    }
}
