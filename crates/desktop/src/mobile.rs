pub struct MobileConfig {
    pub app_id: &'static str,
    pub app_name: &'static str,
    pub bundle_id: &'static str,
}

impl MobileConfig {
    pub fn android() -> Self {
        Self {
            app_id: "com.ferro.app",
            app_name: "Ferro",
            bundle_id: "com.ferro.app",
        }
    }

    pub fn ios() -> Self {
        Self {
            app_id: "com.ferro.ios",
            app_name: "Ferro",
            bundle_id: "com.ferro.ios",
        }
    }

    pub fn identifier(&self) -> String {
        self.bundle_id.to_string()
    }

    pub fn user_agent(&self) -> String {
        format!("{}/{}", self.app_id, env!("CARGO_PKG_VERSION"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_config() {
        let config = MobileConfig::android();
        assert_eq!(config.app_id, "com.ferro.app");
        assert_eq!(config.app_name, "Ferro");
    }

    #[test]
    fn test_ios_config() {
        let config = MobileConfig::ios();
        assert_eq!(config.app_id, "com.ferro.ios");
    }

    #[test]
    fn test_user_agent() {
        let config = MobileConfig::android();
        let ua = config.user_agent();
        assert!(ua.contains("Ferro"));
    }
}
