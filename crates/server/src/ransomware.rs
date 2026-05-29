//! Ransomware detection via file mutation rate monitoring (G-14).
//!
//! Monitors file operations per user and path prefix. When the mutation
//! rate exceeds a configurable threshold, an alert is generated and
//! optional defensive actions are taken.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for ransomware detection.
#[derive(Debug, Clone)]
pub struct RansomwareConfig {
    /// Maximum file mutations (writes/overwrites) per window.
    pub max_mutations: u32,
    /// Time window in seconds.
    pub window_secs: u64,
    /// Action when threshold exceeded: "alert" | "quarantine" | "block".
    pub action: String,
    /// Whether detection is enabled.
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

/// A single mutation event for rate tracking.
#[derive(Debug, Clone)]
struct MutationEvent {
    timestamp: Instant,
    path: String,
    size: u64,
}

/// Per-user mutation tracker.
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

    /// Record a mutation and return the count within the current window.
    fn record(&mut self, path: String, size: u64, window: Duration) -> u32 {
        let now = Instant::now();
        self.events.push(MutationEvent {
            timestamp: now,
            path,
            size,
        });

        // Prune old events outside the window
        let cutoff = now - window;
        self.events.retain(|e| e.timestamp > cutoff);
        self.events.len() as u32
    }

    /// Check and reset alert state.
    fn should_alert(&mut self, cooldown: Duration) -> bool {
        if let Some(last) = self.last_alert {
            if Instant::now() - last < cooldown {
                return false;
            }
        }
        true
    }
}

/// Ransomware detection engine.
pub struct RansomwareDetector {
    config: RansomwareConfig,
    /// Per-user mutation trackers.
    trackers: Arc<RwLock<HashMap<String, UserMutationTracker>>>,
    /// Alert callback (user_id, mutation_count, affected_paths).
    alerts: Arc<RwLock<Vec<RansomwareAlert>>>,
    /// Cooldown between alerts (5 minutes).
    alert_cooldown: Duration,
}

/// A ransomware alert event.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RansomwareAlert {
    pub user_id: String,
    pub mutation_count: u32,
    pub affected_paths: Vec<String>,
    pub timestamp: String,
    pub severity: String,
}

impl RansomwareDetector {
    /// Create a new detector with the given configuration.
    pub fn new(config: RansomwareConfig) -> Self {
        Self {
            config,
            trackers: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
            alert_cooldown: Duration::from_secs(300),
        }
    }

    /// Record a file mutation and check for ransomware patterns.
    /// Returns the mutation count for the user within the current window.
    pub async fn record_mutation(
        &self,
        user_id: &str,
        path: &str,
        size: u64,
    ) -> RansomwareCheckResult {
        if !self.config.enabled {
            return RansomwareCheckResult::Safe;
        }

        let window = Duration::from_secs(self.config.window_secs);
        let mut trackers = self.trackers.write().await;
        let tracker = trackers.entry(user_id.to_string()).or_insert_with(UserMutationTracker::new);
        let count = tracker.record(path.to_string(), size, window);

        if count > self.config.max_mutations {
            if !tracker.should_alert(self.alert_cooldown) {
                return RansomwareCheckResult::Safe;
            }

            // Alert threshold exceeded
            tracker.alerted = true;
            tracker.last_alert = Some(Instant::now());

            let affected_paths: Vec<String> = tracker
                .events
                .iter()
                .map(|e| e.path.clone())
                .collect();

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
            // Keep last 100 alerts
            while alerts.len() > 100 {
                alerts.remove(0);
            }

            // Take action based on configuration
            match self.config.action.as_str() {
                "quarantine" => RansomwareCheckResult::Quarantine,
                "block" => RansomwareCheckResult::Blocked,
                _ => RansomwareCheckResult::Alerted,
            }
        } else {
            RansomwareCheckResult::Safe
        }
    }

    /// Get recent alerts.
    pub async fn alerts(&self) -> Vec<RansomwareAlert> {
        self.alerts.read().await.iter().cloned().collect()
    }

    /// Get current mutation counts for all tracked users.
    pub async fn mutation_counts(&self) -> HashMap<String, u32> {
        let trackers = self.trackers.read().await;
        let window = Duration::from_secs(self.config.window_secs);
        let mut counts = HashMap::new();
        for (user_id, tracker) in trackers.iter() {
            let now = Instant::now();
            let cutoff = now - window;
            let active_count = tracker.events.iter().filter(|e| e.timestamp > cutoff).count() as u32;
            if active_count > 0 {
                counts.insert(user_id.clone(), active_count);
            }
        }
        counts
    }
}

/// Result of a ransomware check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RansomwareCheckResult {
    /// Mutation rate is normal.
    Safe,
    /// Mutation rate exceeded threshold (alert fired).
    Alerted,
    /// Mutation rate exceeded threshold (user quarantined).
    Quarantine,
    /// Mutation rate exceeded threshold (writes blocked).
    Blocked,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

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

            // 5 mutations should be safe
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

            // First 5 mutations are safe
            for i in 0..5 {
                let _ = detector
                    .record_mutation("user1", &format!("/file{}.txt", i), 100)
                    .await;
            }

            // 6th should trigger alert
            let result = detector
                .record_mutation("user1", "/file5.txt", 100)
                .await;
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

            // Even with many mutations, should be safe when disabled
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

            // User1 does 4 mutations (over threshold)
            for i in 0..4 {
                let _ = detector
                    .record_mutation("user1", &format!("/u1/file{}.txt", i), 100)
                    .await;
            }

            // User2 does 2 mutations (under threshold)
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

        // Record 5 mutations
        for i in 0..5 {
            let count = tracker.record(format!("/file{}.txt", i), 100, window);
            assert_eq!(count, i + 1);
        }

        // After simulated time passage, old events expire
        // (In a real test we'd use tokio::time, but the tracker uses Instant)
        assert_eq!(tracker.events.len(), 5);
    }
}
