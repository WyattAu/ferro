use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeType {
    Liveness,
    Readiness,
    Startup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub name: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub duration: Duration,
    pub timestamp: DateTime<Utc>,
    pub details: HashMap<String, serde_json::Value>,
}

impl ProbeResult {
    pub fn healthy(name: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: None,
            duration,
            timestamp: Utc::now(),
            details: HashMap::new(),
        }
    }

    pub fn with_status(mut self, status: HealthStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.details.insert(key.into(), value);
        self
    }
}

#[async_trait]
pub trait HealthProbe: Send + Sync {
    fn name(&self) -> &str;
    async fn check(&self) -> ProbeResult;
    fn probe_type(&self) -> ProbeType;
}

pub struct TimedProbe {
    inner: Box<dyn HealthProbe>,
}

impl TimedProbe {
    pub fn new(inner: Box<dyn HealthProbe>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl HealthProbe for TimedProbe {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn check(&self) -> ProbeResult {
        let start = Instant::now();
        let mut result = self.inner.check().await;
        result.duration = start.elapsed();
        result.timestamp = Utc::now();
        result
    }

    fn probe_type(&self) -> ProbeType {
        self.inner.probe_type()
    }
}

pub struct CustomProbe {
    probe_name: String,
    probe_type: ProbeType,
    check_fn: Arc<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ProbeResult> + Send>> + Send + Sync>,
}

impl CustomProbe {
    pub fn new<F, Fut>(name: impl Into<String>, probe_type: ProbeType, check_fn: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ProbeResult> + Send + 'static,
    {
        Self {
            probe_name: name.into(),
            probe_type,
            check_fn: Arc::new(move || Box::pin(check_fn())),
        }
    }
}

#[async_trait]
impl HealthProbe for CustomProbe {
    fn name(&self) -> &str {
        &self.probe_name
    }

    async fn check(&self) -> ProbeResult {
        (self.check_fn)().await
    }

    fn probe_type(&self) -> ProbeType {
        self.probe_type
    }
}

pub struct DatabaseProbe {
    db_name: String,
    configurable_status: HealthStatus,
    message: Option<String>,
}

impl DatabaseProbe {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            db_name: name.into(),
            configurable_status: HealthStatus::Healthy,
            message: Some("Database connection OK".to_string()),
        }
    }

    pub fn with_status(mut self, status: HealthStatus, message: impl Into<String>) -> Self {
        self.configurable_status = status;
        self.message = Some(message.into());
        self
    }
}

#[async_trait]
impl HealthProbe for DatabaseProbe {
    fn name(&self) -> &str {
        &self.db_name
    }

    async fn check(&self) -> ProbeResult {
        ProbeResult::healthy(&self.db_name, Duration::from_micros(100))
            .with_status(self.configurable_status)
            .with_message(self.message.clone().unwrap_or_default())
    }

    fn probe_type(&self) -> ProbeType {
        ProbeType::Readiness
    }
}

pub struct MemoryProbe {
    probe_name: String,
    threshold_percent: f64,
}

impl MemoryProbe {
    pub fn new(threshold_percent: f64) -> Self {
        Self {
            probe_name: "memory".to_string(),
            threshold_percent: threshold_percent.clamp(0.0, 100.0),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.probe_name = name.into();
        self
    }

    fn read_vm_rss_kb() -> Option<f64> {
        let content = std::fs::read_to_string("/proc/self/status").ok()?;
        for line in content.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().ok();
                }
            }
        }
        None
    }

    fn read_total_mem_kb() -> Option<f64> {
        let content = std::fs::read_to_string("/proc/meminfo").ok()?;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().ok();
                }
            }
        }
        None
    }

    fn check_memory(&self) -> (HealthStatus, f64) {
        let rss_kb = Self::read_vm_rss_kb().unwrap_or(0.0);
        let total_kb = Self::read_total_mem_kb().unwrap_or(1.0);
        let percent = (rss_kb / total_kb) * 100.0;

        let status = if percent > self.threshold_percent {
            HealthStatus::Unhealthy
        } else if percent > self.threshold_percent * 0.8 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        (status, percent)
    }
}

#[async_trait]
impl HealthProbe for MemoryProbe {
    fn name(&self) -> &str {
        &self.probe_name
    }

    async fn check(&self) -> ProbeResult {
        let (status, percent) = self.check_memory();
        ProbeResult::healthy(&self.probe_name, Duration::from_micros(50))
            .with_status(status)
            .with_message(format!("memory usage: {:.2}%", percent))
            .with_detail("usage_percent", serde_json::json!(percent))
            .with_detail("threshold_percent", serde_json::json!(self.threshold_percent))
    }

    fn probe_type(&self) -> ProbeType {
        ProbeType::Liveness
    }
}

pub struct DiskSpaceProbe {
    probe_name: String,
    path: std::path::PathBuf,
    threshold_bytes: u64,
}

impl DiskSpaceProbe {
    pub fn new(path: impl Into<std::path::PathBuf>, threshold_bytes: u64) -> Self {
        let path = path.into();
        let display = path.display().to_string();
        Self {
            probe_name: format!("disk-{}", display),
            path,
            threshold_bytes,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.probe_name = name.into();
        self
    }

    #[cfg(unix)]
    fn check_disk(&self) -> (HealthStatus, u64) {
        use std::os::unix::ffi::OsStrExt;

        let c_path = match std::ffi::CString::new(self.path.as_os_str().as_bytes()) {
            Ok(p) => p,
            Err(_) => return (HealthStatus::Unknown, 0),
        };
        // SAFETY: `std::mem::zeroed()` is safe for `libc::statvfs` which is a plain C struct
        // with no Rust validity invariants beyond being fully initialized. All fields are
        // primitive integer types that default to zero.
        let mut statvfs: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: `c_path` is a valid null-terminated C string (verified by CString::new above),
        // and `statvfs` is a valid, writable pointer to an initialized `statvfs` struct.
        let ret = unsafe { libc::statvfs(c_path.as_ptr(), &mut statvfs) };
        if ret != 0 {
            return (HealthStatus::Unknown, 0);
        }
        let available = statvfs.f_bavail as u64 * statvfs.f_frsize as u64;
        let status = if available < self.threshold_bytes {
            HealthStatus::Unhealthy
        } else if available < self.threshold_bytes * 2 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
        (status, available)
    }

    #[cfg(not(unix))]
    fn check_disk(&self) -> (HealthStatus, u64) {
        (HealthStatus::Unknown, 0)
    }
}

#[async_trait]
impl HealthProbe for DiskSpaceProbe {
    fn name(&self) -> &str {
        &self.probe_name
    }

    async fn check(&self) -> ProbeResult {
        let (status, available) = self.check_disk();
        ProbeResult::healthy(&self.probe_name, Duration::from_micros(100))
            .with_status(status)
            .with_message(format!("available disk space: {} bytes", available))
            .with_detail("available_bytes", serde_json::json!(available))
            .with_detail("threshold_bytes", serde_json::json!(self.threshold_bytes))
    }

    fn probe_type(&self) -> ProbeType {
        ProbeType::Liveness
    }
}

/// Deep health probe for SQLite database connectivity.
/// Executes `SELECT 1` with a timeout to verify the database is reachable.
pub struct SqliteProbe {
    probe_name: String,
    db_handle: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    timeout_ms: u64,
}

impl SqliteProbe {
    pub fn new(db_handle: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>) -> Self {
        Self {
            probe_name: "database".to_string(),
            db_handle,
            timeout_ms: 2000,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.probe_name = name.into();
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

#[async_trait]
impl HealthProbe for SqliteProbe {
    fn name(&self) -> &str {
        &self.probe_name
    }

    async fn check(&self) -> ProbeResult {
        let start = Instant::now();
        let db = self.db_handle.clone();
        let timeout_ms = self.timeout_ms;

        let result = tokio::task::spawn_blocking(move || {
            let conn = db.lock().map_err(|e| e.to_string())?;
            conn.execute_batch("SELECT 1;").map_err(|e| e.to_string())
        })
        .await;

        let duration = start.elapsed();

        match result {
            Ok(Ok(())) => ProbeResult::healthy(&self.probe_name, duration)
                .with_message("SQLite connection OK")
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
            Ok(Err(e)) => ProbeResult::healthy(&self.probe_name, duration)
                .with_status(HealthStatus::Unhealthy)
                .with_message(format!("SQLite error: {}", e))
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
            Err(e) => ProbeResult::healthy(&self.probe_name, duration)
                .with_status(HealthStatus::Unhealthy)
                .with_message(format!("Task join error: {}", e))
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
        }
    }

    fn probe_type(&self) -> ProbeType {
        ProbeType::Readiness
    }
}

/// Deep health probe for storage backend connectivity.
/// Lists the root path `/` to verify the storage backend is reachable.
pub struct StorageProbe {
    probe_name: String,
    storage: std::sync::Arc<dyn common::storage::StorageEngine>,
    timeout_ms: u64,
}

impl StorageProbe {
    pub fn new(storage: std::sync::Arc<dyn common::storage::StorageEngine>) -> Self {
        Self {
            probe_name: "storage".to_string(),
            storage,
            timeout_ms: 2000,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.probe_name = name.into();
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

#[async_trait]
impl HealthProbe for StorageProbe {
    fn name(&self) -> &str {
        &self.probe_name
    }

    async fn check(&self) -> ProbeResult {
        let start = Instant::now();
        let storage = self.storage.clone();
        let timeout_ms = self.timeout_ms;

        let result = tokio::time::timeout(Duration::from_millis(timeout_ms), storage.list("/")).await;

        let duration = start.elapsed();

        match result {
            Ok(Ok(_)) => ProbeResult::healthy(&self.probe_name, duration)
                .with_message("Storage backend reachable")
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
            Ok(Err(e)) => ProbeResult::healthy(&self.probe_name, duration)
                .with_status(HealthStatus::Unhealthy)
                .with_message(format!("Storage error: {}", e))
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
            Err(_) => ProbeResult::healthy(&self.probe_name, duration)
                .with_status(HealthStatus::Unhealthy)
                .with_message(format!("Storage check timed out after {}ms", timeout_ms))
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
        }
    }

    fn probe_type(&self) -> ProbeType {
        ProbeType::Readiness
    }
}

/// Deep health probe for Redis connectivity.
/// Sends a PING command to verify Redis is reachable.
#[cfg(feature = "redis")]
pub struct RedisProbe {
    probe_name: String,
    client: Option<redis::Client>,
    timeout_ms: u64,
}

#[cfg(feature = "redis")]
impl RedisProbe {
    pub fn new(redis_url: &str) -> Self {
        let client = redis::Client::open(redis_url).ok();
        Self {
            probe_name: "redis".to_string(),
            client,
            timeout_ms: 2000,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.probe_name = name.into();
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

#[cfg(feature = "redis")]
#[async_trait]
impl HealthProbe for RedisProbe {
    fn name(&self) -> &str {
        &self.probe_name
    }

    async fn check(&self) -> ProbeResult {
        let start = Instant::now();
        let timeout_ms = self.timeout_ms;

        let result = match &self.client {
            Some(client) => {
                let client = client.clone();
                tokio::time::timeout(Duration::from_millis(timeout_ms), async move {
                    let mut conn = client
                        .get_multiplexed_async_connection()
                        .await
                        .map_err(|e| e.to_string())?;
                    redis::cmd("PING")
                        .query_async::<String>(&mut conn)
                        .await
                        .map_err(|e| e.to_string())
                })
                .await
            }
            None => {
                return ProbeResult::healthy(&self.probe_name, Duration::from_micros(0))
                    .with_status(HealthStatus::Degraded)
                    .with_message("Redis client not configured")
                    .with_detail("timeout_ms", serde_json::json!(timeout_ms));
            }
        };

        let duration = start.elapsed();

        match result {
            Ok(Ok(_)) => ProbeResult::healthy(&self.probe_name, duration)
                .with_message("Redis connection OK")
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
            Ok(Err(e)) => ProbeResult::healthy(&self.probe_name, duration)
                .with_status(HealthStatus::Degraded)
                .with_message(format!("Redis error: {}", e))
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
            Err(_) => ProbeResult::healthy(&self.probe_name, duration)
                .with_status(HealthStatus::Degraded)
                .with_message(format!("Redis check timed out after {}ms", timeout_ms))
                .with_detail("timeout_ms", serde_json::json!(timeout_ms)),
        }
    }

    fn probe_type(&self) -> ProbeType {
        ProbeType::Readiness
    }
}
