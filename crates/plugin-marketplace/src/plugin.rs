use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::MarketplaceError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(String);

impl Default for PluginId {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_str_unchecked(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for PluginId {
    type Err = MarketplaceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        uuid::Uuid::parse_str(s)
            .map(|_| Self(s.to_string()))
            .map_err(|_| MarketplaceError::PluginNotFound { id: s.to_string() })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for Version {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(());
        }
        Ok(Version {
            major: parts[0].parse().map_err(|_| ())?,
            minor: parts[1].parse().map_err(|_| ())?,
            patch: parts[2].parse().map_err(|_| ())?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: PluginId,
    pub name: String,
    pub description: String,
    pub version: Version,
    pub author: String,
    pub license: String,
    pub abi_version: String,
    pub homepage: Option<Url>,
    pub repository: Option<Url>,
    pub icon_url: Option<Url>,
    pub capabilities: Vec<String>,
    pub tags: Vec<String>,
    pub checksum: [u8; 32],
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub downloads: u64,
    pub rating: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub metadata: PluginMetadata,
    pub wasm_url: Url,
    pub dependencies: Vec<PluginDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: PluginId,
    pub min_version: Version,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInstallation {
    pub plugin_id: PluginId,
    pub version: Version,
    pub installed_at: DateTime<Utc>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginReview {
    pub user_id: String,
    pub rating: u8,
    pub comment: String,
    pub created_at: DateTime<Utc>,
}

impl PluginReview {
    pub fn new(
        user_id: impl Into<String>,
        rating: u8,
        comment: impl Into<String>,
    ) -> Result<Self, String> {
        if !(1..=5).contains(&rating) {
            return Err("rating must be between 1 and 5".to_string());
        }
        Ok(Self {
            user_id: user_id.into(),
            rating,
            comment: comment.into(),
            created_at: Utc::now(),
        })
    }
}
