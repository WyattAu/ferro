use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub user_agent: String,
    pub os: Option<String>,
    pub browser: Option<String>,
    pub device_name: Option<String>,
}

impl DeviceInfo {
    pub fn new(device_id: String, user_agent: String) -> Self {
        Self {
            device_id,
            user_agent,
            os: None,
            browser: None,
            device_name: None,
        }
    }

    pub fn with_os(mut self, os: String) -> Self {
        self.os = Some(os);
        self
    }

    pub fn with_browser(mut self, browser: String) -> Self {
        self.browser = Some(browser);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.device_name = Some(name);
        self
    }
}
