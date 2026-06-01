use crate::error::SyncFilterError;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncludeRule {
    pub pattern: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExcludeRule {
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProfile {
    pub name: String,
    pub includes: Vec<IncludeRule>,
    pub excludes: Vec<ExcludeRule>,
    pub max_file_size: Option<u64>,
    pub sync_interval: Duration,
    pub enabled: bool,
}

impl SyncProfile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            includes: Vec::new(),
            excludes: Vec::new(),
            max_file_size: None,
            sync_interval: Duration::from_secs(300),
            enabled: true,
        }
    }

    pub fn add_include(&mut self, pattern: &str) -> Result<(), SyncFilterError> {
        if let Err(e) = glob::Pattern::new(pattern) {
            return Err(SyncFilterError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: e.msg.to_string(),
            });
        }
        self.includes.push(IncludeRule {
            pattern: pattern.to_string(),
        });
        Ok(())
    }

    pub fn add_exclude(&mut self, pattern: &str) -> Result<(), SyncFilterError> {
        if let Err(e) = glob::Pattern::new(pattern) {
            return Err(SyncFilterError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: e.msg.to_string(),
            });
        }
        self.excludes.push(ExcludeRule {
            pattern: pattern.to_string(),
        });
        Ok(())
    }

    pub fn should_sync(&self, relative_path: &str) -> bool {
        if !self.enabled {
            return false;
        }
        if self.includes.is_empty() {
            return false;
        }
        let matches_include = self
            .includes
            .iter()
            .any(|rule| matches_pattern(&rule.pattern, relative_path));
        if !matches_include {
            return false;
        }
        let matches_exclude = self
            .excludes
            .iter()
            .any(|rule| matches_pattern(&rule.pattern, relative_path));
        !matches_exclude
    }

    pub fn should_sync_with_size(&self, relative_path: &str, file_size: u64) -> bool {
        if !self.should_sync(relative_path) {
            return false;
        }
        if let Some(max_size) = self.max_file_size
            && file_size > max_size
        {
            return false;
        }
        true
    }
}

fn matches_pattern(pattern: &str, path: &str) -> bool {
    glob::Pattern::new(pattern)
        .map(|p| p.matches(path))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_include_only() {
        let mut profile = SyncProfile::new("docs");
        profile.add_include("Documents/**").unwrap();
        assert!(profile.should_sync("Documents/report.pdf"));
        assert!(profile.should_sync("Documents/sub/notes.txt"));
        assert!(!profile.should_sync("Photos/img.png"));
    }

    #[test]
    fn test_exclude_overrides_include() {
        let mut profile = SyncProfile::new("docs");
        profile.add_include("Documents/**").unwrap();
        profile.add_exclude("**/*.tmp").unwrap();
        assert!(profile.should_sync("Documents/report.pdf"));
        assert!(!profile.should_sync("Documents/draft.tmp"));
        assert!(!profile.should_sync("Documents/sub/temp.tmp"));
    }

    #[test]
    fn test_nested_patterns() {
        let mut profile = SyncProfile::new("dev");
        profile.add_include("Projects/**/*.rs").unwrap();
        assert!(profile.should_sync("Projects/ferro/src/main.rs"));
        assert!(profile.should_sync("Projects/cli/src/bin.rs"));
        assert!(!profile.should_sync("Projects/ferro/Cargo.toml"));
        assert!(!profile.should_sync("Projects/other.py"));
    }

    #[test]
    fn test_max_file_size_filter() {
        let mut profile = SyncProfile::new("small");
        profile.add_include("**/*").unwrap();
        profile.max_file_size = Some(1024);
        assert!(profile.should_sync_with_size("file.txt", 512));
        assert!(profile.should_sync_with_size("file.txt", 1024));
        assert!(!profile.should_sync_with_size("big.bin", 2048));
    }

    #[test]
    fn test_disabled_profile() {
        let mut profile = SyncProfile::new("off");
        profile.add_include("**/*").unwrap();
        profile.enabled = false;
        assert!(!profile.should_sync("anything.txt"));
    }

    #[test]
    fn test_empty_profile_syncs_nothing() {
        let profile = SyncProfile::new("empty");
        assert!(!profile.should_sync("anything.txt"));
    }

    #[test]
    fn test_wildcard_patterns() {
        let mut profile = SyncProfile::new("wide");
        profile.add_include("**/*.pdf").unwrap();
        profile.add_include("**/*.txt").unwrap();
        profile.add_exclude("tmp/**").unwrap();
        assert!(profile.should_sync("docs/readme.txt"));
        assert!(profile.should_sync("docs/report.pdf"));
        assert!(!profile.should_sync("docs/data.csv"));
        assert!(!profile.should_sync("tmp/draft.txt"));
    }

    #[test]
    fn test_invalid_pattern() {
        let mut profile = SyncProfile::new("bad");
        let err = profile.add_include("[invalid").unwrap_err();
        assert!(matches!(err, SyncFilterError::InvalidPattern { .. }));
    }

    #[test]
    fn test_invalid_exclude_pattern() {
        let mut profile = SyncProfile::new("bad");
        let err = profile.add_exclude("**/*[!").unwrap_err();
        assert!(matches!(err, SyncFilterError::InvalidPattern { .. }));
    }
}
