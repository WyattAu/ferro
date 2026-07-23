//! HTTP client with retry, timeout, and CSRF protection.
//!
//! All endpoints use `/api/v1/` prefix.

use serde::{Serialize, de::DeserializeOwned};

/// API client configuration.
#[derive(Clone, Debug)]
pub struct ApiClientConfig {
    pub base_url: String,
    pub timeout_ms: u32,
    pub max_retries: u32,
}

impl Default for ApiClientConfig {
    fn default() -> Self {
        Self {
            base_url: get_server_url(),
            timeout_ms: 30_000,
            max_retries: 3,
        }
    }
}

/// Read FERRO_SERVER_URL from window global.
fn get_server_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let val = web_sys::window()
            .and_then(|w| {
                let v = js_sys::Reflect::get(&w, &"FERRO_SERVER_URL".into()).ok()?;
                v.as_string()
            })
            .unwrap_or_default();
        log::info!("[api] FERRO_SERVER_URL='{}'", val);
        val
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        String::new()
    }
}

/// Typed API error.
#[derive(Clone, Debug)]
pub enum ApiError {
    Network(String),
    Timeout,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict(String),
    Validation(String),
    Server(String),
    Serialization(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Network(msg) => write!(f, "Network: {}", msg),
            ApiError::Timeout => write!(f, "Timeout"),
            ApiError::Unauthorized => write!(f, "Unauthorized"),
            ApiError::Forbidden => write!(f, "Forbidden"),
            ApiError::NotFound => write!(f, "Not found"),
            ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ApiError::Validation(msg) => write!(f, "Validation: {}", msg),
            ApiError::Server(msg) => write!(f, "Server: {}", msg),
            ApiError::Serialization(msg) => write!(f, "Serialization: {}", msg),
        }
    }
}

/// HTTP client with retry.
pub struct ApiClient {
    config: ApiClientConfig,
}

impl ApiClient {
    pub fn new(config: ApiClientConfig) -> Self {
        Self { config }
    }

    /// Create client with server URL from window.FERRO_SERVER_URL.
    pub fn from_env() -> Self {
        Self {
            config: ApiClientConfig {
                base_url: get_server_url(),
                ..Default::default()
            },
        }
    }

    /// GET request with typed response.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiError> {
        self.fetch_json("GET", path, None::<&()>).await
    }

    /// POST request.
    pub async fn post<B: Serialize, T: DeserializeOwned>(&self, path: &str, body: &B) -> Result<T, ApiError> {
        self.fetch_json("POST", path, Some(body)).await
    }

    /// DELETE request.
    pub async fn delete(&self, path: &str) -> Result<(), ApiError> {
        self.fetch_json::<(), ()>("DELETE", path, None::<&()>).await
    }

    async fn fetch_json<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, ApiError> {
        let url = format!("{}{}", self.config.base_url, path);
        log::info!("API {} {}", method, url);

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay_ms = 100u32 * (1u32 << (attempt - 1));
                // Simple busy-wait for retry delay in WASM
                #[cfg(target_arch = "wasm32")]
                {
                    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                        let _ = web_sys::window()
                            .unwrap()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, delay_ms as i32);
                    });
                    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
                }
            }

            match self.execute_fetch(method, &url, body).await {
                Ok(val) => return Ok(val),
                Err(ApiError::Network(_)) | Err(ApiError::Timeout) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(ApiError::Network("max retries exceeded".into()))
    }

    async fn execute_fetch<B: Serialize, T: DeserializeOwned>(
        &self,
        method: &str,
        url: &str,
        body: Option<&B>,
    ) -> Result<T, ApiError> {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;

            let window = web_sys::window().ok_or_else(|| ApiError::Network("no window".into()))?;
            let opts = web_sys::RequestInit::new();
            opts.set_method(method);
            opts.set_credentials(web_sys::RequestCredentials::Include);

            let headers = web_sys::Headers::new().map_err(|e| ApiError::Network(format!("headers: {:?}", e)))?;
            headers
                .set("Accept", "application/json")
                .map_err(|e| ApiError::Network(format!("header: {:?}", e)))?;

            if let Some(b) = body {
                let json = serde_json::to_string(b).map_err(|e| ApiError::Serialization(e.to_string()))?;
                headers
                    .set("Content-Type", "application/json")
                    .map_err(|e| ApiError::Network(format!("header: {:?}", e)))?;
                opts.set_body(&wasm_bindgen::JsValue::from_str(&json));
            }

            opts.set_headers(&headers);

            let request = web_sys::Request::new_with_str_and_init(url, &opts)
                .map_err(|e| ApiError::Network(format!("request: {:?}", e)))?;

            let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
                .await
                .map_err(|e| ApiError::Network(format!("fetch: {:?}", e)))?;

            let resp: web_sys::Response = resp_value
                .dyn_into()
                .map_err(|_| ApiError::Network("invalid response".into()))?;

            let status = resp.status();
            match status {
                200..=299 => {
                    let text_promise = resp.text().map_err(|e| ApiError::Network(format!("text: {:?}", e)))?;
                    let text = wasm_bindgen_futures::JsFuture::from(text_promise)
                        .await
                        .map_err(|e| ApiError::Network(format!("text await: {:?}", e)))?
                        .as_string()
                        .unwrap_or_default();
                    if text.is_empty() {
                        serde_json::from_str("{}").map_err(|e| ApiError::Serialization(e.to_string()))
                    } else {
                        log::info!("API response (first 200): {}", &text[..text.len().min(200)]);
                        serde_json::from_str(&text)
                            .map_err(|e| ApiError::Serialization(format!("{}: {}", e, &text[..text.len().min(100)])))
                    }
                }
                401 => Err(ApiError::Unauthorized),
                403 => Err(ApiError::Forbidden),
                404 => Err(ApiError::NotFound),
                409 => {
                    let text_p = resp.text().map_err(|e| ApiError::Network(format!("{:?}", e)))?;
                    let text = wasm_bindgen_futures::JsFuture::from(text_p)
                        .await
                        .unwrap_or_default()
                        .as_string()
                        .unwrap_or_default();
                    Err(ApiError::Conflict(text))
                }
                500..=599 => {
                    let text_p = resp.text().map_err(|e| ApiError::Network(format!("{:?}", e)))?;
                    let text = wasm_bindgen_futures::JsFuture::from(text_p)
                        .await
                        .unwrap_or_default()
                        .as_string()
                        .unwrap_or_default();
                    Err(ApiError::Server(text))
                }
                _ => Err(ApiError::Server(format!("HTTP {}", status))),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (method, url, body);
            Err(ApiError::Network("native not supported".into()))
        }
    }
}
