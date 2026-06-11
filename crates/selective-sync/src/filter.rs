use crate::profile::{RuleDirection, SyncRule};

#[derive(Debug, Clone)]
pub struct PathFilter {
    include_patterns: Vec<glob::Pattern>,
    exclude_patterns: Vec<glob::Pattern>,
}

impl PathFilter {
    pub fn from_rules(rules: &[SyncRule]) -> Result<Self, glob::PatternError> {
        let mut include_patterns = Vec::new();
        let mut exclude_patterns = Vec::new();

        for rule in rules {
            let pattern = glob::Pattern::new(&rule.pattern)?;
            match rule.direction {
                RuleDirection::Include => include_patterns.push(pattern),
                RuleDirection::Exclude => exclude_patterns.push(pattern),
            }
        }

        Ok(Self {
            include_patterns,
            exclude_patterns,
        })
    }

    pub fn matches(&self, path: &str) -> bool {
        if self.exclude_patterns.iter().any(|p| p.matches(path)) {
            return false;
        }

        if self.include_patterns.is_empty() {
            return true;
        }

        self.include_patterns.iter().any(|p| p.matches(path))
    }

    pub fn filter_paths<'a>(&self, paths: &'a [String]) -> (Vec<&'a String>, Vec<&'a String>) {
        let mut matched = Vec::new();
        let mut missed = Vec::new();

        for path in paths {
            if self.matches(path) {
                matched.push(path);
            } else {
                missed.push(path);
            }
        }

        (matched, missed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(pattern: &str, direction: RuleDirection) -> SyncRule {
        SyncRule {
            pattern: pattern.to_string(),
            direction,
        }
    }

    #[test]
    fn test_include_only() {
        let rules = vec![rule("*.txt", RuleDirection::Include)];
        let filter = PathFilter::from_rules(&rules).unwrap();
        assert!(filter.matches("readme.txt"));
        assert!(!filter.matches("image.png"));
    }

    #[test]
    fn test_exclude_only() {
        let rules = vec![rule("*.log", RuleDirection::Exclude)];
        let filter = PathFilter::from_rules(&rules).unwrap();
        assert!(!filter.matches("debug.log"));
        assert!(filter.matches("readme.txt"));
    }

    #[test]
    fn test_include_and_exclude() {
        let rules = vec![
            rule("docs/**", RuleDirection::Include),
            rule("docs/draft/**", RuleDirection::Exclude),
        ];
        let filter = PathFilter::from_rules(&rules).unwrap();
        assert!(filter.matches("docs/guide.md"));
        assert!(!filter.matches("docs/draft/notes.md"));
        assert!(!filter.matches("src/main.rs"));
    }

    #[test]
    fn test_empty_rules_match_all() {
        let filter = PathFilter::from_rules(&[]).unwrap();
        assert!(filter.matches("anything.txt"));
    }

    #[test]
    fn test_filter_paths_batch() {
        let rules = vec![rule("*.rs", RuleDirection::Include)];
        let filter = PathFilter::from_rules(&rules).unwrap();
        let paths = vec![
            "main.rs".to_string(),
            "lib.rs".to_string(),
            "readme.md".to_string(),
        ];
        let (matched, missed) = filter.filter_paths(&paths);
        assert_eq!(matched.len(), 2);
        assert_eq!(missed.len(), 1);
    }
}
