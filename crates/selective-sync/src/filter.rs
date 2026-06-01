use crate::sync_profile::SyncProfile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncDecision {
    Sync,
    Skip { reason: String },
    Defer,
}

#[derive(Debug)]
pub struct PathFilter {
    profiles: Vec<SyncProfile>,
}

impl PathFilter {
    pub fn new() -> Self {
        Self {
            profiles: Vec::new(),
        }
    }

    pub fn add_profile(&mut self, profile: SyncProfile) -> &str {
        self.profiles.push(profile);
        self.profiles.last().unwrap().name.as_str()
    }

    pub fn check(&self, relative_path: &str, file_size: Option<u64>) -> SyncDecision {
        for profile in &self.profiles {
            if !profile.enabled {
                continue;
            }
            let size = file_size.unwrap_or(0);
            if profile.should_sync_with_size(relative_path, size) {
                return SyncDecision::Sync;
            }
        }
        SyncDecision::Skip {
            reason: "no matching profile".to_string(),
        }
    }

    pub fn list_syncable(&self, paths: &[(String, Option<u64>)]) -> Vec<(String, SyncDecision)> {
        paths
            .iter()
            .map(|(path, size)| {
                let decision = self.check(path, *size);
                (path.clone(), decision)
            })
            .collect()
    }
}

impl Default for PathFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync_profile::SyncProfile;

    #[test]
    fn test_basic_filtering() {
        let mut filter = PathFilter::new();
        let mut profile = SyncProfile::new("docs");
        profile.add_include("Documents/**").unwrap();
        filter.add_profile(profile);
        assert_eq!(filter.check("Documents/file.txt", None), SyncDecision::Sync);
        assert_eq!(
            filter.check("Photos/img.png", None),
            SyncDecision::Skip {
                reason: "no matching profile".to_string()
            }
        );
    }

    #[test]
    fn test_multiple_profiles() {
        let mut filter = PathFilter::new();
        let mut docs = SyncProfile::new("docs");
        docs.add_include("Documents/**").unwrap();
        let mut photos = SyncProfile::new("photos");
        photos.add_include("Photos/**").unwrap();
        filter.add_profile(docs);
        filter.add_profile(photos);
        assert_eq!(filter.check("Documents/file.txt", None), SyncDecision::Sync);
        assert_eq!(filter.check("Photos/img.png", None), SyncDecision::Sync);
        assert_eq!(
            filter.check("Music/song.mp3", None),
            SyncDecision::Skip {
                reason: "no matching profile".to_string()
            }
        );
    }

    #[test]
    fn test_priority_first_matching_wins() {
        let mut filter = PathFilter::new();
        let mut p1 = SyncProfile::new("restrictive");
        p1.add_include("Documents/**/*.txt").unwrap();
        let mut p2 = SyncProfile::new("broad");
        p2.add_include("Documents/**").unwrap();
        filter.add_profile(p1);
        filter.add_profile(p2);
        assert_eq!(filter.check("Documents/file.txt", None), SyncDecision::Sync);
        assert_eq!(
            filter.check("Documents/report.pdf", None),
            SyncDecision::Sync
        );
    }

    #[test]
    fn test_list_syncable() {
        let mut filter = PathFilter::new();
        let mut profile = SyncProfile::new("text");
        profile.add_include("**/*.txt").unwrap();
        filter.add_profile(profile);
        let paths = vec![
            ("readme.txt".to_string(), None),
            ("photo.jpg".to_string(), None),
            ("notes.txt".to_string(), Some(2048)),
        ];
        let results = filter.list_syncable(&paths);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].1, SyncDecision::Sync);
        assert!(matches!(results[1].1, SyncDecision::Skip { .. }));
        assert_eq!(results[2].1, SyncDecision::Sync);
    }

    #[test]
    fn test_disabled_profile_skipped() {
        let mut filter = PathFilter::new();
        let mut profile = SyncProfile::new("off");
        profile.add_include("**/*").unwrap();
        profile.enabled = false;
        filter.add_profile(profile);
        assert_eq!(
            filter.check("anything.txt", None),
            SyncDecision::Skip {
                reason: "no matching profile".to_string()
            }
        );
    }

    #[test]
    fn test_max_file_size_in_filter() {
        let mut filter = PathFilter::new();
        let mut profile = SyncProfile::new("small");
        profile.add_include("**/*").unwrap();
        profile.max_file_size = Some(100);
        filter.add_profile(profile);
        assert_eq!(filter.check("file.txt", Some(50)), SyncDecision::Sync);
        assert_eq!(
            filter.check("big.bin", Some(200)),
            SyncDecision::Skip {
                reason: "no matching profile".to_string()
            }
        );
        assert_eq!(filter.check("file.txt", None), SyncDecision::Sync);
    }
}
