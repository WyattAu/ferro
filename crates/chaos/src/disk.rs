//! Disk fault injection

use tracing::{info, warn};

/// Disk fault types
#[derive(Debug, Clone)]
pub enum DiskFault {
    /// Simulate disk full
    DiskFull { path: String },
    /// Simulate I/O error
    IoError { path: String, probability: f64 },
    /// Simulate slow disk
    SlowDisk { path: String, delay_ms: u64 },
}

/// Disk fault injector
pub struct DiskFaultInjector {
    faults: Vec<DiskFault>,
}

impl DiskFaultInjector {
    pub fn new() -> Self {
        Self { faults: Vec::new() }
    }

    pub fn add_fault(&mut self, fault: DiskFault) {
        self.faults.push(fault);
    }

    pub async fn inject(&self) {
        for fault in &self.faults {
            match fault {
                DiskFault::DiskFull { path } => {
                    warn!("Simulating disk full at {}", path);
                }
                DiskFault::IoError { path, probability } => {
                    info!("Simulating I/O error at {} with {}%", path, probability * 100.0);
                }
                DiskFault::SlowDisk { path, delay_ms } => {
                    info!("Simulating slow disk at {} with {}ms delay", path, delay_ms);
                }
            }
        }
    }
}

impl Default for DiskFaultInjector {
    fn default() -> Self {
        Self::new()
    }
}
