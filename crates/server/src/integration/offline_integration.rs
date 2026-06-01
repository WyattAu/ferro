//! Offline mode integration.
//!
//! Provides helpers for managing offline operations and reconciliation.

use ferro_offline::monitor::{ConnectionMonitor, ConnectionState};
use std::time::Duration;

pub fn create_connection_monitor() -> ConnectionMonitor {
    ConnectionMonitor::new()
}

pub fn check_online_state(monitor: &ConnectionMonitor) -> ConnectionState {
    monitor.state()
}

pub fn simulate_connectivity_check(monitor: &ConnectionMonitor, success: bool) -> Duration {
    monitor.record_check(success)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_monitor() {
        let monitor = create_connection_monitor();
        assert!(monitor.is_online());
        assert_eq!(check_online_state(&monitor), ConnectionState::Online);
    }

    #[test]
    fn test_offline_transition() {
        let monitor = create_connection_monitor();
        monitor.set_offline();
        assert!(!monitor.is_online());
        assert_eq!(check_online_state(&monitor), ConnectionState::Offline);
    }

    #[test]
    fn test_connectivity_check_failure_backoff() {
        let monitor =
            ConnectionMonitor::with_backoff(Duration::from_secs(1), Duration::from_secs(60));
        let backoff = simulate_connectivity_check(&monitor, false);
        assert!(backoff > Duration::ZERO);
        assert_eq!(monitor.consecutive_failures(), 1);
    }
}
