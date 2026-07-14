use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct RansomwareConfig {
    pub max_mutations: u32,
    pub window_secs: u64,
    pub action: String,
    pub enabled: bool,
}

impl Default for RansomwareConfig {
    fn default() -> Self {
        Self {
            max_mutations: 100,
            window_secs: 60,
            action: "alert".to_string(),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone)]
struct MutationEvent {
    timestamp: Instant,
    path: String,
    #[allow(dead_code)]
    size: u64,
}

#[derive(Debug, Clone)]
struct UserMutationTracker {
    events: Vec<MutationEvent>,
    alerted: bool,
    last_alert: Option<Instant>,
}

impl UserMutationTracker {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            alerted: false,
            last_alert: None,
        }
    }

    fn record(&mut self, path: String, size: u64, window: Duration) -> u32 {
        let now = Instant::now();
        self.events.push(MutationEvent {
            timestamp: now,
            path,
            size,
        });

        let cutoff = now.checked_sub(window).unwrap();
        self.events.retain(|e| e.timestamp > cutoff);
        self.events.len() as u32
    }

    fn should_alert(&mut self, cooldown: Duration) -> bool {
        self.last_alert.is_none_or(|last| last.elapsed() >= cooldown)
    }
}

pub struct RansomwareDetector {
    config: RansomwareConfig,
    trackers: Arc<RwLock<HashMap<String, UserMutationTracker>>>,
    alerts: Arc<RwLock<Vec<RansomwareAlert>>>,
    alert_cooldown: Duration,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RansomwareAlert {
    pub user_id: String,
    pub mutation_count: u32,
    pub affected_paths: Vec<String>,
    pub timestamp: String,
    pub severity: String,
}

impl RansomwareDetector {
    #[must_use]
    pub fn new(config: RansomwareConfig) -> Self {
        Self {
            config,
            trackers: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            alert_cooldown: Duration::from_mins(5),
        }
    }

    pub async fn record_mutation(&self, user_id: &str, path: &str, size: u64) -> RansomwareCheckResult {
        if !self.config.enabled {
            return RansomwareCheckResult::Safe;
        }

        let window = Duration::from_secs(self.config.window_secs);
        let mut trackers = self.trackers.write().await;
        let tracker = trackers
            .entry(user_id.to_string())
            .or_insert_with(UserMutationTracker::new);
        let count = tracker.record(path.to_string(), size, window);

        if count > self.config.max_mutations {
            if !tracker.should_alert(self.alert_cooldown) {
                return RansomwareCheckResult::Safe;
            }

            tracker.alerted = true;
            tracker.last_alert = Some(Instant::now());

            let affected_paths: Vec<String> = tracker.events.iter().map(|e| e.path.clone()).collect();

            let alert = RansomwareAlert {
                user_id: user_id.to_string(),
                mutation_count: count,
                affected_paths: affected_paths.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                severity: if count > self.config.max_mutations * 3 {
                    "critical".to_string()
                } else {
                    "high".to_string()
                },
            };

            tracing::warn!(
                user_id = %user_id,
                mutation_count = count,
                threshold = self.config.max_mutations,
                severity = %alert.severity,
                affected_files = affected_paths.len(),
                "RANSOMWARE ALERT: mutation rate exceeded threshold"
            );

            let mut alerts = self.alerts.write().await;
            alerts.push(alert);
            while alerts.len() > 100 {
                alerts.remove(0);
            }

            match self.config.action.as_str() {
                "quarantine" => RansomwareCheckResult::Quarantine,
                "block" => RansomwareCheckResult::Blocked,
                _ => RansomwareCheckResult::Alerted,
            }
        } else {
            RansomwareCheckResult::Safe
        }
    }

    pub async fn alerts(&self) -> Vec<RansomwareAlert> {
        self.alerts.read().await.iter().cloned().collect()
    }

    pub async fn mutation_counts(&self) -> HashMap<String, u32> {
        let trackers = self.trackers.read().await;
        let window = Duration::from_secs(self.config.window_secs);
        let mut counts = HashMap::new();
        for (user_id, tracker) in trackers.iter() {
            let now = Instant::now();
            let cutoff = now.checked_sub(window).unwrap();
            let active_count = tracker.events.iter().filter(|e| e.timestamp > cutoff).count() as u32;
            if active_count > 0 {
                counts.insert(user_id.clone(), active_count);
            }
        }
        counts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RansomwareCheckResult {
    Safe,
    Alerted,
    Quarantine,
    Blocked,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_safe_mutations() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = RansomwareConfig {
                max_mutations: 10,
                window_secs: 60,
                action: "alert".to_string(),
                enabled: true,
            };
            let detector = RansomwareDetector::new(config);

            for i in 0..5 {
                let result = detector
                    .record_mutation("user1", &format!("/docs/file{}.txt", i), 1024)
                    .await;
                assert_eq!(result, RansomwareCheckResult::Safe);
            }
        });
    }

    #[test]
    fn test_detector_alert_threshold() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = RansomwareConfig {
                max_mutations: 5,
                window_secs: 60,
                action: "alert".to_string(),
                enabled: true,
            };
            let detector = RansomwareDetector::new(config);

            for i in 0..5 {
                let _ = detector.record_mutation("user1", &format!("/file{}.txt", i), 100).await;
            }

            let result = detector.record_mutation("user1", "/file5.txt", 100).await;
            assert_eq!(result, RansomwareCheckResult::Alerted);

            let alerts = detector.alerts().await;
            assert_eq!(alerts.len(), 1);
            assert_eq!(alerts[0].mutation_count, 6);
            assert_eq!(alerts[0].user_id, "user1");
        });
    }

    #[test]
    fn test_detector_disabled() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = RansomwareConfig {
                max_mutations: 1,
                window_secs: 60,
                action: "alert".to_string(),
                enabled: false,
            };
            let detector = RansomwareDetector::new(config);

            let result = detector.record_mutation("user1", "/file.txt", 100).await;
            assert_eq!(result, RansomwareCheckResult::Safe);

            let result = detector.record_mutation("user1", "/file2.txt", 100).await;
            assert_eq!(result, RansomwareCheckResult::Safe);
        });
    }

    #[test]
    fn test_detector_per_user_isolation() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = RansomwareConfig {
                max_mutations: 3,
                window_secs: 60,
                action: "alert".to_string(),
                enabled: true,
            };
            let detector = RansomwareDetector::new(config);

            for i in 0..4 {
                let _ = detector
                    .record_mutation("user1", &format!("/u1/file{}.txt", i), 100)
                    .await;
            }

            for i in 0..2 {
                let result = detector
                    .record_mutation("user2", &format!("/u2/file{}.txt", i), 100)
                    .await;
                assert_eq!(result, RansomwareCheckResult::Safe);
            }
        });
    }

    #[test]
    fn test_mutation_tracker_window() {
        let mut tracker = UserMutationTracker::new();
        let window = Duration::from_secs(10);

        for i in 0..5 {
            let count = tracker.record(format!("/file{}.txt", i), 100, window);
            assert_eq!(count, i + 1);
        }

        assert_eq!(tracker.events.len(), 5);
    }
}
