use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::RoutingError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendId {
    Local,
    S3,
    Gcs,
    AzureBlob,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub pattern: String,
    pub backend_id: BackendId,
    pub priority: u32,
    #[serde(default)]
    pub metadata_filter: HashMap<String, String>,
    #[serde(default)]
    pub read_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub name: String,
    pub rules: Vec<RoutingRule>,
    pub default_backend: BackendId,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingDecision {
    pub backend_id: BackendId,
    pub matched_rule: Option<String>,
    pub policy_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingWarning {
    OverlappingRules {
        path_pattern: String,
        rule_ids: Vec<String>,
    },
    UnreachableDefault {
        default_backend: String,
    },
    UnusedBackend {
        backend_id: String,
    },
    CycleDetected {
        policy_chain: Vec<String>,
    },
}

impl RoutingPolicy {
    pub fn new(name: impl Into<String>, default_backend: BackendId) -> Self {
        Self {
            name: name.into(),
            rules: Vec::new(),
            default_backend,
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn add_rule(mut self, rule: RoutingRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn route(&self, path: &str, metadata: &HashMap<String, String>) -> RoutingDecision {
        let mut sorted: Vec<&RoutingRule> = self.rules.iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.priority));

        for rule in sorted {
            if !glob_match(&rule.pattern, path) {
                continue;
            }
            if !metadata_filter_matches(&rule.metadata_filter, metadata) {
                continue;
            }
            return RoutingDecision {
                backend_id: rule.backend_id.clone(),
                matched_rule: Some(rule.pattern.clone()),
                policy_name: self.name.clone(),
            };
        }

        RoutingDecision {
            backend_id: self.default_backend.clone(),
            matched_rule: None,
            policy_name: self.name.clone(),
        }
    }
}

pub struct BackendRouter {
    policies: Vec<RoutingPolicy>,
}

impl BackendRouter {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    pub fn add_policy(&mut self, policy: RoutingPolicy) -> Result<(), RoutingError> {
        if self.policies.iter().any(|p| p.name == policy.name) {
            return Err(RoutingError::PolicyAlreadyExists(policy.name));
        }
        self.policies.push(policy);
        Ok(())
    }

    pub fn remove_policy(&mut self, name: &str) -> Result<RoutingPolicy, RoutingError> {
        let idx = self
            .policies
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| RoutingError::PolicyNotFound(name.to_string()))?;
        Ok(self.policies.remove(idx))
    }

    pub fn route(
        &self,
        path: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<RoutingDecision, RoutingError> {
        if self.policies.is_empty() {
            return Err(RoutingError::NoDecision(path.to_string()));
        }

        for policy in &self.policies {
            let decision = policy.route(path, metadata);
            if decision.matched_rule.is_some() {
                return Ok(decision);
            }
        }

        let first = &self.policies[0];
        Ok(first.route(path, metadata))
    }

    pub fn list_backends(&self) -> HashSet<BackendId> {
        let mut backends = HashSet::new();
        for policy in &self.policies {
            backends.insert(policy.default_backend.clone());
            for rule in &policy.rules {
                backends.insert(rule.backend_id.clone());
            }
        }
        backends
    }

    pub fn validate(&self) -> Vec<RoutingWarning> {
        let mut warnings = Vec::new();

        let all_backends = self.list_backends();

        for policy in &self.policies {
            let mut rule_pairs: Vec<(&RoutingRule, &RoutingRule)> = Vec::new();
            for i in 0..policy.rules.len() {
                for j in (i + 1)..policy.rules.len() {
                    rule_pairs.push((&policy.rules[i], &policy.rules[j]));
                }
            }

            let mut overlap_groups: HashMap<String, Vec<String>> = HashMap::new();
            for (a, b) in &rule_pairs {
                if rules_overlap(a, b) {
                    overlap_groups
                        .entry(format!("{} / {}", a.pattern, b.pattern))
                        .or_default()
                        .push(a.pattern.clone());
                    overlap_groups
                        .entry(format!("{} / {}", a.pattern, b.pattern))
                        .or_default()
                        .push(b.pattern.clone());
                }
            }

            for ids in overlap_groups.values() {
                let unique: Vec<String> = ids.iter().cloned().collect::<HashSet<_>>().into_iter().collect();
                warnings.push(RoutingWarning::OverlappingRules {
                    path_pattern: unique.join(", "),
                    rule_ids: unique,
                });
            }

            for rule in &policy.rules {
                if rule.pattern == "**" || rule.pattern == "*" {
                    warnings.push(RoutingWarning::UnreachableDefault {
                        default_backend: format!("{:?}", policy.default_backend),
                    });
                }
            }
        }

        let routed_backends = self.compute_routed_backends();
        for backend in &all_backends {
            if !routed_backends.contains(backend) {
                warnings.push(RoutingWarning::UnusedBackend {
                    backend_id: format!("{backend:?}"),
                });
            }
        }

        warnings
    }

    fn compute_routed_backends(&self) -> HashSet<BackendId> {
        let mut routed = HashSet::new();
        let test_paths = [
            "file.txt",
            "public/file.txt",
            "internal/doc.pdf",
            "archive/2024/report.csv",
            "deep/nested/path/to/file.bin",
            "a/b/c/d/e/f.txt",
            "root",
            "",
        ];
        let empty_meta = HashMap::new();
        for path in &test_paths {
            if let Ok(decision) = self.route(path, &empty_meta) {
                routed.insert(decision.backend_id);
            }
        }
        routed
    }
}

impl Default for BackendRouter {
    fn default() -> Self {
        Self::new()
    }
}

fn metadata_filter_matches(filter: &HashMap<String, String>, metadata: &HashMap<String, String>) -> bool {
    if filter.is_empty() {
        return true;
    }
    for (key, value) in filter {
        if metadata.get(key) != Some(value) {
            return false;
        }
    }
    true
}

fn rules_overlap(a: &RoutingRule, b: &RoutingRule) -> bool {
    if a.pattern == b.pattern {
        return true;
    }
    let test_paths = ["public/a/b", "public/x", "internal/c", "x/y", ""];
    let empty_meta = HashMap::new();
    let mut both_match = false;

    for path in &test_paths {
        let a_match =
            glob_match(&a.pattern, path) && metadata_filter_matches(&a.metadata_filter, &empty_meta);
        let b_match =
            glob_match(&b.pattern, path) && metadata_filter_matches(&b.metadata_filter, &empty_meta);
        if a_match && b_match {
            both_match = true;
            break;
        }
    }
    both_match
}

pub fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.trim_start_matches('/');
    let text = text.trim_start_matches('/');

    if pattern == "**" || pattern == "*" {
        return true;
    }

    let pat_parts: Vec<&str> = pattern.split('/').collect();
    let txt_parts: Vec<&str> = text.split('/').filter(|s| !s.is_empty()).collect();

    match_glob_segments(&pat_parts, &txt_parts)
}

fn match_glob_segments(pat: &[&str], text: &[&str]) -> bool {
    if pat.is_empty() && text.is_empty() {
        return true;
    }
    if pat.is_empty() {
        return false;
    }

    let segment = pat[0];
    let rest_pat = &pat[1..];

    if segment == "**" {
        if rest_pat.is_empty() {
            return true;
        }
        for i in 0..=text.len() {
            if match_glob_segments(rest_pat, &text[i..]) {
                return true;
            }
        }
        return false;
    }

    if text.is_empty() {
        return false;
    }

    if segment_matches(segment, text[0]) && match_glob_segments(rest_pat, &text[1..]) {
        return true;
    }

    false
}

fn segment_matches(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.contains('*') {
        return segment_glob_match(pattern, text);
    }
    pattern == text
}

fn segment_glob_match(pattern: &str, text: &str) -> bool {
    let p_bytes = pattern.as_bytes();
    let t_bytes = text.as_bytes();
    let mut pi = 0;
    let mut ti = 0;
    let mut star_pi = usize::MAX;
    let mut star_ti = 0;

    while ti < t_bytes.len() {
        if pi < p_bytes.len() && p_bytes[pi] == b'*' {
            star_pi = pi;
            star_ti = ti;
            pi += 1;
        } else if pi < p_bytes.len() && (p_bytes[pi] == t_bytes[ti]) {
            pi += 1;
            ti += 1;
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < p_bytes.len() && p_bytes[pi] == b'*' {
        pi += 1;
    }

    pi == p_bytes.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("public/file.txt", "public/file.txt"));
    }

    #[test]
    fn test_glob_match_single_star() {
        assert!(glob_match("public/*.txt", "public/readme.txt"));
        assert!(!glob_match("public/*.txt", "public/sub/readme.txt"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("public/**", "public/a/b/c/d.txt"));
        assert!(glob_match("public/**", "public/file.txt"));
    }

    #[test]
    fn test_glob_match_root_patterns() {
        assert!(glob_match("**", "anything/deeply/nested"));
        assert!(glob_match("*", "single"));
    }

    #[test]
    fn test_simple_rule_matching() {
        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "public/**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: false,
        });

        let decision = policy.route("public/img/logo.png", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::S3);
        assert_eq!(decision.matched_rule, Some("public/**".to_string()));
    }

    #[test]
    fn test_default_backend_when_no_rules_match() {
        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "public/**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: false,
        });

        let decision = policy.route("internal/secret.txt", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::Local);
        assert!(decision.matched_rule.is_none());
    }

    #[test]
    fn test_priority_ordering() {
        let policy = RoutingPolicy::new("test", BackendId::Local)
            .add_rule(RoutingRule {
                pattern: "**".to_string(),
                backend_id: BackendId::Gcs,
                priority: 1,
                metadata_filter: HashMap::new(),
                read_fallback: false,
            })
            .add_rule(RoutingRule {
                pattern: "public/**".to_string(),
                backend_id: BackendId::S3,
                priority: 10,
                metadata_filter: HashMap::new(),
                read_fallback: false,
            });

        let decision = policy.route("public/file.txt", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::S3);

        let decision = policy.route("internal/file.txt", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::Gcs);
    }

    #[test]
    fn test_metadata_filter_matching() {
        let mut filter = HashMap::new();
        filter.insert("retention".to_string(), "long".to_string());

        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "archive/**".to_string(),
            backend_id: BackendId::AzureBlob,
            priority: 1,
            metadata_filter: filter,
            read_fallback: false,
        });

        let mut meta_match = HashMap::new();
        meta_match.insert("retention".to_string(), "long".to_string());
        let decision = policy.route("archive/2024/data.csv", &meta_match);
        assert_eq!(decision.backend_id, BackendId::AzureBlob);

        let mut meta_no_match = HashMap::new();
        meta_no_match.insert("retention".to_string(), "short".to_string());
        let decision = policy.route("archive/2024/data.csv", &meta_no_match);
        assert_eq!(decision.backend_id, BackendId::Local);
    }

    #[test]
    fn test_metadata_filter_missing_key() {
        let mut filter = HashMap::new();
        filter.insert("tier".to_string(), "hot".to_string());

        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: filter,
            read_fallback: false,
        });

        let decision = policy.route("anything.txt", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::Local);
    }

    #[test]
    fn test_read_fallback_flag() {
        let rule = RoutingRule {
            pattern: "public/**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: true,
        };
        assert!(rule.read_fallback);

        let rule_no_fallback = RoutingRule {
            read_fallback: false,
            ..rule
        };
        assert!(!rule_no_fallback.read_fallback);
    }

    #[test]
    fn test_multiple_policies_first_matching_wins() {
        let mut router = BackendRouter::new();

        router
            .add_policy(
                RoutingPolicy::new("s3-policy", BackendId::S3).add_rule(RoutingRule {
                    pattern: "uploads/**".to_string(),
                    backend_id: BackendId::S3,
                    priority: 5,
                    metadata_filter: HashMap::new(),
                    read_fallback: false,
                }),
            )
            .unwrap();

        router
            .add_policy(
                RoutingPolicy::new("catch-all", BackendId::Local).add_rule(RoutingRule {
                    pattern: "**".to_string(),
                    backend_id: BackendId::Local,
                    priority: 1,
                    metadata_filter: HashMap::new(),
                    read_fallback: false,
                }),
            )
            .unwrap();

        let decision = router.route("uploads/image.png", &HashMap::new()).unwrap();
        assert_eq!(decision.backend_id, BackendId::S3);
        assert_eq!(decision.policy_name, "s3-policy");

        let decision = router.route("other/file.txt", &HashMap::new()).unwrap();
        assert_eq!(decision.backend_id, BackendId::Local);
        assert_eq!(decision.policy_name, "catch-all");
    }

    #[test]
    fn test_policy_removal() {
        let mut router = BackendRouter::new();
        router
            .add_policy(
                RoutingPolicy::new("s3-policy", BackendId::S3).add_rule(RoutingRule {
                    pattern: "uploads/**".to_string(),
                    backend_id: BackendId::S3,
                    priority: 5,
                    metadata_filter: HashMap::new(),
                    read_fallback: false,
                }),
            )
            .unwrap();

        let removed = router.remove_policy("s3-policy").unwrap();
        assert_eq!(removed.name, "s3-policy");

        let result = router.remove_policy("s3-policy");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_backends() {
        let mut router = BackendRouter::new();
        router
            .add_policy(
                RoutingPolicy::new("multi", BackendId::Local)
                    .add_rule(RoutingRule {
                        pattern: "a/**".to_string(),
                        backend_id: BackendId::S3,
                        priority: 1,
                        metadata_filter: HashMap::new(),
                        read_fallback: false,
                    })
                    .add_rule(RoutingRule {
                        pattern: "b/**".to_string(),
                        backend_id: BackendId::Gcs,
                        priority: 1,
                        metadata_filter: HashMap::new(),
                        read_fallback: false,
                    }),
            )
            .unwrap();

        let backends = router.list_backends();
        assert!(backends.contains(&BackendId::Local));
        assert!(backends.contains(&BackendId::S3));
        assert!(backends.contains(&BackendId::Gcs));
    }

    #[test]
    fn test_validate_overlapping_rules() {
        let mut router = BackendRouter::new();
        router
            .add_policy(
                RoutingPolicy::new("overlap", BackendId::Local)
                    .add_rule(RoutingRule {
                        pattern: "public/**".to_string(),
                        backend_id: BackendId::S3,
                        priority: 5,
                        metadata_filter: HashMap::new(),
                        read_fallback: false,
                    })
                    .add_rule(RoutingRule {
                        pattern: "public/**".to_string(),
                        backend_id: BackendId::Gcs,
                        priority: 3,
                        metadata_filter: HashMap::new(),
                        read_fallback: false,
                    }),
            )
            .unwrap();

        let warnings = router.validate();
        assert!(warnings.iter().any(|w| matches!(w, RoutingWarning::OverlappingRules { .. })));
    }

    #[test]
    fn test_validate_unreachable_default() {
        let mut router = BackendRouter::new();
        router
            .add_policy(
                RoutingPolicy::new("catch-all", BackendId::Local).add_rule(RoutingRule {
                    pattern: "**".to_string(),
                    backend_id: BackendId::S3,
                    priority: 1,
                    metadata_filter: HashMap::new(),
                    read_fallback: false,
                }),
            )
            .unwrap();

        let warnings = router.validate();
        assert!(warnings
            .iter()
            .any(|w| matches!(w, RoutingWarning::UnreachableDefault { .. })));
    }

    #[test]
    fn test_validate_no_warnings_for_valid_policy() {
        let mut router = BackendRouter::new();
        router
            .add_policy(
                RoutingPolicy::new("clean", BackendId::Local).add_rule(RoutingRule {
                    pattern: "public/**".to_string(),
                    backend_id: BackendId::S3,
                    priority: 1,
                    metadata_filter: HashMap::new(),
                    read_fallback: false,
                }),
            )
            .unwrap();

        let warnings = router.validate();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_empty_path() {
        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: false,
        });

        let decision = policy.route("", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::S3);
    }

    #[test]
    fn test_root_path() {
        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "*.txt".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: false,
        });

        let decision = policy.route("file.txt", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::S3);
    }

    #[test]
    fn test_deeply_nested_paths() {
        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "a/b/c/**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: false,
        });

        let decision = policy.route("a/b/c/d/e/f/g/h.txt", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::S3);

        let decision = policy.route("a/b/shallow.txt", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::Local);
    }

    #[test]
    fn test_add_duplicate_policy_errors() {
        let mut router = BackendRouter::new();
        let policy = RoutingPolicy::new("dup", BackendId::Local);
        assert!(router.add_policy(policy.clone()).is_ok());
        assert!(router.add_policy(policy).is_err());
    }

    #[test]
    fn test_route_with_no_policies() {
        let router = BackendRouter::new();
        let result = router.route("file.txt", &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_backend_id() {
        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "special/**".to_string(),
            backend_id: BackendId::Custom("minio-prod".to_string()),
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: false,
        });

        let decision = policy.route("special/data.bin", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::Custom("minio-prod".to_string()));
    }

    #[test]
    fn test_backend_id_equality() {
        assert_eq!(BackendId::S3, BackendId::S3);
        assert_eq!(BackendId::Custom("a".to_string()), BackendId::Custom("a".to_string()));
        assert_ne!(BackendId::S3, BackendId::Gcs);
    }

    #[test]
    fn test_routing_decision_policy_name() {
        let policy = RoutingPolicy::new("my-policy", BackendId::Local).add_rule(RoutingRule {
            pattern: "x/**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: false,
        });

        let decision = policy.route("x/y", &HashMap::new());
        assert_eq!(decision.policy_name, "my-policy");
    }

    #[test]
    fn test_policy_description() {
        let policy = RoutingPolicy::new("test", BackendId::Local).with_description("A test policy");
        assert_eq!(policy.description.as_deref(), Some("A test policy"));
    }

    #[test]
    fn test_multiple_metadata_filters() {
        let mut filter = HashMap::new();
        filter.insert("env".to_string(), "prod".to_string());
        filter.insert("tier".to_string(), "hot".to_string());

        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "data/**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: filter,
            read_fallback: false,
        });

        let mut meta_full = HashMap::new();
        meta_full.insert("env".to_string(), "prod".to_string());
        meta_full.insert("tier".to_string(), "hot".to_string());
        let decision = policy.route("data/file.txt", &meta_full);
        assert_eq!(decision.backend_id, BackendId::S3);

        let mut meta_partial = HashMap::new();
        meta_partial.insert("env".to_string(), "prod".to_string());
        let decision = policy.route("data/file.txt", &meta_partial);
        assert_eq!(decision.backend_id, BackendId::Local);
    }

    #[test]
    fn test_star_in_middle_of_segment() {
        assert!(glob_match("log*.txt", "logfile.txt"));
        assert!(glob_match("log*.txt", "log.txt"));
        assert!(!glob_match("log*.txt", "logfile.csv"));
    }

    #[test]
    fn test_leading_slash_normalized() {
        let policy = RoutingPolicy::new("test", BackendId::Local).add_rule(RoutingRule {
            pattern: "public/**".to_string(),
            backend_id: BackendId::S3,
            priority: 1,
            metadata_filter: HashMap::new(),
            read_fallback: false,
        });

        let decision = policy.route("/public/file.txt", &HashMap::new());
        assert_eq!(decision.backend_id, BackendId::S3);
    }
}
