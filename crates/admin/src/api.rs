use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConnectionConfig {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Clone, Default)]
pub struct ApiState {
    pub config: Option<AdminConnectionConfig>,
}

impl ApiState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(&mut self, url: String, token: String) {
        self.config = Some(AdminConnectionConfig { url, token });
    }

    pub fn disconnect(&mut self) {
        self.config = None;
    }

    pub fn is_connected(&self) -> bool {
        self.config.is_some()
    }

    fn base_url(&self) -> Result<String, String> {
        let config = self.config.as_ref().ok_or("Not connected to server")?;
        Ok(config.url.trim_end_matches('/').to_string())
    }

    fn auth_header(&self) -> Result<String, String> {
        let config = self.config.as_ref().ok_or("Not connected to server")?;
        Ok(format!("Bearer {}", config.token))
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let base = self.base_url()?;
        let auth = self.auth_header()?;
        let url = format!("{}{}", base, path);
        let resp = gloo_net::http::Request::get(&url)
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            return Err(format!("HTTP error: {}", status));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &impl Serialize,
    ) -> Result<T, String> {
        let base = self.base_url()?;
        let auth = self.auth_header()?;
        let url = format!("{}{}", base, path);
        let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::post(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .body(body_str)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            let status = resp.status();
            return Err(format!("HTTP error: {}", status));
        }
        resp.json().await.map_err(|e| e.to_string())
    }

    pub async fn delete(&self, path: &str) -> Result<(), String> {
        let base = self.base_url()?;
        let auth = self.auth_header()?;
        let url = format!("{}{}", base, path);
        let resp = gloo_net::http::Request::delete(&url)
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            return Err(format!("HTTP error: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn put(&self, path: &str, body: &impl Serialize) -> Result<(), String> {
        let base = self.base_url()?;
        let auth = self.auth_header()?;
        let url = format!("{}{}", base, path);
        let body_str = serde_json::to_string(body).map_err(|e| e.to_string())?;
        let resp = gloo_net::http::Request::put(&url)
            .header("Authorization", &auth)
            .header("Content-Type", "application/json")
            .body(body_str)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            return Err(format!("HTTP error: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn get_text(&self, path: &str) -> Result<String, String> {
        let base = self.base_url()?;
        let auth = self.auth_header()?;
        let url = format!("{}{}", base, path);
        let resp = gloo_net::http::Request::get(&url)
            .header("Authorization", &auth)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.ok() {
            return Err(format!("HTTP error: {}", resp.status()));
        }
        resp.text().await.map_err(|e| e.to_string())
    }

    pub async fn server_stats(&self) -> Result<serde_json::Value, String> {
        self.get("/api/v1/admin/stats").await
    }

    pub async fn storage_info(&self) -> Result<serde_json::Value, String> {
        self.get("/api/v1/admin/storage").await
    }

    pub async fn list_users(&self) -> Result<Vec<serde_json::Value>, String> {
        self.get("/api/v1/admin/users").await
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        role: &str,
    ) -> Result<serde_json::Value, String> {
        #[derive(Serialize)]
        struct CreateUser<'a> {
            username: &'a str,
            password: &'a str,
            role: &'a str,
        }
        self.post(
            "/api/v1/admin/users",
            &CreateUser {
                username,
                password,
                role,
            },
        )
        .await
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<(), String> {
        self.delete(&format!("/api/v1/admin/users/{}", user_id))
            .await
    }

    pub async fn list_webhooks(&self) -> Result<Vec<serde_json::Value>, String> {
        self.get("/api/v1/admin/webhooks").await
    }

    pub async fn create_webhook(
        &self,
        url: &str,
        events: Vec<String>,
        secret: &str,
    ) -> Result<serde_json::Value, String> {
        #[derive(Serialize)]
        struct CreateWebhook<'a> {
            url: &'a str,
            events: Vec<String>,
            secret: &'a str,
        }
        self.post(
            "/api/v1/admin/webhooks",
            &CreateWebhook {
                url,
                events,
                secret,
            },
        )
        .await
    }

    pub async fn delete_webhook(&self, id: &str) -> Result<(), String> {
        self.delete(&format!("/api/v1/admin/webhooks/{}", id)).await
    }

    pub async fn audit_log(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<serde_json::Value, String> {
        self.get(&format!(
            "/api/v1/admin/audit?limit={}&offset={}",
            limit, offset
        ))
        .await
    }

    pub async fn prometheus_metrics(&self) -> Result<String, String> {
        self.get_text("/metrics/prometheus").await
    }

    pub async fn server_health(&self) -> Result<serde_json::Value, String> {
        self.get("/.well-known/ferro").await
    }

    pub async fn federation_followers(
        &self,
        username: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.get(&format!("/fed/actor/{}/followers", username))
            .await
    }

    pub async fn federation_following(
        &self,
        username: &str,
    ) -> Result<Vec<serde_json::Value>, String> {
        self.get(&format!("/fed/actor/{}/following", username))
            .await
    }

    pub async fn federation_inbox(&self) -> Result<Vec<serde_json::Value>, String> {
        self.get("/fed/inbox").await
    }

    pub async fn federation_outbox(&self) -> Result<Vec<serde_json::Value>, String> {
        self.get("/fed/outbox").await
    }

    pub async fn federation_nodeinfo(&self) -> Result<serde_json::Value, String> {
        self.get("/fed/nodeinfo").await
    }

    pub async fn test_connection(&self) -> Result<serde_json::Value, String> {
        self.server_health().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_state_new() {
        let state = ApiState::new();
        assert!(!state.is_connected());
        assert!(state.config.is_none());
    }

    #[test]
    fn test_api_state_connect() {
        let mut state = ApiState::new();
        state.connect("http://localhost:8080".to_string(), "secret".to_string());
        assert!(state.is_connected());
        let config = state.config.as_ref().unwrap();
        assert_eq!(config.url, "http://localhost:8080");
        assert_eq!(config.token, "secret");
    }

    #[test]
    fn test_api_state_disconnect() {
        let mut state = ApiState::new();
        state.connect("http://localhost:8080".to_string(), "token".to_string());
        assert!(state.is_connected());
        state.disconnect();
        assert!(!state.is_connected());
    }

    #[test]
    fn test_api_state_base_url_not_connected() {
        let state = ApiState::new();
        assert!(state.base_url().is_err());
    }

    #[test]
    fn test_api_state_base_url_trims_slash() {
        let mut state = ApiState::new();
        state.connect("http://localhost:8080/".to_string(), "t".to_string());
        assert_eq!(state.base_url().unwrap(), "http://localhost:8080");
    }

    #[test]
    fn test_api_state_base_url_no_trailing_slash() {
        let mut state = ApiState::new();
        state.connect("http://localhost:8080".to_string(), "t".to_string());
        assert_eq!(state.base_url().unwrap(), "http://localhost:8080");
    }

    #[test]
    fn test_api_state_auth_header_not_connected() {
        let state = ApiState::new();
        assert!(state.auth_header().is_err());
    }

    #[test]
    fn test_api_state_auth_header_connected() {
        let mut state = ApiState::new();
        state.connect("http://localhost:8080".to_string(), "mytoken".to_string());
        assert_eq!(state.auth_header().unwrap(), "Bearer mytoken");
    }

    #[test]
    fn test_api_state_default() {
        let state = ApiState::default();
        assert!(!state.is_connected());
    }

    #[test]
    fn test_admin_connection_config_serde() {
        let config = AdminConnectionConfig {
            url: "http://localhost:8080".to_string(),
            token: "secret123".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AdminConnectionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, "http://localhost:8080");
        assert_eq!(parsed.token, "secret123");
    }
}
