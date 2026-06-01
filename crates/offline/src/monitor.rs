//! Connection state monitoring with online/offline detection.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// Connection state of the offline store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConnectionState {
    /// Connected to the remote server.
    Online,
    /// Disconnected from the remote server.
    Offline,
}

/// Monitors connectivity state with configurable polling and exponential backoff.
pub struct ConnectionMonitor {
    is_online: Arc<AtomicBool>,
    notify: Arc<Notify>,
    /// Timestamp of the last successful connectivity check.
    last_online: Arc<std::sync::RwLock<Option<Instant>>>,
    /// Timestamp of the last failed connectivity check.
    last_offline: Arc<std::sync::RwLock<Option<Instant>>>,
    /// Number of consecutive failed checks (for backoff).
    consecutive_failures: Arc<std::sync::atomic::AtomicU32>,
    /// Maximum backoff interval between connectivity checks.
    max_backoff: Duration,
    /// Base backoff interval (doubles on each failure).
    base_backoff: Duration,
}

impl ConnectionMonitor {
    /// Create a new monitor, initially online.
    pub fn new() -> Self {
        Self {
            is_online: Arc::new(AtomicBool::new(true)),
            notify: Arc::new(Notify::new()),
            last_online: Arc::new(std::sync::RwLock::new(Some(Instant::now()))),
            last_offline: Arc::new(std::sync::RwLock::new(None)),
            consecutive_failures: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            max_backoff: Duration::from_secs(300),
            base_backoff: Duration::from_secs(5),
        }
    }

    /// Create a new monitor with custom backoff settings.
    pub fn with_backoff(base_backoff: Duration, max_backoff: Duration) -> Self {
        Self {
            max_backoff,
            base_backoff,
            ..Self::new()
        }
    }

    /// Check if currently online.
    pub fn is_online(&self) -> bool {
        self.is_online.load(Ordering::Relaxed)
    }

    /// Get the current connection state.
    pub fn state(&self) -> ConnectionState {
        if self.is_online() {
            ConnectionState::Online
        } else {
            ConnectionState::Offline
        }
    }

    /// Transition to online state.
    pub fn set_online(&self) {
        let was_offline = !self.is_online();
        self.is_online.store(true, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);
        if was_offline {
            *self.last_online.write().unwrap() = Some(Instant::now());
            self.notify.notify_waiters();
        }
    }

    /// Transition to offline state.
    pub fn set_offline(&self) {
        let was_online = self.is_online();
        self.is_online.store(false, Ordering::Relaxed);
        if was_online {
            *self.last_offline.write().unwrap() = Some(Instant::now());
            self.notify.notify_waiters();
        }
    }

    /// Simulate a connectivity check result.
    ///
    /// `success = true` → online, `success = false` → offline with backoff.
    /// Returns the recommended delay before the next check.
    pub fn record_check(&self, success: bool) -> Duration {
        if success {
            self.set_online();
            Duration::from_secs(0)
        } else {
            self.set_offline();
            let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
            let backoff_secs = (self.base_backoff.as_secs() * 2u64.pow(failures.min(8)))
                .min(self.max_backoff.as_secs());
            Duration::from_secs(backoff_secs)
        }
    }

    /// Get the current backoff interval based on consecutive failures.
    pub fn current_backoff(&self) -> Duration {
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        let backoff_secs = (self.base_backoff.as_secs() * 2u64.pow(failures.min(8)))
            .min(self.max_backoff.as_secs());
        Duration::from_secs(backoff_secs)
    }

    /// Wait for a state transition (online→offline or offline→online).
    /// Returns the new state.
    pub async fn wait_for_change(&self) -> ConnectionState {
        self.notify.notified().await;
        self.state()
    }

    /// Get the timestamp of the last online transition.
    pub fn last_online_at(&self) -> Option<Instant> {
        *self.last_online.read().unwrap()
    }

    /// Get the timestamp of the last offline transition.
    pub fn last_offline_at(&self) -> Option<Instant> {
        *self.last_offline.read().unwrap()
    }

    /// Get the number of consecutive failed connectivity checks.
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::Relaxed)
    }
}

impl Default for ConnectionMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_online() {
        let monitor = ConnectionMonitor::new();
        assert!(monitor.is_online());
        assert_eq!(monitor.state(), ConnectionState::Online);
    }

    #[test]
    fn test_set_offline() {
        let monitor = ConnectionMonitor::new();
        monitor.set_offline();
        assert!(!monitor.is_online());
        assert_eq!(monitor.state(), ConnectionState::Offline);
        assert!(monitor.last_offline_at().is_some());
    }

    #[test]
    fn test_set_online_after_offline() {
        let monitor = ConnectionMonitor::new();
        monitor.set_offline();
        monitor.set_online();
        assert!(monitor.is_online());
        assert!(monitor.last_online_at().is_some());
        assert_eq!(monitor.consecutive_failures(), 0);
    }

    #[test]
    fn test_set_online_when_already_online() {
        let monitor = ConnectionMonitor::new();
        let before = monitor.last_online_at();
        monitor.set_online();
        // Should not update timestamp (no transition)
        assert_eq!(monitor.last_online_at(), before);
    }

    #[test]
    fn test_record_check_success() {
        let monitor = ConnectionMonitor::new();
        monitor.set_offline();
        monitor.record_check(true);
        assert!(monitor.is_online());
        assert_eq!(monitor.consecutive_failures(), 0);
    }

    #[test]
    fn test_record_check_failure_increments_backoff() {
        let monitor =
            ConnectionMonitor::with_backoff(Duration::from_secs(1), Duration::from_secs(60));
        let b0 = monitor.current_backoff();
        assert_eq!(b0, Duration::from_secs(1));

        monitor.record_check(false);
        let b1 = monitor.current_backoff();
        assert_eq!(b1, Duration::from_secs(2)); // 1 * 2^1

        monitor.record_check(false);
        let b2 = monitor.current_backoff();
        assert_eq!(b2, Duration::from_secs(4)); // 1 * 2^2

        monitor.record_check(false);
        let b3 = monitor.current_backoff();
        assert_eq!(b3, Duration::from_secs(8)); // 1 * 2^3

        // Success resets
        monitor.record_check(true);
        assert_eq!(monitor.current_backoff(), Duration::from_secs(1));
    }

    #[test]
    fn test_backoff_caps_at_max() {
        let monitor =
            ConnectionMonitor::with_backoff(Duration::from_secs(10), Duration::from_secs(30));

        for _ in 0..10 {
            monitor.record_check(false);
        }
        // 10 * 2^10 = 10240, capped at 30
        assert!(monitor.current_backoff() <= Duration::from_secs(30));
    }

    #[test]
    fn test_backoff_from_failure() {
        let monitor =
            ConnectionMonitor::with_backoff(Duration::from_secs(1), Duration::from_secs(300));
        monitor.record_check(false);
        let backoff = monitor.record_check(false);
        assert_eq!(backoff, Duration::from_secs(4));
    }

    #[test]
    fn test_consecutive_failures_count() {
        let monitor = ConnectionMonitor::new();
        assert_eq!(monitor.consecutive_failures(), 0);
        monitor.record_check(false);
        assert_eq!(monitor.consecutive_failures(), 1);
        monitor.record_check(false);
        assert_eq!(monitor.consecutive_failures(), 2);
        monitor.record_check(true);
        assert_eq!(monitor.consecutive_failures(), 0);
    }
}
