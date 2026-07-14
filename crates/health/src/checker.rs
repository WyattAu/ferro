use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use crate::HealthError;
use crate::probe::{HealthProbe, HealthStatus, ProbeType, TimedProbe};
use crate::response::HealthResponse;

#[derive(Debug)]
pub struct GlobalStatus {
    pub status: HealthStatus,
    pub message: String,
}

pub struct HealthChecker {
    probes: DashMap<String, TimedProbe>,
    version: String,
    start_time: Instant,
    global_status: tokio::sync::RwLock<Option<GlobalStatus>>,
    check_counter: AtomicU64,
}

impl HealthChecker {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            probes: DashMap::new(),
            version: version.into(),
            start_time: Instant::now(),
            global_status: tokio::sync::RwLock::new(None),
            check_counter: AtomicU64::new(0),
        }
    }

    pub fn register(&self, probe: Box<dyn HealthProbe>) -> Result<(), HealthError> {
        let name = probe.name().to_string();
        if self.probes.contains_key(&name) {
            return Err(HealthError::ProbeAlreadyRegistered { name });
        }
        self.probes.insert(name, TimedProbe::new(probe));
        Ok(())
    }

    pub fn unregister(&self, name: &str) -> Result<(), HealthError> {
        if self.probes.remove(name).is_none() {
            return Err(HealthError::ProbeNotFound { name: name.to_string() });
        }
        Ok(())
    }

    pub async fn check_liveness(&self) -> HealthResponse {
        self.check_by_type(ProbeType::Liveness).await
    }

    pub async fn check_readiness(&self) -> HealthResponse {
        self.check_by_type(ProbeType::Readiness).await
    }

    pub async fn check_startup(&self) -> HealthResponse {
        self.check_by_type(ProbeType::Startup).await
    }

    pub async fn check_all(&self) -> HealthResponse {
        let mut results = Vec::new();
        for entry in self.probes.iter() {
            let result = entry.value().check().await;
            results.push(result);
        }
        self.check_counter.fetch_add(1, Ordering::Relaxed);
        let uptime = self.start_time.elapsed();
        let mut response = HealthResponse::new(self.version.clone(), uptime).with_checks(results);
        if let Some(ref global) = *self.global_status.read().await {
            response = response.override_status(global.status, &global.message);
        }
        response
    }

    pub async fn set_global_status(&self, status: HealthStatus, message: &str) {
        let mut guard = self.global_status.write().await;
        *guard = Some(GlobalStatus {
            status,
            message: message.to_string(),
        });
    }

    pub fn component_status(&self, name: &str) -> Option<HealthStatus> {
        self.probes.get(name).map(|_entry| HealthStatus::Unknown)
    }

    pub fn probe_count(&self) -> usize {
        self.probes.len()
    }

    async fn check_by_type(&self, probe_type: ProbeType) -> HealthResponse {
        let mut results = Vec::new();
        for entry in self.probes.iter() {
            if entry.value().probe_type() == probe_type {
                let result = entry.value().check().await;
                results.push(result);
            }
        }
        self.check_counter.fetch_add(1, Ordering::Relaxed);
        let uptime = self.start_time.elapsed();
        let mut response = HealthResponse::new(self.version.clone(), uptime).with_checks(results);
        if let Some(ref global) = *self.global_status.read().await {
            response = response.override_status(global.status, &global.message);
        }
        response
    }
}
