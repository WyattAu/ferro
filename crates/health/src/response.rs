use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::probe::{HealthStatus, ProbeResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub version: String,
    pub uptime: Duration,
    pub timestamp: DateTime<Utc>,
    pub checks: Vec<ProbeResult>,
    pub info: HashMap<String, String>,
}

impl HealthResponse {
    pub fn new(version: String, uptime: Duration) -> Self {
        Self {
            status: HealthStatus::Healthy,
            version,
            uptime,
            timestamp: Utc::now(),
            checks: Vec::new(),
            info: HashMap::new(),
        }
    }

    pub fn with_checks(mut self, checks: Vec<ProbeResult>) -> Self {
        self.status = Self::aggregate_status(&checks);
        self.timestamp = Utc::now();
        self.checks = checks;
        self
    }

    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.info.insert(key.into(), value.into());
        self
    }

    pub fn override_status(mut self, status: HealthStatus, message: &str) -> Self {
        self.status = status;
        self.info
            .insert("override_message".to_string(), message.to_string());
        self
    }

    fn aggregate_status(checks: &[ProbeResult]) -> HealthStatus {
        if checks.is_empty() {
            return HealthStatus::Unknown;
        }
        let all_unknown = checks.iter().all(|c| c.status == HealthStatus::Unknown);
        if all_unknown {
            return HealthStatus::Unknown;
        }
        let all_unhealthy = checks.iter().all(|c| c.status == HealthStatus::Unhealthy);
        if all_unhealthy {
            return HealthStatus::Unhealthy;
        }
        let has_unhealthy = checks.iter().any(|c| c.status == HealthStatus::Unhealthy);
        let has_degraded = checks.iter().any(|c| c.status == HealthStatus::Degraded);
        if has_unhealthy || has_degraded {
            return HealthStatus::Degraded;
        }
        HealthStatus::Healthy
    }
}
