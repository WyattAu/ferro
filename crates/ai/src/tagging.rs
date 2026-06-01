use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub confidence: f32,
    pub source: TagSource,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum TagSource {
    Auto { content_type: String },
    Manual { user_id: String },
    Rule { rule_id: String },
}

#[derive(Debug, Clone)]
pub struct TaggingConfig {
    pub min_confidence: f32,
    pub max_tags_per_file: usize,
    pub rules: Vec<TaggingRule>,
}

impl Default for TaggingConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            max_tags_per_file: 20,
            rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaggingRule {
    pub id: String,
    pub name: String,
    pub pattern: TagPattern,
    pub tag_name: String,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum TagPattern {
    Extension { extensions: Vec<String> },
    ContentType { types: Vec<String> },
    PathPrefix { prefix: String },
    SizeRange { min: Option<u64>, max: Option<u64> },
    Combined { rules: Vec<TaggingRule> },
}

impl TagPattern {
    fn matches(&self, path: &str, content_type: Option<&str>, size: Option<u64>) -> bool {
        match self {
            TagPattern::Extension { extensions } => {
                let ext = std::path::Path::new(path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase());
                ext.map(|e| extensions.iter().any(|x| x.to_lowercase() == e))
                    .unwrap_or(false)
            }
            TagPattern::ContentType { types } => content_type
                .map(|ct| {
                    types
                        .iter()
                        .any(|t| ct.eq_ignore_ascii_case(t) || ct.starts_with(&format!("{t};")))
                })
                .unwrap_or(false),
            TagPattern::PathPrefix { prefix } => path.starts_with(prefix),
            TagPattern::SizeRange { min, max } => size
                .map(|s| {
                    if let Some(min_val) = min
                        && s < *min_val
                    {
                        return false;
                    }
                    if let Some(max_val) = max
                        && s > *max_val
                    {
                        return false;
                    }
                    true
                })
                .unwrap_or(false),
            TagPattern::Combined { rules } => rules
                .iter()
                .any(|r| r.pattern.matches(path, content_type, size)),
        }
    }
}

pub struct AutoTagger {
    config: TaggingConfig,
}

impl AutoTagger {
    pub fn new(config: TaggingConfig) -> Self {
        Self { config }
    }

    pub fn suggest_tags(
        &self,
        path: &str,
        content_type: Option<&str>,
        size: Option<u64>,
    ) -> Vec<Tag> {
        let mut tags: Vec<Tag> = Vec::new();
        for rule in &self.config.rules {
            if rule.confidence < self.config.min_confidence {
                continue;
            }
            if rule.pattern.matches(path, content_type, size) {
                tags.push(Tag {
                    name: rule.tag_name.clone(),
                    confidence: rule.confidence,
                    source: TagSource::Rule {
                        rule_id: rule.id.clone(),
                    },
                    created_at: Utc::now(),
                });
            }
        }
        tags.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        tags.truncate(self.config.max_tags_per_file);
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_extension_rule(tag: &str, exts: Vec<&str>, conf: f32) -> TaggingRule {
        TaggingRule {
            id: "r1".to_string(),
            name: "ext rule".to_string(),
            pattern: TagPattern::Extension {
                extensions: exts.into_iter().map(String::from).collect(),
            },
            tag_name: tag.to_string(),
            confidence: conf,
        }
    }

    #[test]
    fn test_extension_rule_matches() {
        let config = TaggingConfig {
            rules: vec![make_extension_rule("image", vec!["jpg", "png"], 0.9)],
            ..Default::default()
        };
        let tagger = AutoTagger::new(config);
        let tags = tagger.suggest_tags("/photos/cat.jpg", None, None);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "image");
    }

    #[test]
    fn test_content_type_rule() {
        let config = TaggingConfig {
            rules: vec![TaggingRule {
                id: "r1".to_string(),
                name: "ct rule".to_string(),
                pattern: TagPattern::ContentType {
                    types: vec!["application/pdf".to_string()],
                },
                tag_name: "document".to_string(),
                confidence: 0.85,
            }],
            ..Default::default()
        };
        let tagger = AutoTagger::new(config);
        let tags = tagger.suggest_tags("/files/report.pdf", Some("application/pdf"), None);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "document");
    }

    #[test]
    fn test_path_prefix_rule() {
        let config = TaggingConfig {
            rules: vec![TaggingRule {
                id: "r1".to_string(),
                name: "prefix rule".to_string(),
                pattern: TagPattern::PathPrefix {
                    prefix: "/uploads/".to_string(),
                },
                tag_name: "uploaded".to_string(),
                confidence: 0.8,
            }],
            ..Default::default()
        };
        let tagger = AutoTagger::new(config);
        let tags = tagger.suggest_tags("/uploads/photo.jpg", None, None);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "uploaded");
    }

    #[test]
    fn test_size_range_rule() {
        let config = TaggingConfig {
            rules: vec![TaggingRule {
                id: "r1".to_string(),
                name: "size rule".to_string(),
                pattern: TagPattern::SizeRange {
                    min: Some(1024),
                    max: Some(10_485_760),
                },
                tag_name: "medium-file".to_string(),
                confidence: 0.75,
            }],
            ..Default::default()
        };
        let tagger = AutoTagger::new(config);
        let tags = tagger.suggest_tags("/data/file.bin", None, Some(5000));
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "medium-file");
    }

    #[test]
    fn test_combined_rule() {
        let config = TaggingConfig {
            rules: vec![TaggingRule {
                id: "r1".to_string(),
                name: "combined".to_string(),
                pattern: TagPattern::Combined {
                    rules: vec![
                        make_extension_rule("image", vec!["png"], 0.8),
                        TaggingRule {
                            id: "r1b".to_string(),
                            name: "pdf".to_string(),
                            pattern: TagPattern::Extension {
                                extensions: vec!["pdf".to_string()],
                            },
                            tag_name: "document".to_string(),
                            confidence: 0.7,
                        },
                    ],
                },
                tag_name: "media".to_string(),
                confidence: 0.9,
            }],
            ..Default::default()
        };
        let tagger = AutoTagger::new(config);
        let tags = tagger.suggest_tags("/pics/photo.png", None, None);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "media");
    }

    #[test]
    fn test_confidence_threshold_filtering() {
        let config = TaggingConfig {
            min_confidence: 0.8,
            rules: vec![
                make_extension_rule("high", vec!["rs"], 0.9),
                make_extension_rule("low", vec!["rs"], 0.6),
            ],
            ..Default::default()
        };
        let tagger = AutoTagger::new(config);
        let tags = tagger.suggest_tags("/src/main.rs", None, None);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "high");
    }

    #[test]
    fn test_no_matches_returns_empty() {
        let config = TaggingConfig {
            rules: vec![make_extension_rule("image", vec!["jpg"], 0.9)],
            ..Default::default()
        };
        let tagger = AutoTagger::new(config);
        let tags = tagger.suggest_tags("/docs/readme.md", None, None);
        assert!(tags.is_empty());
    }

    #[test]
    fn test_multiple_rules() {
        let config = TaggingConfig {
            rules: vec![
                make_extension_rule("rust-code", vec!["rs"], 0.95),
                TaggingRule {
                    id: "r2".to_string(),
                    name: "src prefix".to_string(),
                    pattern: TagPattern::PathPrefix {
                        prefix: "/src/".to_string(),
                    },
                    tag_name: "source".to_string(),
                    confidence: 0.8,
                },
            ],
            ..Default::default()
        };
        let tagger = AutoTagger::new(config);
        let tags = tagger.suggest_tags("/src/main.rs", None, None);
        assert_eq!(tags.len(), 2);
    }
}
