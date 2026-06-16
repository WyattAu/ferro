//! Platform-specific implementations for iOS and Android.

#[cfg(target_os = "ios")]
pub mod ios {
    #[allow(dead_code)]
    pub fn platform_name() -> &'static str {
        "iOS"
    }

    #[allow(dead_code)]
    pub fn default_cache_path() -> &'static str {
        "/var/mobile/Library/Caches/com.wyattau.ferro.mobile"
    }
}

#[cfg(target_os = "android")]
pub mod android {
    #[allow(dead_code)]
    pub fn platform_name() -> &'static str {
        "Android"
    }

    #[allow(dead_code)]
    pub fn default_cache_path() -> &'static str {
        "/data/data/com.wyattau.ferro.mobile/cache"
    }
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub mod stub {
    #[allow(dead_code)]
    pub fn platform_name() -> &'static str {
        "unknown"
    }

    #[allow(dead_code)]
    pub fn default_cache_path() -> &'static str {
        "/tmp/ferro-mobile-cache"
    }
}
