use serde::{Deserialize, Serialize};

pub const FERRO_ABI_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginAbiManifest {
    pub abi_version: u32,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub entry_points: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PluginResult {
    Ok = 0,
    ErrorInvalidInput = 1,
    ErrorIo = 2,
    ErrorMemory = 3,
    ErrorPermission = 4,
    ErrorTimeout = 5,
    ErrorUnknown = 255,
}

impl PluginResult {
    pub fn from_u32(code: u32) -> Self {
        match code {
            0 => Self::Ok,
            1 => Self::ErrorInvalidInput,
            2 => Self::ErrorIo,
            3 => Self::ErrorMemory,
            4 => Self::ErrorPermission,
            5 => Self::ErrorTimeout,
            _ => Self::ErrorUnknown,
        }
    }
}

pub fn validate_abi_version(manifest: &PluginAbiManifest) -> Result<(), String> {
    if manifest.abi_version != FERRO_ABI_VERSION {
        return Err(format!(
            "ABI version mismatch: plugin requires v{}, host provides v{}",
            manifest.abi_version, FERRO_ABI_VERSION
        ));
    }
    if manifest.name.is_empty() {
        return Err("plugin name must not be empty".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_abi_version_ok() {
        let manifest = PluginAbiManifest {
            abi_version: FERRO_ABI_VERSION,
            name: "test-plugin".to_string(),
            version: "0.1.0".to_string(),
            entry_points: vec!["process".to_string()],
        };
        assert!(validate_abi_version(&manifest).is_ok());
    }

    #[test]
    fn test_validate_abi_version_mismatch() {
        let manifest = PluginAbiManifest {
            abi_version: 99,
            name: "test-plugin".to_string(),
            version: "0.1.0".to_string(),
            entry_points: vec![],
        };
        let err = validate_abi_version(&manifest).unwrap_err();
        assert!(err.contains("ABI version mismatch"));
        assert!(err.contains("v99"));
        assert!(err.contains(&format!("v{}", FERRO_ABI_VERSION)));
    }

    #[test]
    fn test_validate_abi_version_empty_name() {
        let manifest = PluginAbiManifest {
            abi_version: FERRO_ABI_VERSION,
            name: String::new(),
            version: "0.1.0".to_string(),
            entry_points: vec![],
        };
        let err = validate_abi_version(&manifest).unwrap_err();
        assert!(err.contains("plugin name must not be empty"));
    }

    #[test]
    fn test_plugin_result_from_u32() {
        assert_eq!(PluginResult::from_u32(0), PluginResult::Ok);
        assert_eq!(PluginResult::from_u32(1), PluginResult::ErrorInvalidInput);
        assert_eq!(PluginResult::from_u32(2), PluginResult::ErrorIo);
        assert_eq!(PluginResult::from_u32(3), PluginResult::ErrorMemory);
        assert_eq!(PluginResult::from_u32(4), PluginResult::ErrorPermission);
        assert_eq!(PluginResult::from_u32(5), PluginResult::ErrorTimeout);
        assert_eq!(PluginResult::from_u32(999), PluginResult::ErrorUnknown);
    }
}
